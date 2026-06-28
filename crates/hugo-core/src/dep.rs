//! Analyse en **dépendances** (Universal Dependencies) par parser
//! *transition-based* (arc-eager) à classifieur linéaire.
//!
//! Là où [`pos`](crate::pos) attribue à chaque token **une** catégorie
//! ([`Upos`]), ce module construit l'**arbre syntaxique** de la phrase : chaque
//! token reçoit un **gouverneur** (`head`) et une **relation** ([`DepRel`]) le
//! reliant à ce gouverneur. C'est la structure qui permet aux règles de savoir
//! *quel* nom un adjectif qualifie, *quel* est le sujet d'un verbe, où commence
//! et finit une proposition — au lieu de le deviner par des heuristiques
//! linéaires.
//!
//! ## Représentation
//!
//! L'arbre est stocké **dans** [`Tagged`](crate::pos::Tagged) : `tags[i].head`
//! est l'index (dans le tableau de tokens d'origine) du gouverneur de
//! `tokens[i]`, et `tags[i].dep` la relation. Par convention, **la racine de la
//! phrase pointe sur elle-même** (`head == i`). Les tokens non analysés
//! (blancs, ou avant l'appel à [`parse`]) portent le sentinelle
//! [`HEAD_UNSET`].
//!
//! ## Algorithme
//!
//! Système de transitions **arc-eager** de Nivre : une pile, un tampon, et
//! quatre transitions (`Shift`, `Reduce`, `LeftArc(rel)`, `RightArc(rel)`). Un
//! classifieur linéaire — entraîné hors-ligne par perceptron moyenné, voir
//! `tools/train-dep` — score les transitions légales à chaque configuration ;
//! le décodage est **glouton**, O(n). C'est exactement la famille des anciens
//! modèles spaCy (transition-based, pas transformer) : léger, rapide, embarqué.
//!
//! La fonction d'extraction de traits [`transition_features`] est **partagée**
//! avec l'entraîneur : même contrat que [`pos::observation_attributes`].

use std::sync::OnceLock;

use fst::Map as FstMap;

use crate::pos::{Tagged, Upos};
use crate::tokenizer::{Token, TokenKind};

/// Sentinelle de gouverneur non assigné (token non analysé syntaxiquement).
pub const HEAD_UNSET: u32 = u32::MAX;

/// Relation de dépendance (sous-ensemble du jeu Universal Dependencies v2,
/// enrichi des sous-types décisifs pour le français : `nsubj:pass`,
/// `acl:relcl`, `obl:agent`).
///
/// Les relations absentes du jeu sont repliées sur [`DepRel::Dep`] (relation
/// générique) à la lecture du CoNLL-U.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DepRel {
    /// Racine de la phrase (`root`).
    Root,
    /// Sujet nominal (`nsubj`).
    Nsubj,
    /// Sujet d'un verbe passif (`nsubj:pass`).
    NsubjPass,
    /// Sujet clausal (`csubj`).
    Csubj,
    /// Objet direct (`obj`).
    Obj,
    /// Objet indirect (`iobj`).
    Iobj,
    /// Complément clausal (`ccomp`).
    Ccomp,
    /// Complément clausal ouvert / attribut de l'objet (`xcomp`).
    Xcomp,
    /// Oblique (`obl`).
    Obl,
    /// Complément d'agent du passif (`obl:agent`).
    OblAgent,
    /// Adverbe / modifieur adverbial (`advmod`).
    Advmod,
    /// Proposition circonstancielle (`advcl`).
    Advcl,
    /// Modifieur adjectival — épithète (`amod`).
    Amod,
    /// Modifieur nominal (`nmod`).
    Nmod,
    /// Modifieur numéral (`nummod`).
    Nummod,
    /// Apposition (`appos`).
    Appos,
    /// Proposition adjective / relative non restrictive (`acl`).
    Acl,
    /// Proposition relative (`acl:relcl`).
    AclRelcl,
    /// Déterminant (`det`).
    Det,
    /// Marqueur de cas / préposition (`case`).
    Case,
    /// Marqueur de subordination (`mark`).
    Mark,
    /// Conjonction de coordination (`cc`).
    Cc,
    /// Conjoint coordonné (`conj`).
    Conj,
    /// Copule (`cop`).
    Cop,
    /// Auxiliaire (`aux`).
    Aux,
    /// Auxiliaire passif (`aux:pass`).
    AuxPass,
    /// Explétif (`expl`).
    Expl,
    /// Expression figée (`fixed`).
    Fixed,
    /// Construction plate / nom composé (`flat`).
    Flat,
    /// Parataxe (`parataxis`).
    Parataxis,
    /// Ponctuation (`punct`).
    Punct,
    /// Vocatif (`vocative`).
    Vocative,
    /// Élément disloqué (`dislocated`).
    Dislocated,
    /// Relation générique / non couverte (`dep` et repli).
    Dep,
}

