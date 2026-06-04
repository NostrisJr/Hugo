//! Étiquetage morphosyntaxique (POS) par CRF à chaîne linéaire.
//!
//! Ce module désambiguïse la catégorie grammaticale de chaque token. Là où
//! [`morpho::lookup`](crate::morpho::lookup) renvoie *toutes* les analyses
//! possibles d'une forme (`Vec<Morph>`), l'étiqueteur tranche et assigne à
//! chaque token **une** étiquette [`Upos`] (Universal POS), la plus probable
//! dans son contexte. Les règles grammaticales peuvent alors s'appuyer sur une
//! catégorie unique au lieu d'inspecter toutes les analyses candidates.
//!
//! Le modèle est un CRF linéaire entraîné hors-ligne sur Universal Dependencies
//! French-GSD (voir `tools/train-crf`) et **embarqué** dans la bibliothèque
//! (`assets/pos.crf`). Le décodage runtime est un Viterbi en O(n·T²). Le modèle
//! est chargé paresseusement, une seule fois, via un [`OnceLock`] — comme le
//! lexique morphologique.
//!
//! La fonction d'extraction de traits [`observation_attributes`] est **partagée**
//! avec l'entraîneur : c'est le contrat qui garantit que les features vues à
//! l'entraînement et au décodage sont rigoureusement identiques.

use std::sync::OnceLock;

use fst::Map as FstMap;

use crate::morpho::{self, MorphCategory};
use crate::tokenizer::{Token, TokenKind};

/// Étiquette morphosyntaxique universelle (jeu UPOS de Universal Dependencies).
///
/// Plus fine que [`MorphCategory`] : elle distingue notamment l'auxiliaire
/// ([`Upos::Aux`]) du verbe plein ([`Upos::Verb`]) — distinction nécessaire à
/// l'accord du participe passé — ainsi que les conjonctions de coordination et
/// de subordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Upos {
    /// Adjectif (`ADJ`).
    Adj,
    /// Adposition / préposition (`ADP`).
    Adp,
    /// Adverbe (`ADV`).
    Adv,
    /// Auxiliaire (`AUX`) — être, avoir employés comme auxiliaires.
    Aux,
    /// Conjonction de coordination (`CCONJ`).
    Cconj,
    /// Déterminant (`DET`).
    Det,
    /// Interjection (`INTJ`).
    Intj,
    /// Nom commun (`NOUN`).
    Noun,
    /// Numéral (`NUM`).
    Num,
    /// Particule (`PART`).
    Part,
    /// Pronom (`PRON`).
    Pron,
    /// Nom propre (`PROPN`).
    Propn,
    /// Ponctuation (`PUNCT`).
    Punct,
    /// Conjonction de subordination (`SCONJ`).
    Sconj,
    /// Symbole (`SYM`).
    Sym,
    /// Verbe plein (`VERB`).
    Verb,
    /// Autre / indéterminé (`X`).
    X,
}

impl Upos {
    /// Toutes les étiquettes, dans l'ordre de leur code de sérialisation.
    pub const ALL: [Upos; 17] = [
        Upos::Adj,
        Upos::Adp,
        Upos::Adv,
        Upos::Aux,
        Upos::Cconj,
        Upos::Det,
        Upos::Intj,
        Upos::Noun,
        Upos::Num,
        Upos::Part,
        Upos::Pron,
        Upos::Propn,
        Upos::Punct,
        Upos::Sconj,
        Upos::Sym,
        Upos::Verb,
        Upos::X,
    ];

    /// Code compact (0..16) utilisé pour la sérialisation du modèle.
    pub fn code(self) -> u8 {
        Upos::ALL.iter().position(|&u| u == self).unwrap() as u8
    }

    /// Étiquette correspondant à un code compact, ou `None` si hors plage.
    pub fn from_code(code: u8) -> Option<Upos> {
        Upos::ALL.get(code as usize).copied()
    }

