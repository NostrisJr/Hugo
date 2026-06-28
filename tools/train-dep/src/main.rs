//! Entraînement du **parser de dépendances** (arc-eager) de Hugo.
//!
//! Lit le CoNLL-U de **Universal Dependencies French-GSD** (colonnes HEAD et
//! DEPREL), entraîne un classifieur de transitions par **perceptron moyenné**
//! avec oracle statique et *teacher forcing*, évalue l'UAS/LAS sur le dev, puis
//! sérialise le modèle quantifié vers l'asset embarqué
//! `crates/hugo-core/assets/pos.dep` lu par le runtime.
//!
//! ```text
//! cargo run -p train-dep --release -- <train.conllu> <dev.conllu> <sortie.dep>
//! ```
//!
//! Le système de transitions, l'extraction de traits ([`dep::transition_features`])
//! et le sérialiseur ([`dep::serialize_model`]) sont **réutilisés du runtime** :
//! les traits vus à l'entraînement et au décodage sont rigoureusement identiques.
//! Les POS fournies au parser sont les POS **prédites** par le CRF embarqué
//! ([`pos::tag_forms`]), comme au runtime — et non les POS de référence du
//! treebank, pour éviter tout décalage entraînement/décodage.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use hugo_core::dep::{self, DepRel, Move, State, NMOVE};
use hugo_core::pos::{self, Upos};

/// Occurrences minimales d'un trait pour être retenu (élagage — pèse sur la
/// taille de l'asset embarqué).
const MIN_COUNT: u32 = 3;
/// Nombre d'époques de perceptron (le moyennage plafonne tôt).
const EPOCHS: usize = 8;

/// Une phrase annotée en dépendances.
struct Sent {
    forms: Vec<String>,
    /// Gouverneur de référence de chaque token : index `0..n`, ou `n` (= ROOT)
    /// pour la racine de la phrase.
    gold_head: Vec<usize>,
    /// Relation de référence de chaque token.
    gold_label: Vec<DepRel>,
}

// --- Lecture CoNLL-U ----------------------------------------------------------

/// Lit un fichier CoNLL-U en phrases (formes + HEAD + DEPREL). Ignore les
/// commentaires, les tokens multi-mots (`n-m`) et les nœuds vides (`n.m`). Les
/// identifiants de tokens entiers sont contigus (1..n) : l'index interne est
/// `id - 1`, et `HEAD == 0` (racine) devient `n` (ROOT virtuel).
fn read_conllu(path: &str) -> std::io::Result<Vec<Sent>> {
    let reader = BufReader::new(File::open(path)?);
    let mut sents = Vec::new();
    let mut forms: Vec<String> = Vec::new();
    let mut heads_raw: Vec<usize> = Vec::new();
    let mut labels: Vec<DepRel> = Vec::new();

    let flush = |forms: &mut Vec<String>,
                 heads_raw: &mut Vec<usize>,
                 labels: &mut Vec<DepRel>,
                 sents: &mut Vec<Sent>| {
        if forms.is_empty() {
            return;
        }
        let n = forms.len();
        let gold_head: Vec<usize> = heads_raw
            .iter()
            .map(|&h| if h == 0 { n } else { (h - 1).min(n) })
            .collect();
        sents.push(Sent {
            forms: std::mem::take(forms),
            gold_head,
            gold_label: std::mem::take(labels),
        });
        heads_raw.clear();
    };

    for line in reader.lines() {
        let line = line?;
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut forms, &mut heads_raw, &mut labels, &mut sents);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 8 {
            continue;
        }
        if cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        let head: usize = cols[6].parse().unwrap_or(0);
        forms.push(cols[1].to_string());
        heads_raw.push(head);
        labels.push(DepRel::from_conllu(cols[7]));
    }
    flush(&mut forms, &mut heads_raw, &mut labels, &mut sents);
    Ok(sents)
}

// --- Oracle statique arc-eager ------------------------------------------------