impl DepRel {
    /// Toutes les relations, dans l'ordre de leur code de sérialisation.
    pub const ALL: [DepRel; 33] = [
        DepRel::Root,
        DepRel::Nsubj,
        DepRel::NsubjPass,
        DepRel::Csubj,
        DepRel::Obj,
        DepRel::Iobj,
        DepRel::Ccomp,
        DepRel::Xcomp,
        DepRel::Obl,
        DepRel::OblAgent,
        DepRel::Advmod,
        DepRel::Advcl,
        DepRel::Amod,
        DepRel::Nmod,
        DepRel::Nummod,
        DepRel::Appos,
        DepRel::Acl,
        DepRel::AclRelcl,
        DepRel::Det,
        DepRel::Case,
        DepRel::Mark,
        DepRel::Cc,
        DepRel::Conj,
        DepRel::Cop,
        DepRel::Aux,
        DepRel::AuxPass,
        DepRel::Expl,
        DepRel::Fixed,
        DepRel::Flat,
        DepRel::Parataxis,
        DepRel::Punct,
        DepRel::Vocative,
        DepRel::Dislocated,
    ];

    /// Code compact (0..32) pour la sérialisation et l'indexation des classes.
    pub fn code(self) -> u8 {
        // `Dep` n'est pas dans ALL : c'est le repli, codé en dernier.
        DepRel::ALL
            .iter()
            .position(|&r| r == self)
            .unwrap_or(DepRel::ALL.len()) as u8
    }

    /// Relation correspondant à un code compact (`Dep` hors plage).
    pub fn from_code(code: u8) -> DepRel {
        DepRel::ALL
            .get(code as usize)
            .copied()
            .unwrap_or(DepRel::Dep)
    }

    /// Convertit une étiquette DEPREL CoNLL-U (« nsubj », « acl:relcl »…).
    /// Le sous-type est conservé pour les relations clés, sinon réduit au type
    /// principal ; une relation inconnue tombe sur [`DepRel::Dep`].
    pub fn from_conllu(tag: &str) -> DepRel {
        match tag {
            "root" => DepRel::Root,
            "nsubj" => DepRel::Nsubj,
            "nsubj:pass" => DepRel::NsubjPass,
            "csubj" | "csubj:pass" => DepRel::Csubj,
            "obj" => DepRel::Obj,
            "iobj" => DepRel::Iobj,
            "ccomp" => DepRel::Ccomp,
            "xcomp" => DepRel::Xcomp,
            "obl:agent" => DepRel::OblAgent,
            "advmod" | "advmod:emph" => DepRel::Advmod,
            "advcl" | "advcl:cleft" => DepRel::Advcl,
            "amod" => DepRel::Amod,
            "nummod" => DepRel::Nummod,
            "appos" => DepRel::Appos,
            "acl:relcl" => DepRel::AclRelcl,
            "acl" => DepRel::Acl,
            "det" => DepRel::Det,
            "case" => DepRel::Case,
            "mark" => DepRel::Mark,
            "cc" | "cc:preconj" => DepRel::Cc,
            "conj" => DepRel::Conj,
            "cop" => DepRel::Cop,
            "aux" | "aux:tense" | "aux:caus" => DepRel::Aux,
            "aux:pass" => DepRel::AuxPass,
            "cop:expl" => DepRel::Cop,
            "fixed" => DepRel::Fixed,
            "flat" | "flat:name" | "flat:foreign" => DepRel::Flat,
            "parataxis" => DepRel::Parataxis,
            "punct" => DepRel::Punct,
            "vocative" => DepRel::Vocative,
            "dislocated" => DepRel::Dislocated,
            _ if tag.starts_with("nmod") => DepRel::Nmod,
            _ if tag.starts_with("obl") => DepRel::Obl,
            _ if tag.starts_with("expl") => DepRel::Expl,
            _ => DepRel::Dep,
        }
    }