    /// Convertit une étiquette UPOS CoNLL-U (« NOUN », « VERB »…) en [`Upos`].
    pub fn from_conllu(tag: &str) -> Option<Upos> {
        Some(match tag {
            "ADJ" => Upos::Adj,
            "ADP" => Upos::Adp,
            "ADV" => Upos::Adv,
            "AUX" => Upos::Aux,
            "CCONJ" => Upos::Cconj,
            "DET" => Upos::Det,
            "INTJ" => Upos::Intj,
            "NOUN" => Upos::Noun,
            "NUM" => Upos::Num,
            "PART" => Upos::Part,
            "PRON" => Upos::Pron,
            "PROPN" => Upos::Propn,
            "PUNCT" => Upos::Punct,
            "SCONJ" => Upos::Sconj,
            "SYM" => Upos::Sym,
            "VERB" => Upos::Verb,
            "X" => Upos::X,
            _ => return None,
        })
    }

    /// Projette l'étiquette fine sur la [`MorphCategory`] grossière consommée par
    /// les règles existantes.
    pub fn to_category(self) -> MorphCategory {
        match self {
            Upos::Adj => MorphCategory::Adjective,
            Upos::Adp => MorphCategory::Preposition,
            Upos::Adv => MorphCategory::Adverb,
            Upos::Aux | Upos::Verb => MorphCategory::Verb,
            Upos::Cconj | Upos::Sconj => MorphCategory::Conjunction,
            Upos::Det => MorphCategory::Determiner,
            Upos::Intj => MorphCategory::Interjection,
            Upos::Noun | Upos::Propn => MorphCategory::Noun,
            Upos::Pron => MorphCategory::Pronoun,
            Upos::Num | Upos::Part | Upos::Sym | Upos::X | Upos::Punct => MorphCategory::Unknown,
        }
    }

    /// Nom court de l'étiquette (utile au débogage et aux messages).
    pub fn as_str(self) -> &'static str {
        match self {
            Upos::Adj => "ADJ",
            Upos::Adp => "ADP",
            Upos::Adv => "ADV",
            Upos::Aux => "AUX",
            Upos::Cconj => "CCONJ",
            Upos::Det => "DET",
            Upos::Intj => "INTJ",
            Upos::Noun => "NOUN",
            Upos::Num => "NUM",
            Upos::Part => "PART",
            Upos::Pron => "PRON",
            Upos::Propn => "PROPN",
            Upos::Punct => "PUNCT",
            Upos::Sconj => "SCONJ",
            Upos::Sym => "SYM",
            Upos::Verb => "VERB",
            Upos::X => "X",
        }
    }
}

/// Un token étiqueté : son étiquette fine [`Upos`] et la [`MorphCategory`]
/// grossière qui en découle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tagged {
    /// Étiquette POS fine choisie par le CRF.
    pub upos: Upos,
    /// Catégorie grossière correspondante (`upos.to_category()`).
    pub category: MorphCategory,
}

impl Tagged {
    fn new(upos: Upos) -> Self {
        Tagged {
            upos,
            category: upos.to_category(),
        }
    }
}

// --- Extraction de traits (partagée entre l'entraîneur et le décodeur). ---

/// Code court d'une catégorie morphologique, pour les traits « sac de
/// catégories candidates ».
fn category_code(c: MorphCategory) -> &'static str {
    match c {
        MorphCategory::Noun => "N",
        MorphCategory::Verb => "V",
        MorphCategory::Adjective => "A",
        MorphCategory::Determiner => "D",
        MorphCategory::Pronoun => "P",
        MorphCategory::Adverb => "R",
        MorphCategory::Preposition => "S",
        MorphCategory::Conjunction => "C",
        MorphCategory::Interjection => "I",
        MorphCategory::Unknown => "U",
    }
}

/// Catégories morphologiques distinctes proposées par le lexique pour `form`.
fn candidate_categories(form: &str) -> Vec<MorphCategory> {
    let mut cats: Vec<MorphCategory> = Vec::new();
    for m in morpho::lookup(form) {
        if !cats.contains(&m.category) {
            cats.push(m.category);
        }
    }
    cats
}

