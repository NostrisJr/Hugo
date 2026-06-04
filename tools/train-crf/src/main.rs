//! Entraînement du CRF d'étiquetage morphosyntaxique (POS) de Hugo.
//!
//! Lit le CoNLL-U de **Universal Dependencies French-GSD**, construit les traits
//! via [`hugo_core::pos::observation_attributes`] (les *mêmes* qu'au décodage),
//! entraîne un CRF à chaîne linéaire par maximum de vraisemblance pénalisé (L2),
//! évalue l'exactitude POS, puis sérialise le modèle quantifié vers l'asset
//! embarqué `crates/hugo-core/assets/pos.crf` lu par le runtime.
//!
//! ```text
//! cargo run -p train-crf --release -- <train.conllu> <dev.conllu> <test.conllu> <sortie.crf>
//! ```
//! (`<test.conllu>` peut être remplacé par `-` pour l'omettre.)
//!
//! L'optimisation (forward-backward + L-BFGS) est écrite à la main, sans
//! dépendance externe : seul `hugo-core` est requis (extraction + sérialisation).

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use hugo_core::pos::{self, Upos};

/// Occurrences minimales d'un attribut pour être retenu (élagage des traits
/// rares — limite la taille du modèle et le surapprentissage).
const MIN_COUNT: u32 = 3;
/// Coefficient de régularisation L2.
const L2: f64 = 1.0;
/// Itérations maximales de L-BFGS.
const MAX_ITER: usize = 200;
/// Taille de l'historique L-BFGS (paires (s, y) conservées).
const LBFGS_HISTORY: usize = 8;

/// Une phrase annotée : formes des tokens et indices d'étiquettes de référence.
struct Sentence {
    forms: Vec<String>,
    golds: Vec<usize>,
}

/// Jeu de données prêt pour l'entraînement : phrases + lignes d'attributs
/// actives précalculées (par phrase, par position).
struct Dataset {
    sentences: Vec<Sentence>,
    active: Vec<Vec<Vec<u32>>>,
}

// --- Lecture CoNLL-U ----------------------------------------------------------