    /// Nom court de la relation (débogage, messages).
    pub fn as_str(self) -> &'static str {
        match self {
            DepRel::Root => "root",
            DepRel::Nsubj => "nsubj",
            DepRel::NsubjPass => "nsubj:pass",
            DepRel::Csubj => "csubj",
            DepRel::Obj => "obj",
            DepRel::Iobj => "iobj",
            DepRel::Ccomp => "ccomp",
            DepRel::Xcomp => "xcomp",
            DepRel::Obl => "obl",
            DepRel::OblAgent => "obl:agent",
            DepRel::Advmod => "advmod",
            DepRel::Advcl => "advcl",
            DepRel::Amod => "amod",
            DepRel::Nmod => "nmod",
            DepRel::Nummod => "nummod",
            DepRel::Appos => "appos",
            DepRel::Acl => "acl",
            DepRel::AclRelcl => "acl:relcl",
            DepRel::Det => "det",
            DepRel::Case => "case",
            DepRel::Mark => "mark",
            DepRel::Cc => "cc",
            DepRel::Conj => "conj",
            DepRel::Cop => "cop",
            DepRel::Aux => "aux",
            DepRel::AuxPass => "aux:pass",
            DepRel::Expl => "expl",
            DepRel::Fixed => "fixed",
            DepRel::Flat => "flat",
            DepRel::Parataxis => "parataxis",
            DepRel::Punct => "punct",
            DepRel::Vocative => "vocative",
            DepRel::Dislocated => "dislocated",
            DepRel::Dep => "dep",
        }
    }
}

/// Nombre de codes de relation distincts servant à indexer les classes de
/// transitions étiquetées : les [`DepRel::ALL`] (codes `0..33`) **plus** le
/// repli `Dep` (code `33`). Soit `ALL.len() + 1`. Indispensable pour que
/// `LeftArc`/`RightArc` ne se chevauchent pas dans l'espace des classes.
const NREL: usize = DepRel::ALL.len() + 1; // 34

// --- Système de transitions arc-eager ----------------------------------------

/// Une transition du parser arc-eager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Empile le premier élément du tampon.
    Shift,
    /// Dépile le sommet de pile (déjà gouverné).
    Reduce,
    /// Arc gauche étiqueté : `b0 → s0`, puis dépile `s0`.
    LeftArc(DepRel),
    /// Arc droit étiqueté : `s0 → b0`, puis empile `b0`.
    RightArc(DepRel),
}

/// Nombre total de classes de transitions : `Shift`, `Reduce`, puis
/// `LeftArc(rel)` et `RightArc(rel)` pour chaque relation.
pub const NMOVE: usize = 2 + 2 * NREL;

impl Move {
    /// Index de classe compact (0..[`NMOVE`]).
    pub fn class(self) -> usize {
        match self {
            Move::Shift => 0,
            Move::Reduce => 1,
            Move::LeftArc(r) => 2 + r.code() as usize,
            Move::RightArc(r) => 2 + NREL + r.code() as usize,
        }
    }

    /// Transition correspondant à un index de classe.
    pub fn from_class(c: usize) -> Move {
        match c {
            0 => Move::Shift,
            1 => Move::Reduce,
            _ if c < 2 + NREL => Move::LeftArc(DepRel::from_code((c - 2) as u8)),
            _ => Move::RightArc(DepRel::from_code((c - 2 - NREL) as u8)),
        }
    }
}

/// Configuration du parser : pile, tampon, arcs partiels. Partagée entre le
/// décodeur et l'entraîneur (l'oracle rejoue les transitions de référence).
///
/// L'identifiant `ROOT` virtuel vaut `n` (un au-delà du dernier token réel).
/// `forms` et `upos` ne couvrent que les `n` tokens réels.
pub struct State<'a> {
    /// Formes des tokens réels (telles quelles ; mises en minuscule au besoin).
    pub forms: &'a [&'a str],
    /// Étiquettes POS des tokens réels.
    pub upos: &'a [Upos],
    /// Nombre de tokens réels.
    pub n: usize,
    /// Pile d'indices (peut contenir `ROOT == n`).
    pub stack: Vec<usize>,
    /// Position de tête du tampon dans `0..=n`.
    pub buffer: usize,
    /// Gouverneur assigné à chaque token réel (`ROOT` ou index), ou `NONE`.
    pub heads: Vec<usize>,
    /// Relation assignée à chaque token réel.
    pub deps: Vec<DepRel>,
    /// Enfant le plus à gauche / à droite de chaque nœud (traits structurels).
    pub leftmost: Vec<usize>,
    pub rightmost: Vec<usize>,
}

/// Marqueur « pas de tête / pas d'enfant ».
const NONE: usize = usize::MAX;