/// Caractéristiques de forme d'un mot (casse, chiffres, ponctuation…).
fn shape_attributes(word: &str, off: i32, out: &mut Vec<String>) {
    let chars: Vec<char> = word.chars().collect();
    let mut first_upper = false;
    let mut has_digit = false;
    let mut has_alpha = false;
    let mut all_upper = true;
    for (k, c) in chars.iter().enumerate() {
        if c.is_numeric() {
            has_digit = true;
        }
        if c.is_alphabetic() {
            has_alpha = true;
            if c.is_uppercase() {
                if k == 0 {
                    first_upper = true;
                }
            } else {
                all_upper = false;
            }
        }
    }
    if first_upper {
        out.push(format!("sh:cap[{off}]"));
    }
    if has_alpha && all_upper && chars.len() > 1 {
        out.push(format!("sh:allcaps[{off}]"));
    }
    if has_digit {
        out.push(format!("sh:digit[{off}]"));
    }
    if !has_alpha && !has_digit {
        out.push(format!("sh:punct[{off}]"));
    }
    if word.contains('-') {
        out.push(format!("sh:hyphen[{off}]"));
    }
    if word.contains('\'') || word.contains('\u{2019}') {
        out.push(format!("sh:apos[{off}]"));
    }
}

/// Préfixe / suffixe de `n` caractères (Unicode), ou `None` si le mot est trop
/// court.
fn prefix(word: &str, n: usize) -> Option<String> {
    let chars: Vec<char> = word.chars().collect();
    (chars.len() >= n).then(|| chars[..n].iter().collect())
}

fn suffix(word: &str, n: usize) -> Option<String> {
    let chars: Vec<char> = word.chars().collect();
    (chars.len() >= n).then(|| chars[chars.len() - n..].iter().collect())
}

/// Produit les **attributs** d'observation à la position `i` de la séquence de
/// formes `forms`, en les ajoutant à `out` (vidé au préalable par l'appelant).
///
/// Un attribut est une chaîne (ex. `w[0]=chat`, `suf3[0]=hat`, `c[0]=N`,
/// `w[-1]=le`) ; le CRF apprend un poids par couple (attribut, étiquette). Les
/// traits portent l'offset relatif (`[-2]`…`[2]`) pour distinguer la position.
///
/// **Cette fonction est le contrat de cohérence entraînement/décodage** : elle
/// est appelée à l'identique par `tools/train-crf` et par [`tag`].
pub fn observation_attributes(forms: &[&str], i: usize, out: &mut Vec<String>) {
    out.clear();
    // Trait de biais : capte la distribution a priori des étiquettes.
    out.push("bias".to_string());

    for off in [-2i32, -1, 0, 1, 2] {
        let idx = i as i32 + off;
        if idx < 0 || idx as usize >= forms.len() {
            // Marqueur de bord (début/fin de phrase).
            out.push(format!("w[{off}]=<BND>"));
            continue;
        }
        let word = forms[idx as usize];
        let lower = word.to_lowercase();
        out.push(format!("w[{off}]={lower}"));

        // Sac des catégories candidates du lexique sur la fenêtre proche.
        if (-1..=1).contains(&off) {
            for c in candidate_categories(word) {
                out.push(format!("c[{off}]={}", category_code(c)));
            }
        }

        // Préfixes/suffixes : longueur 1–4 au centre, 2–3 sur les voisins.
        if off == 0 {
            for n in 1..=4 {
                if let Some(p) = prefix(&lower, n) {
                    out.push(format!("pre{n}[0]={p}"));
                }
                if let Some(s) = suffix(&lower, n) {
                    out.push(format!("suf{n}[0]={s}"));
                }
            }
            shape_attributes(word, off, out);
        } else if off == -1 || off == 1 {
            for n in [2, 3] {
                if let Some(s) = suffix(&lower, n) {
                    out.push(format!("suf{n}[{off}]={s}"));
                }
            }
            shape_attributes(word, off, out);
        }
    }
}

// --- Format binaire du modèle (`assets/pos.crf`). ---
//
// Disposition (entiers en little-endian) :
//   magic        : 8 octets (= MAGIC)
//   t            : u8                       nombre d'étiquettes
//   labels       : t × u8                   codes des étiquettes (Upos::code)
//   state_scale  : f32                      échelle des poids d'état quantifiés
//   trans_scale  : f32                      échelle des transitions quantifiées
//   transitions  : t·t × i16                trans[prev·t + cur]
//   rows         : u32                      nombre de lignes d'attributs
//   state_w      : rows·t × i16             state[row·t + label]
//   fst_len      : u32                      taille de la FST d'attributs
//   fst          : fst_len octets           fst::Map<attribut → row_id (u64)>

const MAGIC: &[u8; 8] = b"HUGOCRF\x01";