/// Transition de référence dans la configuration `st`, d'après l'arbre gold.
/// Oracle statique standard pour arbres projectifs.
fn oracle(st: &State, gh: &[usize], gl: &[DepRel]) -> Move {
    let n = st.n;
    let s0 = *st.stack.last().unwrap();
    let b0 = st.buffer; // garanti < n (appelé hors état final)

    // LEFT-ARC : s0 réel, non gouverné, et sa tête gold est b0.
    if s0 != n && !st.has_head(s0) && gh[s0] == b0 {
        return Move::LeftArc(gl[s0]);
    }
    // RIGHT-ARC : la tête gold de b0 est s0 (s0 peut être ROOT → racine).
    if gh[b0] == s0 {
        return Move::RightArc(gl[b0]);
    }
    // REDUCE : s0 gouverné et sans dépendant droit gold restant dans le tampon.
    if s0 != n && st.has_head(s0) {
        let has_right_dep = (b0..n).any(|k| gh[k] == s0);
        if !has_right_dep {
            return Move::Reduce;
        }
    }
    Move::Shift
}

/// POS prédites pour une phrase (mêmes que le runtime).
fn predict_pos(forms: &[String]) -> Vec<Upos> {
    let refs: Vec<&str> = forms.iter().map(String::as_str).collect();
    pos::tag_forms(&refs)
}

// --- Perceptron moyenné (averaging paresseux) ---------------------------------

struct Perceptron {
    m: usize,
    w: Vec<f64>,
    acc: Vec<f64>,
    ts: Vec<u64>,
    t: u64,
}

impl Perceptron {
    fn new(rows: usize, m: usize) -> Self {
        Perceptron {
            m,
            w: vec![0.0; rows * m],
            acc: vec![0.0; rows * m],
            ts: vec![0; rows * m],
            t: 1,
        }
    }

    #[inline]
    fn touch(&mut self, idx: usize) {
        self.acc[idx] += self.w[idx] * (self.t - self.ts[idx]) as f64;
        self.ts[idx] = self.t;
    }

    #[inline]
    fn add(&mut self, row: usize, class: usize, d: f64) {
        let idx = row * self.m + class;
        self.touch(idx);
        self.w[idx] += d;
    }

    /// Scores courants (non moyennés) des `m` classes pour les lignes actives.
    fn score(&self, rows: &[u32], out: &mut [f64]) {
        out.iter_mut().for_each(|x| *x = 0.0);
        for &r in rows {
            let base = r as usize * self.m;
            for c in 0..self.m {
                out[c] += self.w[base + c];
            }
        }
    }

    /// Fige et renvoie les poids **moyennés** (pour l'évaluation et l'asset).
    fn averaged(&mut self) -> Vec<f32> {
        for idx in 0..self.w.len() {
            self.touch(idx);
        }
        let t = self.t as f64;
        self.acc.iter().map(|&a| (a / t) as f32).collect()
    }
}

// --- Décodage glouton (évaluation, avec poids moyennés) -----------------------

fn fallback_priority(m: Move) -> i32 {
    match m {
        Move::RightArc(_) => 3,
        Move::Shift => 2,
        Move::LeftArc(_) => 1,
        Move::Reduce => 0,
    }
}

fn lookup(attrs: &[String], vocab: &HashMap<String, u32>) -> Vec<u32> {
    attrs.iter().filter_map(|a| vocab.get(a).copied()).collect()
}