impl<'a> State<'a> {
    /// Configuration initiale : pile = [ROOT], tampon = tous les tokens.
    pub fn new(forms: &'a [&'a str], upos: &'a [Upos]) -> Self {
        let n = forms.len();
        State {
            forms,
            upos,
            n,
            stack: vec![n], // ROOT
            buffer: 0,
            heads: vec![NONE; n],
            deps: vec![DepRel::Dep; n],
            leftmost: vec![NONE; n + 1],
            rightmost: vec![NONE; n + 1],
        }
    }

    /// L'analyse est terminée quand le tampon est vide.
    pub fn is_final(&self) -> bool {
        self.buffer >= self.n
    }

    fn s0(&self) -> usize {
        *self.stack.last().unwrap_or(&NONE)
    }

    /// Vrai si le token réel `i` a déjà reçu un gouverneur. Public pour que
    /// l'oracle de `tools/train-dep` puisse interroger l'état partiel.
    pub fn has_head(&self, i: usize) -> bool {
        i < self.n && self.heads[i] != NONE
    }

    /// Vrai si la transition est applicable dans la configuration courante.
    pub fn is_legal(&self, m: Move) -> bool {
        let buffer_ok = self.buffer < self.n;
        let s0 = self.s0();
        match m {
            Move::Shift => buffer_ok,
            // On ne peut pas réduire ROOT ; on ne réduit qu'un nœud déjà gouverné.
            Move::Reduce => s0 != NONE && s0 != self.n && self.has_head(s0),
            // Arc gauche : s0 réel, sans tête, et tampon non vide.
            Move::LeftArc(_) => buffer_ok && s0 != NONE && s0 != self.n && !self.has_head(s0),
            // Arc droit : tampon non vide (s0 peut être ROOT, donnant la racine).
            Move::RightArc(_) => buffer_ok && s0 != NONE,
        }
    }

    fn set_child(&mut self, head: usize, child: usize) {
        if self.leftmost[head] == NONE || child < self.leftmost[head] {
            self.leftmost[head] = child;
        }
        if self.rightmost[head] == NONE || child > self.rightmost[head] {
            self.rightmost[head] = child;
        }
    }

    /// Applique une transition (supposée légale).
    pub fn apply(&mut self, m: Move) {
        match m {
            Move::Shift => {
                self.stack.push(self.buffer);
                self.buffer += 1;
            }
            Move::Reduce => {
                self.stack.pop();
            }
            Move::LeftArc(rel) => {
                let s0 = self.stack.pop().unwrap();
                let b0 = self.buffer;
                self.heads[s0] = b0;
                self.deps[s0] = rel;
                self.set_child(b0, s0);
            }
            Move::RightArc(rel) => {
                let s0 = self.s0();
                let b0 = self.buffer;
                self.heads[b0] = s0;
                self.deps[b0] = rel;
                if s0 != self.n {
                    self.set_child(s0, b0);
                }
                self.stack.push(b0);
                self.buffer += 1;
            }
        }
    }
}

// --- Extraction de traits (partagée entraîneur/décodeur) ----------------------

#[inline]
fn word_at<'a>(state: &State<'a>, id: usize) -> &'a str {
    if id == NONE {
        "<NULL>"
    } else if id == state.n {
        "<ROOT>"
    } else {
        state.forms[id]
    }
}

#[inline]
fn pos_at(state: &State, id: usize) -> &'static str {
    if id == NONE {
        "<NULL>"
    } else if id == state.n {
        "<ROOT>"
    } else {
        state.upos[id].as_str()
    }
}

#[inline]
fn dep_at(state: &State, id: usize) -> &'static str {
    if id == NONE || id == state.n {
        "<NULL>"
    } else {
        state.deps[id].as_str()
    }
}