/// Le modèle CRF embarqué, ou un slice vide tant qu'il n'a pas été entraîné
/// (bootstrap). Le chargement ([`load`]) tolère ce cas et bascule sur un modèle
/// dégénéré plutôt que de paniquer.
static MODEL_BYTES: &[u8] = include_bytes!("../assets/pos.crf");

/// Modèle CRF chargé en mémoire et prêt pour le décodage.
struct Model {
    t: usize,
    labels: Vec<Upos>,
    state_scale: f32,
    /// Transitions déquantifiées, indexées `prev * t + cur`.
    transitions: Vec<f32>,
    /// FST des attributs → identifiant de ligne. `None` pour le modèle dégénéré.
    attr: Option<FstMap<&'static [u8]>>,
    /// Poids d'état bruts (i16 little-endian), `rows * t` valeurs.
    state_raw: &'static [u8],
    rows: usize,
}

/// Petit curseur de lecture little-endian sur un slice statique.
struct Cursor {
    data: &'static [u8],
    pos: usize,
}

impl Cursor {
    fn new(data: &'static [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'static [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u32(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f32(&mut self) -> Option<f32> {
        let b = self.take(4)?;
        Some(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

#[inline]
fn read_i16(raw: &[u8], idx: usize) -> i16 {
    let b = idx * 2;
    i16::from_le_bytes([raw[b], raw[b + 1]])
}

impl Model {
    /// Modèle dégénéré (aucun trait), utilisé tant que le CRF n'est pas entraîné.
    fn degenerate() -> Self {
        let labels = Upos::ALL.to_vec();
        let t = labels.len();
        Model {
            t,
            labels,
            state_scale: 1.0,
            transitions: vec![0.0; t * t],
            attr: None,
            state_raw: &[],
            rows: 0,
        }
    }

    /// Analyse les octets du modèle embarqué. Renvoie `None` si le format est
    /// invalide ou les octets vides (déclenche le repli dégénéré).
    fn parse(bytes: &'static [u8]) -> Option<Model> {
        if bytes.len() < MAGIC.len() {
            return None;
        }
        let mut cur = Cursor::new(bytes);
        if cur.take(MAGIC.len())? != MAGIC {
            return None;
        }
        let t = cur.u8()? as usize;
        let mut labels = Vec::with_capacity(t);
        for _ in 0..t {
            labels.push(Upos::from_code(cur.u8()?)?);
        }
        let state_scale = cur.f32()?;
        let trans_scale = cur.f32()?;
        let trans_raw = cur.take(t * t * 2)?;
        let transitions = (0..t * t)
            .map(|k| read_i16(trans_raw, k) as f32 * trans_scale)
            .collect::<Vec<f32>>();
        let rows = cur.u32()? as usize;
        let state_raw = cur.take(rows * t * 2)?;
        let fst_len = cur.u32()? as usize;
        let fst_bytes = cur.take(fst_len)?;
        let attr = FstMap::new(fst_bytes).ok()?;

        Some(Model {
            t,
            labels,
            state_scale,
            transitions,
            attr: Some(attr),
            state_raw,
            rows,
        })
    }

    /// Poids d'état pour la ligne `row` et l'étiquette `label`.
    #[inline]
    fn state_weight(&self, row: usize, label: usize) -> f32 {
        debug_assert!(row < self.rows && label < self.t);
        read_i16(self.state_raw, row * self.t + label) as f32 * self.state_scale
    }

    /// Lignes d'attributs actives à une position (identifiants dans la FST).
    fn active_rows(&self, attrs: &[String]) -> Vec<usize> {
        let Some(map) = &self.attr else {
            return Vec::new();
        };
        let mut rows = Vec::with_capacity(attrs.len());
        for a in attrs {
            if let Some(id) = map.get(a.as_bytes()) {
                rows.push(id as usize);
            }
        }
        rows
    }

    /// Scores d'état (émission) par position : `emit[p * t + l]`, somme des
    /// poids des attributs actifs en `p` pour l'étiquette `l`.
    fn emissions(&self, forms: &[&str]) -> Vec<f32> {
        let t = self.t;
        let mut emit = vec![0.0f32; forms.len() * t];
        let mut attrs: Vec<String> = Vec::new();
        for p in 0..forms.len() {
            observation_attributes(forms, p, &mut attrs);
            for &row in &self.active_rows(&attrs) {
                for l in 0..t {
                    emit[p * t + l] += self.state_weight(row, l);
                }
            }
        }
        emit
    }

    /// Meilleur score de chemin Viterbi pour `forms` (log-score non normalisé).
    ///
    /// Sert à comparer deux graphies (étiquetage contrefactuel) : un score
    /// nettement plus élevé désigne la lecture la plus plausible.
    fn best_score(&self, forms: &[&str]) -> f32 {
        let n = forms.len();
        if n == 0 {
            return 0.0;
        }
        let t = self.t;
        let emit = self.emissions(forms);
        let mut dp = emit[..t].to_vec();
        for p in 1..n {
            let mut next = vec![f32::NEG_INFINITY; t];
            for l in 0..t {
                let mut best = f32::NEG_INFINITY;
                for (prev, &dpv) in dp.iter().enumerate() {
                    best = best.max(dpv + self.transitions[prev * t + l]);
                }
                next[l] = best + emit[p * t + l];
            }
            dp = next;
        }
        dp.into_iter().fold(f32::NEG_INFINITY, f32::max)
    }

    /// Décodage de Viterbi : étiquette la séquence de formes `forms`.
    fn viterbi(&self, forms: &[&str]) -> Vec<Upos> {
        let n = forms.len();
        if n == 0 {
            return Vec::new();
        }
        let t = self.t;
        let emit = self.emissions(forms);

        // Programmation dynamique de Viterbi.
        let mut dp = vec![0.0f32; n * t];
        let mut back = vec![0usize; n * t];
        dp[..t].copy_from_slice(&emit[..t]);
        for p in 1..n {
            for l in 0..t {
                let mut best = f32::NEG_INFINITY;
                let mut best_prev = 0;
                for prev in 0..t {
                    let score = dp[(p - 1) * t + prev] + self.transitions[prev * t + l];
                    if score > best {
                        best = score;
                        best_prev = prev;
                    }
                }
                dp[p * t + l] = best + emit[p * t + l];
                back[p * t + l] = best_prev;
            }
        }

        // Remontée du meilleur chemin.
        let mut last = 0;
        let mut best = f32::NEG_INFINITY;
        for l in 0..t {
            if dp[(n - 1) * t + l] > best {
                best = dp[(n - 1) * t + l];
                last = l;
            }
        }
        let mut path = vec![0usize; n];
        path[n - 1] = last;
        for p in (1..n).rev() {
            path[p - 1] = back[p * t + path[p]];
        }
        path.into_iter().map(|l| self.labels[l]).collect()
    }
}

fn load() -> Model {
    Model::parse(MODEL_BYTES).unwrap_or_else(Model::degenerate)
}

fn instance() -> &'static Model {
    static INSTANCE: OnceLock<Model> = OnceLock::new();
    INSTANCE.get_or_init(load)
}

/// Indique si un token doit recevoir une étiquette (tout sauf les blancs).
fn is_taggable(token: &Token) -> bool {
    token.kind != TokenKind::Whitespace
}

/// Frontière de phrase (mêmes terminateurs que le moteur de règles).
fn is_terminator(token: &Token) -> bool {
    token.kind == TokenKind::Punctuation
        && matches!(token.text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
}

/// Découpe les tokens en phrases, chacune représentée par les **index
/// d'origine** de ses jetons étiquetables (blancs exclus, terminateur inclus).
fn sentence_spans(tokens: &[Token]) -> Vec<Vec<usize>> {
    let mut sentences = Vec::new();
    let mut current = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if !is_taggable(token) {
            continue;
        }
        current.push(i);
        if is_terminator(token) {
            sentences.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        sentences.push(current);
    }
    sentences
}

/// Étiquette tous les tokens d'un texte.
///
/// Le vecteur renvoyé est **aligné** sur `tokens` (même longueur, même ordre) :
/// `result[i]` est l'étiquette de `tokens[i]`. Les blancs reçoivent une
/// étiquette de remplissage ([`Upos::X`]). Le décodage se fait phrase par phrase
/// pour que les transitions ne « fuient » pas d'une phrase à la suivante.
pub fn tag(tokens: &[Token]) -> Vec<Tagged> {
    let model = instance();
    let mut result = vec![Tagged::new(Upos::X); tokens.len()];
    for sentence in sentence_spans(tokens) {
        let forms: Vec<&str> = sentence.iter().map(|&i| tokens[i].text.as_str()).collect();
        for (&orig, upos) in sentence.iter().zip(model.viterbi(&forms)) {
            result[orig] = Tagged::new(upos);
        }
    }
    result
}

/// Somme des meilleurs scores Viterbi par phrase d'un texte tokenisé.
///
/// Outil d'**étiquetage contrefactuel** : pour décider entre deux graphies
/// homophones (« ou »/« où », « son »/« sont »…), on compare le score du texte
/// tel quel à celui obtenu en substituant la graphie alternative. Un écart net
/// en faveur de l'alternative signale une faute. Comme seules les phrases
/// modifiées changent de score, la différence isole la substitution.
pub fn best_score(tokens: &[Token]) -> f32 {
    let model = instance();
    sentence_spans(tokens)
        .into_iter()
        .map(|sentence| {
            let forms: Vec<&str> = sentence.iter().map(|&i| tokens[i].text.as_str()).collect();
            model.best_score(&forms)
        })
        .sum()
}

/// Sérialise un modèle CRF au format `pos.crf`.
///
/// Utilisé par `tools/train-crf` ; vit ici pour que l'écriture et la lecture
/// partagent la même définition de format. Les poids flottants sont **quantifiés
/// en `i16`** via une échelle commune (max |poids| / 32767).
///
/// - `labels` : étiquettes du modèle (indices = colonnes des matrices) ;
/// - `transitions` : `t·t` poids, indexés `prev·t + cur` ;
/// - `state_weights` : `rows·t` poids, indexés `row·t + label` ;
/// - `attr_rows` : couples (attribut, identifiant de ligne) — triés ici.
pub fn serialize_model(
    labels: &[Upos],
    transitions: &[f32],
    state_weights: &[f32],
    attr_rows: &[(String, u32)],
) -> Result<Vec<u8>, fst::Error> {
    let t = labels.len();
    assert_eq!(
        transitions.len(),
        t * t,
        "matrice de transition mal dimensionnée"
    );
    assert_eq!(
        state_weights.len() % t.max(1),
        0,
        "poids d'état mal dimensionnés"
    );
    let rows = if t == 0 { 0 } else { state_weights.len() / t };

    let scale = |w: &[f32]| -> f32 {
        let max = w.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        if max > 0.0 {
            max / 32767.0
        } else {
            1.0
        }
    };
    let state_scale = scale(state_weights);
    let trans_scale = scale(transitions);

    let quant = |x: f32, s: f32| -> i16 { (x / s).round().clamp(-32767.0, 32767.0) as i16 };

    // FST des attributs (clés triées bytewise, valeur = identifiant de ligne).
    let mut pairs: Vec<(String, u32)> = attr_rows.to_vec();
    pairs.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut builder = fst::MapBuilder::memory();
    for (attr, row) in &pairs {
        builder.insert(attr.as_bytes(), *row as u64)?;
    }
    let fst_bytes = builder.into_inner()?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(t as u8);
    for l in labels {
        out.push(l.code());
    }
    out.extend_from_slice(&state_scale.to_le_bytes());
    out.extend_from_slice(&trans_scale.to_le_bytes());
    for &x in transitions {
        out.extend_from_slice(&quant(x, trans_scale).to_le_bytes());
    }
    out.extend_from_slice(&(rows as u32).to_le_bytes());
    for &x in state_weights {
        out.extend_from_slice(&quant(x, state_scale).to_le_bytes());
    }
    out.extend_from_slice(&(fst_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&fst_bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upos_code_roundtrips() {
        for u in Upos::ALL {
            assert_eq!(Upos::from_code(u.code()), Some(u));
        }
    }

    #[test]
    fn attributes_are_deterministic() {
        let forms = ["le", "chat", "dort"];
        let mut a = Vec::new();
        let mut b = Vec::new();
        observation_attributes(&forms, 1, &mut a);
        observation_attributes(&forms, 1, &mut b);
        assert_eq!(a, b);
        assert!(a.contains(&"w[0]=chat".to_string()));
        assert!(a.contains(&"w[-1]=le".to_string()));
        assert!(a.contains(&"w[1]=dort".to_string()));
        assert!(a.contains(&"suf3[0]=hat".to_string()));
        assert!(a.contains(&"bias".to_string()));
    }

    #[test]
    fn boundary_markers_at_edges() {
        let forms = ["chat"];
        let mut a = Vec::new();
        observation_attributes(&forms, 0, &mut a);
        assert!(a.contains(&"w[-1]=<BND>".to_string()));
        assert!(a.contains(&"w[1]=<BND>".to_string()));
    }

    /// Construit un mini-modèle à la main, le sérialise puis le relit, et vérifie
    /// que le Viterbi choisit l'étiquette favorisée par les poids.
    #[test]
    fn serialize_parse_and_decode() {
        // Deux étiquettes seulement pour le test : NOUN (0) et VERB (1).
        let labels = vec![Upos::Noun, Upos::Verb];
        let t = labels.len();
        let transitions = vec![0.0f32; t * t];
        // Une seule ligne d'attribut « w[0]=chat » qui pousse fortement vers NOUN.
        let attr_rows = vec![("w[0]=chat".to_string(), 0u32)];
        let mut state_weights = vec![0.0f32; t]; // rows = 1
        state_weights[0] = 5.0; // NOUN
        state_weights[1] = -5.0; // VERB

        let bytes = serialize_model(&labels, &transitions, &state_weights, &attr_rows).unwrap();
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let model = Model::parse(leaked).expect("modèle relisible");

        assert_eq!(model.t, 2);
        let tags = model.viterbi(&["chat"]);
        assert_eq!(tags, vec![Upos::Noun]);
    }

    #[test]
    fn degenerate_model_does_not_panic() {
        let model = Model::degenerate();
        let tags = model.viterbi(&["chat", "dort"]);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn to_category_maps_aux_to_verb() {
        assert_eq!(Upos::Aux.to_category(), MorphCategory::Verb);
        assert_eq!(Upos::Propn.to_category(), MorphCategory::Noun);
        assert_eq!(Upos::Sconj.to_category(), MorphCategory::Conjunction);
    }

    // --- Tests sur le modèle réel embarqué (`assets/pos.crf`). ---

    /// Étiquettes des tokens lexicaux/ponctuation d'une phrase (blancs exclus).
    fn tags_of(text: &str) -> Vec<(String, Upos)> {
        let tokens = crate::tokenizer::tokenize(text);
        crate::tokenizer::tokenize(text)
            .iter()
            .zip(tag(&tokens))
            .filter(|(t, _)| t.kind != TokenKind::Whitespace)
            .map(|(t, g)| (t.text.clone(), g.upos))
            .collect()
    }

    fn upos_of(text: &str, word: &str) -> Upos {
        tags_of(text)
            .into_iter()
            .find(|(w, _)| w == word)
            .map(|(_, u)| u)
            .unwrap_or_else(|| panic!("« {word} » introuvable dans « {text} »"))
    }

    #[test]
    fn tags_aligned_with_tokens() {
        let tokens = crate::tokenizer::tokenize("Le chat dort.");
        assert_eq!(tag(&tokens).len(), tokens.len());
    }

    #[test]
    fn disambiguates_homographs() {
        // « ferme » (nom/adjectif/verbe) et « porte » (nom/verbe) doivent être
        // tranchés par le contexte.
        let s = "Elle ferme la porte.";
        assert_eq!(upos_of(s, "ferme"), Upos::Verb);
        assert_eq!(upos_of(s, "porte"), Upos::Noun);
        assert_eq!(upos_of(s, "la"), Upos::Det);
        assert_eq!(upos_of(s, "Elle"), Upos::Pron);
    }

    #[test]
    fn distinguishes_auxiliary_from_verb() {
        // Nécessaire à l'accord du participe passé : « sont » auxiliaire vs le
        // participe « partis » verbe plein.
        let s = "Les enfants sont partis.";
        assert_eq!(upos_of(s, "sont"), Upos::Aux);
        assert_eq!(upos_of(s, "partis"), Upos::Verb);
    }

    #[test]
    fn basic_sentence_tagging() {
        let s = "Le chat dort.";
        assert_eq!(upos_of(s, "Le"), Upos::Det);
        assert_eq!(upos_of(s, "chat"), Upos::Noun);
        assert_eq!(upos_of(s, "dort"), Upos::Verb);
    }
}