fn decode(
    forms: &[String],
    upos: &[Upos],
    w: &[f32],
    vocab: &HashMap<String, u32>,
) -> Vec<(usize, DepRel)> {
    let n = forms.len();
    if n == 0 {
        return Vec::new();
    }
    let fr: Vec<&str> = forms.iter().map(String::as_str).collect();
    let mut st = State::new(&fr, upos);
    let mut attrs: Vec<String> = Vec::new();
    let mut scores = vec![0.0f32; NMOVE];
    let max_steps = 2 * n + 2;
    let mut steps = 0;

    while !st.is_final() && steps < max_steps {
        steps += 1;
        dep::transition_features(&st, &mut attrs);
        let rows = lookup(&attrs, vocab);
        scores.iter_mut().for_each(|x| *x = 0.0);
        for &r in &rows {
            let base = r as usize * NMOVE;
            for (c, s) in scores.iter_mut().enumerate() {
                *s += w[base + c];
            }
        }
        let mut best: Option<(Move, f32, i32)> = None;
        for c in 0..NMOVE {
            let mv = Move::from_class(c);
            if !st.is_legal(mv) {
                continue;
            }
            let sc = scores[c];
            let pr = fallback_priority(mv);
            let better = match best {
                None => true,
                Some((_, bs, bp)) => sc > bs || (sc == bs && pr > bp),
            };
            if better {
                best = Some((mv, sc, pr));
            }
        }
        match best {
            Some((mv, _, _)) => st.apply(mv),
            None => {
                if st.is_legal(Move::Shift) {
                    st.apply(Move::Shift);
                } else {
                    break;
                }
            }
        }
    }
    (0..n)
        .map(|i| {
            if st.heads[i] == usize::MAX {
                (n, DepRel::Root)
            } else {
                (st.heads[i], st.deps[i])
            }
        })
        .collect()
}

/// UAS (attachement non étiqueté) et LAS (étiqueté) sur un jeu, ponctuation
/// exclue (convention CoNLL).
fn evaluate(sents: &[Sent], w: &[f32], vocab: &HashMap<String, u32>) -> (f64, f64) {
    let mut uas = 0usize;
    let mut las = 0usize;
    let mut total = 0usize;
    for s in sents {
        let upos = predict_pos(&s.forms);
        let pred = decode(&s.forms, &upos, w, vocab);
        for i in 0..s.forms.len() {
            if upos[i] == Upos::Punct {
                continue;
            }
            total += 1;
            if pred[i].0 == s.gold_head[i] {
                uas += 1;
                if pred[i].1 == s.gold_label[i] {
                    las += 1;
                }
            }
        }
    }
    if total == 0 {
        (0.0, 0.0)
    } else {
        (uas as f64 / total as f64, las as f64 / total as f64)
    }
}

// --- Pilote -------------------------------------------------------------------