/// Lit un fichier CoNLL-U en une liste de phrases `(formes, étiquettes)`.
///
/// Ignore les lignes de commentaire (`#`) et les tokens multi-mots (id `n-m` ou
/// `n.m`). L'étiquette est l'UPOS (colonne 4) ; une étiquette inconnue tombe sur
/// [`Upos::X`].
fn read_conllu(path: &str) -> std::io::Result<Vec<Sentence>> {
    let reader = BufReader::new(File::open(path)?);
    let mut sentences = Vec::new();
    let mut forms = Vec::new();
    let mut golds = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim_end();
        if line.is_empty() {
            if !forms.is_empty() {
                sentences.push(Sentence {
                    forms: std::mem::take(&mut forms),
                    golds: std::mem::take(&mut golds),
                });
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 {
            continue;
        }
        // Tokens multi-mots (« du » = de+le) et nœuds vides : ignorés.
        if cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        let upos = Upos::from_conllu(cols[3]).unwrap_or(Upos::X);
        forms.push(cols[1].to_string());
        golds.push(upos.code() as usize);
    }
    if !forms.is_empty() {
        sentences.push(Sentence { forms, golds });
    }
    Ok(sentences)
}

// --- Vocabulaire d'attributs --------------------------------------------------

/// Parcourt les phrases, compte les attributs et retient ceux vus au moins
/// [`MIN_COUNT`] fois. Renvoie la table `attribut → identifiant de ligne` (ids
/// attribués par ordre lexicographique, pour un modèle déterministe).
fn build_vocab(sentences: &[Sentence]) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut attrs: Vec<String> = Vec::new();
    for s in sentences {
        let forms: Vec<&str> = s.forms.iter().map(String::as_str).collect();
        for p in 0..forms.len() {
            pos::observation_attributes(&forms, p, &mut attrs);
            for a in &attrs {
                *counts.entry(a.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut kept: Vec<String> = counts
        .into_iter()
        .filter(|(_, c)| *c >= MIN_COUNT)
        .map(|(a, _)| a)
        .collect();
    kept.sort();
    kept.into_iter()
        .enumerate()
        .map(|(i, a)| (a, i as u32))
        .collect()
}

/// Précalcule, pour chaque position de chaque phrase, les identifiants de lignes
/// d'attributs actives (présentes dans le vocabulaire).
fn build_active(sentences: &[Sentence], vocab: &HashMap<String, u32>) -> Vec<Vec<Vec<u32>>> {
    let mut attrs: Vec<String> = Vec::new();
    sentences
        .iter()
        .map(|s| {
            let forms: Vec<&str> = s.forms.iter().map(String::as_str).collect();
            (0..forms.len())
                .map(|p| {
                    pos::observation_attributes(&forms, p, &mut attrs);
                    attrs.iter().filter_map(|a| vocab.get(a).copied()).collect()
                })
                .collect()
        })
        .collect()
}

// --- Algèbre CRF --------------------------------------------------------------

const T: usize = 17; // nombre d'étiquettes (= Upos::ALL.len())

#[inline]
fn logsumexp(xs: &[f64]) -> f64 {
    let m = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if m == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    let sum: f64 = xs.iter().map(|x| (x - m).exp()).sum();
    m + sum.ln()
}

/// Perte (log-vraisemblance négative) et gradient d'une **seule** phrase,
/// accumulés dans `grad`. `state` pointe sur les `rows·T` poids d'état, `trans`
/// sur les `T·T` transitions, au sein de `params` ; `grad` a la même disposition.
fn sentence_grad(
    golds: &[usize],
    active: &[Vec<u32>],
    params: &[f64],
    rows: usize,
    grad: &mut [f64],
) -> f64 {
    let n = golds.len();
    if n == 0 {
        return 0.0;
    }
    let trans_off = rows * T;
    let trans = &params[trans_off..];

    // Potentiels d'émission U[p·T + k].
    let mut u = vec![0.0f64; n * T];
    for (p, rows) in active.iter().enumerate() {
        let up = p * T;
        for &row in rows {
            let base = row as usize * T;
            for k in 0..T {
                u[up + k] += params[base + k];
            }
        }
    }

    // Forward : alpha[p·T + k].
    let mut alpha = vec![0.0f64; n * T];
    alpha[..T].copy_from_slice(&u[..T]);
    let mut tmp = [0.0f64; T];
    for p in 1..n {
        for k in 0..T {
            for prev in 0..T {
                tmp[prev] = alpha[(p - 1) * T + prev] + trans[prev * T + k];
            }
            alpha[p * T + k] = u[p * T + k] + logsumexp(&tmp);
        }
    }
    let logz = logsumexp(&alpha[(n - 1) * T..n * T]);

    // Backward : beta[p·T + k] (dernière ligne nulle).
    let mut beta = vec![0.0f64; n * T];
    for p in (0..n - 1).rev() {
        for k in 0..T {
            for j in 0..T {
                tmp[j] = trans[k * T + j] + u[(p + 1) * T + j] + beta[(p + 1) * T + j];
            }
            beta[p * T + k] = logsumexp(&tmp);
        }
    }

    // Perte : logZ − score(référence).
    let mut gold_score = 0.0;
    for p in 0..n {
        gold_score += u[p * T + golds[p]];
    }
    for p in 1..n {
        gold_score += trans[golds[p - 1] * T + golds[p]];
    }
    let loss = logz - gold_score;

    // Gradient d'état : Σ (P(y_p=k) − 1[y*_p=k]) sur les lignes actives.
    let mut marg = [0.0f64; T];
    for (p, rows) in active.iter().enumerate() {
        for (k, m) in marg.iter_mut().enumerate() {
            *m = (alpha[p * T + k] + beta[p * T + k] - logz).exp();
        }
        for &row in rows {
            let base = row as usize * T;
            for k in 0..T {
                grad[base + k] += marg[k];
            }
            grad[base + golds[p]] -= 1.0;
        }
    }

    // Gradient de transition : Σ (P(y_{p-1}=j, y_p=k) − 1[…]).
    for p in 1..n {
        for j in 0..T {
            let aj = alpha[(p - 1) * T + j];
            for k in 0..T {
                let m = (aj + trans[j * T + k] + u[p * T + k] + beta[p * T + k] - logz).exp();
                grad[trans_off + j * T + k] += m;
            }
        }
        grad[trans_off + golds[p - 1] * T + golds[p]] -= 1.0;
    }

    loss
}

/// Perte et gradient sur **tout** le jeu, parallélisés par phrases.
fn objective(data: &Dataset, params: &[f64], rows: usize) -> (f64, Vec<f64>) {
    let p = params.len();
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(data.sentences.len().max(1));

    let chunk = data.sentences.len().div_ceil(n_threads.max(1));
    let (mut loss, mut grad) = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for t in 0..n_threads {
            let start = t * chunk;
            if start >= data.sentences.len() {
                break;
            }
            let end = (start + chunk).min(data.sentences.len());
            handles.push(scope.spawn(move || {
                let mut g = vec![0.0f64; p];
                let mut l = 0.0;
                for i in start..end {
                    l += sentence_grad(
                        &data.sentences[i].golds,
                        &data.active[i],
                        params,
                        rows,
                        &mut g,
                    );
                }
                (l, g)
            }));
        }
        let mut loss = 0.0;
        let mut grad = vec![0.0f64; p];
        for h in handles {
            let (l, g) = h.join().expect("thread d'objectif");
            loss += l;
            for (a, b) in grad.iter_mut().zip(g) {
                *a += b;
            }
        }
        (loss, grad)
    });

    // Régularisation L2.
    for i in 0..p {
        loss += 0.5 * L2 * params[i] * params[i];
        grad[i] += L2 * params[i];
    }
    (loss, grad)
}

// --- L-BFGS -------------------------------------------------------------------

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn inf_norm(a: &[f64]) -> f64 {
    a.iter().fold(0.0, |m, &x| m.max(x.abs()))
}

/// Minimise l'objectif CRF par L-BFGS avec recherche linéaire d'Armijo.
fn train(data: &Dataset, n_params: usize, rows: usize) -> Vec<f64> {
    let mut x = vec![0.0f64; n_params];
    let (mut fx, mut g) = objective(data, &x, rows);

    let mut s_hist: VecDeque<Vec<f64>> = VecDeque::new();
    let mut y_hist: VecDeque<Vec<f64>> = VecDeque::new();
    let mut rho_hist: VecDeque<f64> = VecDeque::new();

    for iter in 0..MAX_ITER {
        // Direction : -H·g par la récursion à deux boucles.
        let mut q = g.clone();
        let mut alphas = Vec::with_capacity(s_hist.len());
        for (s, (y, &rho)) in s_hist
            .iter()
            .rev()
            .zip(y_hist.iter().rev().zip(rho_hist.iter().rev()))
        {
            let a = rho * dot(s, &q);
            for i in 0..n_params {
                q[i] -= a * y[i];
            }
            alphas.push(a);
        }
        let gamma = match (s_hist.back(), y_hist.back()) {
            (Some(s), Some(y)) => dot(s, y) / dot(y, y).max(1e-12),
            _ => 1.0,
        };
        for v in q.iter_mut() {
            *v *= gamma;
        }
        for ((s, y), (&rho, &a)) in s_hist
            .iter()
            .zip(y_hist.iter())
            .zip(rho_hist.iter().zip(alphas.iter().rev()))
        {
            let b = rho * dot(y, &q);
            for i in 0..n_params {
                q[i] += (a - b) * s[i];
            }
        }
        let mut d: Vec<f64> = q.iter().map(|v| -v).collect();

        // Garde-fou : direction de descente.
        let mut dg = dot(&d, &g);
        if dg >= 0.0 {
            d = g.iter().map(|v| -v).collect();
            dg = dot(&d, &g);
        }

        // Recherche linéaire (backtracking Armijo).
        let mut step = if iter == 0 {
            (1.0 / inf_norm(&g).max(1e-12)).min(1.0)
        } else {
            1.0
        };
        let c1 = 1e-4;
        let mut x_new = x.clone();
        let mut f_new;
        let mut g_new;
        loop {
            for i in 0..n_params {
                x_new[i] = x[i] + step * d[i];
            }
            let r = objective(data, &x_new, rows);
            f_new = r.0;
            g_new = r.1;
            if f_new <= fx + c1 * step * dg || step < 1e-12 {
                break;
            }
            step *= 0.5;
        }

        let rel = (fx - f_new).abs() / fx.abs().max(1.0);
        eprintln!(
            "  itér {iter:3} : perte = {f_new:.2}  |∇|∞ = {:.4}  pas = {step:.3e}",
            inf_norm(&g_new)
        );

        // Mise à jour de l'historique.
        let s: Vec<f64> = (0..n_params).map(|i| x_new[i] - x[i]).collect();
        let y: Vec<f64> = (0..n_params).map(|i| g_new[i] - g[i]).collect();
        let ys = dot(&y, &s);
        if ys > 1e-10 {
            if s_hist.len() == LBFGS_HISTORY {
                s_hist.pop_front();
                y_hist.pop_front();
                rho_hist.pop_front();
            }
            s_hist.push_back(s);
            y_hist.push_back(y);
            rho_hist.push_back(1.0 / ys);
        }

        x = x_new;
        fx = f_new;
        g = g_new;

        if rel < 1e-5 {
            eprintln!("  convergence (variation relative < 1e-5).");
            break;
        }
    }
    x
}

// --- Décodage & évaluation ----------------------------------------------------

/// Viterbi en virgule flottante, sur les poids en mémoire (pour l'évaluation).
fn decode(forms: &[&str], params: &[f64], rows: usize, vocab: &HashMap<String, u32>) -> Vec<usize> {
    let n = forms.len();
    if n == 0 {
        return Vec::new();
    }
    let trans = &params[rows * T..];
    let mut attrs: Vec<String> = Vec::new();

    let mut emit = vec![0.0f64; n * T];
    for p in 0..n {
        pos::observation_attributes(forms, p, &mut attrs);
        for a in &attrs {
            if let Some(&row) = vocab.get(a) {
                let base = row as usize * T;
                for k in 0..T {
                    emit[p * T + k] += params[base + k];
                }
            }
        }
    }

    let mut dp = vec![0.0f64; n * T];
    let mut back = vec![0usize; n * T];
    dp[..T].copy_from_slice(&emit[..T]);
    for p in 1..n {
        for k in 0..T {
            let mut best = f64::NEG_INFINITY;
            let mut bp = 0;
            for prev in 0..T {
                let sc = dp[(p - 1) * T + prev] + trans[prev * T + k];
                if sc > best {
                    best = sc;
                    bp = prev;
                }
            }
            dp[p * T + k] = best + emit[p * T + k];
            back[p * T + k] = bp;
        }
    }

    let mut last = 0;
    let mut best = f64::NEG_INFINITY;
    for k in 0..T {
        if dp[(n - 1) * T + k] > best {
            best = dp[(n - 1) * T + k];
            last = k;
        }
    }
    let mut path = vec![0usize; n];
    path[n - 1] = last;
    for p in (1..n).rev() {
        path[p - 1] = back[p * T + path[p]];
    }
    path
}

/// Exactitude POS par token sur un jeu.
fn accuracy(
    sentences: &[Sentence],
    params: &[f64],
    rows: usize,
    vocab: &HashMap<String, u32>,
) -> f64 {
    let mut correct = 0usize;
    let mut total = 0usize;
    for s in sentences {
        let forms: Vec<&str> = s.forms.iter().map(String::as_str).collect();
        let pred = decode(&forms, params, rows, vocab);
        for (p, g) in pred.iter().zip(&s.golds) {
            if p == g {
                correct += 1;
            }
            total += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        correct as f64 / total as f64
    }
}

// --- Pilote -------------------------------------------------------------------

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage : train-crf <train.conllu> <dev.conllu> <test.conllu|-> <sortie.crf>");
        return Err("arguments invalides".into());
    }
    assert_eq!(Upos::ALL.len(), T, "T doit valoir Upos::ALL.len()");

    eprintln!("Lecture du corpus d'entraînement : {}", args[1]);
    let train_set = read_conllu(&args[1])?;
    let dev = read_conllu(&args[2])?;
    let tokens: usize = train_set.iter().map(|s| s.forms.len()).sum();
    eprintln!("  {} phrases, {tokens} tokens", train_set.len());

    eprintln!("Construction du vocabulaire d'attributs (min. {MIN_COUNT} occurrences)…");
    let vocab = build_vocab(&train_set);
    let rows = vocab.len();
    let n_params = rows * T + T * T;
    eprintln!("  {rows} attributs retenus → {n_params} poids");

    eprintln!("Précalcul des traits actifs…");
    let data = Dataset {
        active: build_active(&train_set, &vocab),
        sentences: train_set,
    };

    eprintln!("Entraînement (L-BFGS, L2 = {L2}, max {MAX_ITER} itérations)…");
    let params = train(&data, n_params, rows);

    let dev_acc = accuracy(&dev, &params, rows, &vocab);
    eprintln!("Exactitude POS dev : {:.2} %", dev_acc * 100.0);
    if args[3] != "-" {
        let test = read_conllu(&args[3])?;
        let test_acc = accuracy(&test, &params, rows, &vocab);
        eprintln!("Exactitude POS test : {:.2} %", test_acc * 100.0);
    }

    // Sérialisation vers l'asset embarqué.
    let labels = Upos::ALL.to_vec();
    let state_weights: Vec<f32> = params[..rows * T].iter().map(|&w| w as f32).collect();
    let transitions: Vec<f32> = params[rows * T..].iter().map(|&w| w as f32).collect();
    let mut attr_rows: Vec<(String, u32)> = vocab.into_iter().collect();
    attr_rows.sort_by(|a, b| a.1.cmp(&b.1));
    let bytes = pos::serialize_model(&labels, &transitions, &state_weights, &attr_rows)?;

    let out = PathBuf::from(&args[4]);
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