/// Produit les **traits** de la configuration `state`, en les ajoutant à `out`
/// (vidé au préalable). Inspiré des gabarits Zhang & Nivre (2011) : formes et
/// POS des sommets de pile/tampon, de leurs enfants extrêmes, et leurs paires.
///
/// **Contrat de cohérence entraînement/décodage** : appelée à l'identique par
/// `tools/train-dep` et par le décodeur.
pub fn transition_features(state: &State, out: &mut Vec<String>) {
    out.clear();
    out.push("bias".to_string());

    let n = state.stack.len();
    let s0 = if n >= 1 { state.stack[n - 1] } else { NONE };
    let s1 = if n >= 2 { state.stack[n - 2] } else { NONE };
    let b0 = if state.buffer < state.n { state.buffer } else { NONE };
    let b1 = if state.buffer + 1 < state.n { state.buffer + 1 } else { NONE };
    let b2 = if state.buffer + 2 < state.n { state.buffer + 2 } else { NONE };

    let s0l = if s0 < state.n { state.leftmost[s0] } else { NONE };
    let s0r = if s0 < state.n { state.rightmost[s0] } else { NONE };
    let b0l = if b0 != NONE { state.leftmost[b0] } else { NONE };

    let lw = |id| word_at(state, id).to_lowercase();
    let p = |id| pos_at(state, id);

    // Unigrammes : mot, POS, mot⊕POS pour les positions clés.
    for (tag, id) in [("s0", s0), ("s1", s1), ("b0", b0), ("b1", b1), ("b2", b2)] {
        out.push(format!("{tag}.w={}", lw(id)));
        out.push(format!("{tag}.p={}", p(id)));
        out.push(format!("{tag}.wp={}|{}", lw(id), p(id)));
    }

    // Enfants extrêmes : POS et relation.
    for (tag, id) in [("s0l", s0l), ("s0r", s0r), ("b0l", b0l)] {
        out.push(format!("{tag}.p={}", p(id)));
        out.push(format!("{tag}.d={}", dep_at(state, id)));
    }

    // Bigrammes structurels : POS de paires adjacentes du noyau.
    out.push(format!("s0p.b0p={}|{}", p(s0), p(b0)));
    out.push(format!("s0w.b0w={}|{}", lw(s0), lw(b0)));
    out.push(format!("s0p.b0p.b1p={}|{}|{}", p(s0), p(b0), p(b1)));
    out.push(format!("s1p.s0p.b0p={}|{}|{}", p(s1), p(s0), p(b0)));
    out.push(format!("s0w.b0p={}|{}", lw(s0), p(b0)));
    out.push(format!("s0p.b0w={}|{}", p(s0), lw(b0)));

    // Distance s0–b0 (par paliers) : utile pour l'attachement.
    if s0 != NONE && s0 != state.n && b0 != NONE {
        let dist = b0.saturating_sub(s0);
        let bucket = match dist {
            0 => "0",
            1 => "1",
            2 => "2",
            3..=5 => "3-5",
            _ => "6+",
        };
        out.push(format!("dist={bucket}|{}|{}", p(s0), p(b0)));
    }
}

// --- Format binaire du modèle (`assets/pos.dep`) ------------------------------
//
// **Stockage creux** : pour chaque ligne d'attribut, seuls les poids de classe
// **non nuls** sont stockés (la grande majorité des couples (trait, transition)
// ne sont jamais activés). Indispensable à la légèreté de l'asset embarqué.
//
// Disposition (little-endian) :
//   magic        : 8 octets (= MAGIC)
//   m            : u16                       nombre de classes de transitions (= NMOVE)
//   scale        : f32                       échelle des poids quantifiés
//   rows         : u32                       nombre de lignes d'attributs
//   offsets      : (rows+1) × u32            bornes des entrées par ligne (unité = entrée)
//   entries      : total × (u8 classe, i16 poids)   poids non nuls, 3 octets chacun
//   fst_len      : u32                       taille de la FST d'attributs
//   fst          : fst_len octets            fst::Map<attribut → row_id (u64)>

const MAGIC: &[u8; 8] = b"HUGODEP\x02";

/// Taille d'une entrée creuse en octets : `u8` classe + `i16` poids.
const ENTRY: usize = 3;

static MODEL_BYTES: &[u8] = include_bytes!("../assets/pos.dep");