/// Rejoue l'oracle sur une phrase, en appelant `visit(features, gold_move)` à
/// chaque configuration. Renvoie `false` si l'oracle bloque (phrase ignorée).
fn rollout<F: FnMut(&[String], Move)>(s: &Sent, upos: &[Upos], mut visit: F) {
    let fr: Vec<&str> = s.forms.iter().map(String::as_str).collect();
    let mut st = State::new(&fr, upos);
    let mut attrs: Vec<String> = Vec::new();
    let max_steps = 2 * s.forms.len() + 2;
    let mut steps = 0;
    while !st.is_final() && steps < max_steps {
        steps += 1;
        let gold = oracle(&st, &s.gold_head, &s.gold_label);
        dep::transition_features(&st, &mut attrs);
        visit(&attrs, gold);
        st.apply(gold);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage : train-dep <train.conllu> <dev.conllu> <sortie.dep>");
        return Err("arguments invalides".into());
    }

    eprintln!("Lecture du corpus d'entraînement : {}", args[1]);
    let train = read_conllu(&args[1])?;
    let dev = read_conllu(&args[2])?;
    let n_tok: usize = train.iter().map(|s| s.forms.len()).sum();
    eprintln!("  {} phrases, {n_tok} tokens", train.len());

    // POS prédites, précalculées une fois par phrase (réutilisées chaque époque).
    eprintln!("Étiquetage POS (CRF) des phrases d'entraînement…");
    let train_pos: Vec<Vec<Upos>> = train.iter().map(|s| predict_pos(&s.forms)).collect();

    // Vocabulaire des traits, le long des chemins de l'oracle.
    eprintln!("Construction du vocabulaire de traits (min. {MIN_COUNT})…");
    let mut counts: HashMap<String, u32> = HashMap::new();
    for (s, upos) in train.iter().zip(&train_pos) {
        rollout(s, upos, |feats, _gold| {
            for f in feats {
                *counts.entry(f.clone()).or_insert(0) += 1;
            }
        });
    }
    let mut kept: Vec<String> = counts
        .into_iter()
        .filter(|(_, c)| *c >= MIN_COUNT)
        .map(|(a, _)| a)
        .collect();
    kept.sort();
    let vocab: HashMap<String, u32> = kept
        .into_iter()
        .enumerate()
        .map(|(i, a)| (a, i as u32))
        .collect();
    let rows = vocab.len();
    eprintln!("  {rows} traits retenus → {} poids", rows * NMOVE);

    // Entraînement perceptron moyenné, teacher forcing sur l'oracle.
    eprintln!("Entraînement (perceptron moyenné, {EPOCHS} époques)…");
    let mut model = Perceptron::new(rows, NMOVE);
    let mut scores = vec![0.0f64; NMOVE];
    for epoch in 0..EPOCHS {
        let mut updates = 0usize;
        let mut steps = 0usize;
        for (s, upos) in train.iter().zip(&train_pos) {
            // Rollout avec accès au modèle : on doit prédire et comparer.
            let fr: Vec<&str> = s.forms.iter().map(String::as_str).collect();
            let mut st = State::new(&fr, upos);
            let mut attrs: Vec<String> = Vec::new();
            let max_steps = 2 * s.forms.len() + 2;
            let mut st_steps = 0;
            while !st.is_final() && st_steps < max_steps {
                st_steps += 1;
                steps += 1;
                let gold = oracle(&st, &s.gold_head, &s.gold_label);
                dep::transition_features(&st, &mut attrs);
                let rows_active = lookup(&attrs, &vocab);
                model.score(&rows_active, &mut scores);

                // Meilleure transition légale selon le modèle courant.
                let mut pred = Move::Shift;
                let mut best = f64::NEG_INFINITY;
                let mut best_pr = i32::MIN;
                for c in 0..NMOVE {
                    let mv = Move::from_class(c);
                    if !st.is_legal(mv) {
                        continue;
                    }
                    let sc = scores[c];
                    let pr = fallback_priority(mv);
                    if sc > best || (sc == best && pr > best_pr) {
                        best = sc;
                        best_pr = pr;
                        pred = mv;
                    }
                }

                model.t += 1;
                if pred.class() != gold.class() {
                    updates += 1;
                    let gc = gold.class();
                    let pc = pred.class();
                    for &r in &rows_active {
                        model.add(r as usize, gc, 1.0);
                        model.add(r as usize, pc, -1.0);
                    }
                }
                st.apply(gold); // teacher forcing
            }
        }
        // Évaluation périodique sur les poids moyennés courants.
        let avg = model.averaged();
        let (uas, las) = evaluate(&dev, &avg, &vocab);
        eprintln!(
            "  époque {epoch:2} : {updates} maj / {steps} transitions  |  dev UAS {:.2} % LAS {:.2} %",
            uas * 100.0,
            las * 100.0
        );
    }

    let avg = model.averaged();
    let (uas, las) = evaluate(&dev, &avg, &vocab);
    eprintln!("Dev final : UAS {:.2} %  LAS {:.2} %", uas * 100.0, las * 100.0);

    // Sérialisation.
    let attr_rows: Vec<(String, u32)> = vocab.into_iter().collect();
    let bytes = dep::serialize_model(&avg, &attr_rows)?;
    let out = PathBuf::from(&args[3]);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, &bytes)?;
    eprintln!(
        "Modèle écrit : {} ({:.2} Mo)",
        out.display(),
        bytes.len() as f64 / 1_048_576.0
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