struct Model {
    m: usize,
    scale: f32,
    /// Bornes d'entrées par ligne, `(rows+1)` entiers `u32` little-endian.
    offsets: &'static [u8],
    /// Entrées creuses : `total × (u8 classe, i16 poids)`.
    entries: &'static [u8],
    attr: Option<FstMap<&'static [u8]>>,
}

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
        let s = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u16(&mut self) -> Option<u16> {
        let b = self.take(2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
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
fn read_u32_at(raw: &[u8], idx: usize) -> usize {
    let b = idx * 4;
    u32::from_le_bytes([raw[b], raw[b + 1], raw[b + 2], raw[b + 3]]) as usize
}

impl Model {
    fn degenerate() -> Self {
        Model {
            m: NMOVE,
            scale: 1.0,
            offsets: &[],
            entries: &[],
            attr: None,
        }
    }

    fn parse(bytes: &'static [u8]) -> Option<Model> {
        if bytes.len() < MAGIC.len() {
            return None;
        }
        let mut cur = Cursor::new(bytes);
        if cur.take(MAGIC.len())? != MAGIC {
            return None;
        }
        let m = cur.u16()? as usize;
        let scale = cur.f32()?;
        let rows = cur.u32()? as usize;
        let offsets = cur.take((rows + 1) * 4)?;
        let total = read_u32_at(offsets, rows);
        let entries = cur.take(total * ENTRY)?;
        let fst_len = cur.u32()? as usize;
        let fst_bytes = cur.take(fst_len)?;
        let attr = FstMap::new(fst_bytes).ok()?;
        Some(Model {
            m,
            scale,
            offsets,
            entries,
            attr: Some(attr),
        })
    }

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

    /// Scores des [`NMOVE`] classes pour la configuration courante, en sommant
    /// les entrées creuses non nulles des lignes d'attributs actives.
    fn scores(&self, attrs: &[String]) -> Vec<f32> {
        let mut s = vec![0.0f32; self.m];
        if self.offsets.is_empty() {
            return s;
        }
        for &row in &self.active_rows(attrs) {
            let start = read_u32_at(self.offsets, row);
            let end = read_u32_at(self.offsets, row + 1);
            for e in start..end {
                let off = e * ENTRY;
                let class = self.entries[off] as usize;
                let w = i16::from_le_bytes([self.entries[off + 1], self.entries[off + 2]]);
                s[class] += w as f32 * self.scale;
            }
        }
        s
    }
}

fn load() -> Model {
    Model::parse(MODEL_BYTES).unwrap_or_else(Model::degenerate)
}

fn instance() -> &'static Model {
    static INSTANCE: OnceLock<Model> = OnceLock::new();
    INSTANCE.get_or_init(load)
}

// --- Décodage glouton ---------------------------------------------------------

/// Priorité de repli déterministe entre transitions de score égal (modèle
/// dégénéré non entraîné) : favorise un arbre projectif droit raisonnable.
fn fallback_priority(m: Move) -> i32 {
    match m {
        Move::RightArc(_) => 3,
        Move::Shift => 2,
        Move::LeftArc(_) => 1,
        Move::Reduce => 0,
    }
}

/// Analyse une phrase (formes + POS) et renvoie, pour chaque token réel, le
/// couple (gouverneur, relation). Le gouverneur est un index dans `0..n`, ou
/// `n` pour la racine (rattachée à ROOT).
fn parse_sentence(forms: &[&str], upos: &[Upos]) -> Vec<(usize, DepRel)> {
    let n = forms.len();
    if n == 0 {
        return Vec::new();
    }
    let model = instance();
    let mut state = State::new(forms, upos);
    let mut attrs: Vec<String> = Vec::new();

    // Garde-fou : borne le nombre de transitions (2n suffit en arc-eager).
    let max_steps = 2 * n + 2;
    let mut steps = 0;
    while !state.is_final() && steps < max_steps {
        steps += 1;
        transition_features(&state, &mut attrs);
        let scores = model.scores(&attrs);

        // Choisit la transition légale de score maximal (départage par priorité
        // de repli, ce qui rend le modèle dégénéré déterministe et sûr).
        let mut best: Option<(Move, f32, i32)> = None;
        for c in 0..NMOVE {
            let mv = Move::from_class(c);
            if !state.is_legal(mv) {
                continue;
            }
            let score = scores.get(c).copied().unwrap_or(0.0);
            let prio = fallback_priority(mv);
            let better = match best {
                None => true,
                Some((_, bs, bp)) => score > bs || (score == bs && prio > bp),
            };
            if better {
                best = Some((mv, score, prio));
            }
        }

        match best {
            Some((mv, _, _)) => state.apply(mv),
            // Aucune transition légale (ne devrait pas arriver) : on force un
            // Shift si possible, sinon on s'arrête.
            None => {
                if state.is_legal(Move::Shift) {
                    state.apply(Move::Shift);
                } else {
                    break;
                }
            }
        }
    }

    // Rattache les tokens restés sans tête à ROOT (robustesse).
    (0..n)
        .map(|i| {
            let h = state.heads[i];
            if h == NONE {
                (n, DepRel::Root)
            } else {
                (h, state.deps[i])
            }
        })
        .collect()
}

// --- Découpage en phrases (identique au POS) ----------------------------------

fn is_taggable(token: &Token) -> bool {
    token.kind != TokenKind::Whitespace
}

fn is_terminator(token: &Token) -> bool {
    token.kind == TokenKind::Punctuation
        && matches!(token.text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
}

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

/// Analyse en dépendances un texte déjà étiqueté en POS, et **renseigne**
/// `tags[i].head` (index de token d'origine du gouverneur ; `i` lui-même pour la
/// racine) et `tags[i].dep` (relation).
///
/// `tags` doit être aligné sur `tokens` (sortie de [`pos::tag`]). Les tokens
/// blancs gardent leur sentinelle [`HEAD_UNSET`].
pub fn parse(tokens: &[Token], tags: &mut [Tagged]) {
    for sentence in sentence_spans(tokens) {
        let forms: Vec<&str> = sentence.iter().map(|&i| tokens[i].text.as_str()).collect();
        let upos: Vec<Upos> = sentence.iter().map(|&i| tags[i].upos).collect();
        let arcs = parse_sentence(&forms, &upos);
        for (local, (head_local, rel)) in arcs.into_iter().enumerate() {
            let orig = sentence[local];
            // ROOT (head_local == n) → la racine pointe sur elle-même.
            let head_orig = if head_local >= sentence.len() {
                orig
            } else {
                sentence[head_local]
            };
            tags[orig].head = head_orig as u32;
            tags[orig].dep = rel;
        }
    }
}

// --- API de requêtes sur l'arbre (consommée par les règles) -------------------
//
// Toutes ces fonctions raisonnent dans l'**espace des index de tokens** (le même
// que celui manipulé par les règles) : `tags` est aligné sur `tokens`, et un
// `head` est un index de token. La racine de la phrase vérifie `head == i`.

/// Index du gouverneur syntaxique de `tokens[i]`, ou `None` si `i` est la racine
/// de sa phrase ou n'a pas été analysé.
pub fn head_of(tags: &[Tagged], i: usize) -> Option<usize> {
    let h = tags[i].head;
    if h == HEAD_UNSET || h as usize == i {
        None
    } else {
        Some(h as usize)
    }
}

/// Vrai si `tokens[i]` est la racine de sa phrase (`head == i`).
pub fn is_root(tags: &[Tagged], i: usize) -> bool {
    tags[i].head != HEAD_UNSET && tags[i].head as usize == i
}

/// Relation de dépendance reliant `tokens[i]` à son gouverneur.
pub fn deprel(tags: &[Tagged], i: usize) -> DepRel {
    tags[i].dep
}

/// Enfants directs de `tokens[i]` (index de tokens, ordre croissant).
pub fn children(tags: &[Tagged], i: usize) -> Vec<usize> {
    (0..tags.len())
        .filter(|&j| j != i && tags[j].head != HEAD_UNSET && tags[j].head as usize == i)
        .collect()
}

/// Premier enfant de `tokens[i]` dont la relation figure dans `rels`.
pub fn child_with(tags: &[Tagged], i: usize, rels: &[DepRel]) -> Option<usize> {
    children(tags, i)
        .into_iter()
        .find(|&c| rels.contains(&tags[c].dep))
}

/// Sujet du verbe (ou auxiliaire/copule) `v` : enfant `nsubj`, `nsubj:pass` ou
/// `csubj`. Si `v` est porté par un auxiliaire ou une copule, le sujet est en
/// pratique attaché à la tête lexicale ; on cherche donc aussi chez la tête.
pub fn subject_of(tags: &[Tagged], v: usize) -> Option<usize> {
    const SUBJ: &[DepRel] = &[DepRel::Nsubj, DepRel::NsubjPass, DepRel::Csubj];
    child_with(tags, v, SUBJ)
}

/// Objet direct du verbe `v` : enfant `obj`.
pub fn object_of(tags: &[Tagged], v: usize) -> Option<usize> {
    child_with(tags, v, &[DepRel::Obj])
}

/// Si `tokens[i]` est un adjectif épithète (`amod`), le nom qu'il qualifie
/// (son gouverneur).
pub fn modified_noun(tags: &[Tagged], i: usize) -> Option<usize> {
    if tags[i].dep == DepRel::Amod {
        head_of(tags, i)
    } else {
        None
    }
}

/// Vrai s'il existe un ancêtre de `tokens[i]` (en remontant les `head`) relié
/// par `acl:relcl` — c.-à-d. si `i` est à l'intérieur d'une proposition
/// relative. Borné par la profondeur de l'arbre (anti-cycle).
pub fn in_relative_clause(tags: &[Tagged], i: usize) -> bool {
    let mut cur = i;
    for _ in 0..tags.len() {
        if tags[cur].dep == DepRel::AclRelcl {
            return true;
        }
        match head_of(tags, cur) {
            Some(h) => cur = h,
            None => break,
        }
    }
    false
}

/// Remonte jusqu'à la tête verbale gouvernant `tokens[i]` (saute déterminants,
/// adjectifs… en suivant `head`). Renvoie le premier ancêtre étiqueté
/// `Verb`/`Aux` selon `tags`, ou `None`.
pub fn governing_verb(tags: &[Tagged], i: usize) -> Option<usize> {
    let mut cur = head_of(tags, i)?;
    for _ in 0..tags.len() {
        if matches!(tags[cur].upos, Upos::Verb | Upos::Aux) {
            return Some(cur);
        }
        cur = head_of(tags, cur)?;
    }
    None
}

/// Sérialise un modèle de parser au format `pos.dep`.
///
/// Utilisé par `tools/train-dep` ; vit ici pour partager la définition du format
/// avec le lecteur runtime. Les poids sont quantifiés en `i16` via une échelle
/// commune (max |poids| / 32767).
///
/// - `weights` : `rows·NMOVE` poids, indexés `row·NMOVE + class` ;
/// - `attr_rows` : couples (attribut, identifiant de ligne).
pub fn serialize_model(weights: &[f32], attr_rows: &[(String, u32)]) -> Result<Vec<u8>, fst::Error> {
    let m = NMOVE;
    assert_eq!(weights.len() % m, 0, "poids mal dimensionnés");
    let rows = weights.len() / m;

    let max = weights.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
    let scale = if max > 0.0 { max / 32767.0 } else { 1.0 };
    let quant = |x: f32| -> i16 { (x / scale).round().clamp(-32767.0, 32767.0) as i16 };

    // Entrées creuses : par ligne, on ne garde que les classes de poids non nul.
    let mut offsets: Vec<u32> = Vec::with_capacity(rows + 1);
    let mut entries: Vec<u8> = Vec::new();
    let mut count: u32 = 0;
    for row in 0..rows {
        offsets.push(count);
        for class in 0..m {
            let q = quant(weights[row * m + class]);
            if q != 0 {
                entries.push(class as u8);
                entries.extend_from_slice(&q.to_le_bytes());
                count += 1;
            }
        }
    }
    offsets.push(count);

    let mut pairs: Vec<(String, u32)> = attr_rows.to_vec();
    pairs.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut builder = fst::MapBuilder::memory();
    for (attr, row) in &pairs {
        builder.insert(attr.as_bytes(), *row as u64)?;
    }
    let fst_bytes = builder.into_inner()?;

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(m as u16).to_le_bytes());
    out.extend_from_slice(&scale.to_le_bytes());
    out.extend_from_slice(&(rows as u32).to_le_bytes());
    for &o in &offsets {
        out.extend_from_slice(&o.to_le_bytes());
    }
    out.extend_from_slice(&entries);
    out.extend_from_slice(&(fst_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&fst_bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    /// Tout token réel a exactement une tête, et il existe au moins une racine.
    #[test]
    fn produces_valid_tree() {
        let tokens = tokenize("Le chat dort sur le tapis.");
        let mut tags = crate::pos::tag(&tokens);
        parse(&tokens, &mut tags);
        let mut roots = 0;
        for (i, t) in tokens.iter().enumerate() {
            if t.kind == TokenKind::Whitespace {
                continue;
            }
            assert_ne!(tags[i].head, HEAD_UNSET, "token « {} » sans tête", t.text);
            if tags[i].head as usize == i {
                roots += 1;
            }
        }
        assert!(roots >= 1, "au moins une racine attendue");
    }

    #[test]
    fn heads_point_within_bounds() {
        let tokens = tokenize("Les enfants mangent une pomme.");
        let mut tags = crate::pos::tag(&tokens);
        parse(&tokens, &mut tags);
        for (i, t) in tokens.iter().enumerate() {
            if t.kind == TokenKind::Whitespace {
                continue;
            }
            assert!((tags[i].head as usize) < tokens.len());
        }
    }

    #[test]
    fn move_class_roundtrips() {
        for c in 0..NMOVE {
            assert_eq!(Move::from_class(c).class(), c);
        }
    }

    #[test]
    fn deprel_code_roundtrips() {
        for r in DepRel::ALL {
            assert_eq!(DepRel::from_code(r.code()), r);
        }
        // `Dep` est le repli hors table.
        assert_eq!(DepRel::Dep.code() as usize, DepRel::ALL.len());
        assert_eq!(DepRel::from_code(DepRel::ALL.len() as u8), DepRel::Dep);
    }

    #[test]
    fn arc_eager_terminates_on_short_input() {
        let tokens = tokenize("Il dort.");
        let mut tags = crate::pos::tag(&tokens);
        parse(&tokens, &mut tags);
        // Pas de panique, et alignement conservé.
        assert_eq!(tags.len(), tokens.len());
    }

    #[test]
    fn empty_and_single_token() {
        assert!(parse_sentence(&[], &[]).is_empty());
        let arcs = parse_sentence(&["mot"], &[Upos::Noun]);
        assert_eq!(arcs.len(), 1);
        // Seul token → racine.
        assert_eq!(arcs[0].0, 1);
    }
}
