//! Analyse morphologique.
//!
//! Ce module définit les traits morphologiques d'une forme fléchie (catégorie,
//! genre, nombre, personne, lemme) et fournit [`lookup`], qui interroge le
//! lexique compilé depuis Lexique383 (voir `tools/compile-morpho`) et
//! **embarqué** dans la bibliothèque.
//!
//! Le lexique est chargé paresseusement, une seule fois, au premier appel.

use std::collections::HashMap;
use std::sync::OnceLock;

use fst::Streamer;

/// Catégorie grammaticale (partie du discours).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MorphCategory {
    /// Nom commun.
    Noun,
    /// Verbe (y compris auxiliaires).
    Verb,
    /// Adjectif qualificatif.
    Adjective,
    /// Déterminant (article, possessif, démonstratif, numéral…).
    Determiner,
    /// Pronom.
    Pronoun,
    /// Adverbe.
    Adverb,
    /// Préposition.
    Preposition,
    /// Conjonction.
    Conjunction,
    /// Interjection / onomatopée.
    Interjection,
    /// Catégorie inconnue ou non déterminée.
    Unknown,
}

/// Genre grammatical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Gender {
    /// Masculin.
    Masculine,
    /// Féminin.
    Feminine,
    /// Épicène / invariable en genre.
    Epicene,
}

/// Nombre grammatical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Number {
    /// Singulier.
    Singular,
    /// Pluriel.
    Plural,
    /// Invariable en nombre.
    Invariable,
}

/// Personne grammaticale (pour les verbes et pronoms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Person {
    /// Première personne.
    First,
    /// Deuxième personne.
    Second,
    /// Troisième personne.
    Third,
}

/// Mode et temps d'une forme verbale finie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MoodTense {
    /// Indicatif présent.
    IndicativePresent,
    /// Indicatif imparfait.
    IndicativeImperfect,
    /// Indicatif futur simple.
    IndicativeFuture,
    /// Indicatif passé simple.
    IndicativePast,
    /// Conditionnel présent.
    ConditionalPresent,
    /// Subjonctif présent.
    SubjunctivePresent,
    /// Subjonctif imparfait.
    SubjunctiveImperfect,
    /// Impératif présent.
    Imperative,
}

impl MoodTense {
    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => MoodTense::IndicativePresent,
            2 => MoodTense::IndicativeImperfect,
            3 => MoodTense::IndicativeFuture,
            4 => MoodTense::IndicativePast,
            5 => MoodTense::ConditionalPresent,
            6 => MoodTense::SubjunctivePresent,
            7 => MoodTense::SubjunctiveImperfect,
            8 => MoodTense::Imperative,
            _ => return None,
        })
    }

    fn code(self) -> u8 {
        match self {
            MoodTense::IndicativePresent => 1,
            MoodTense::IndicativeImperfect => 2,
            MoodTense::IndicativeFuture => 3,
            MoodTense::IndicativePast => 4,
            MoodTense::ConditionalPresent => 5,
            MoodTense::SubjunctivePresent => 6,
            MoodTense::SubjunctiveImperfect => 7,
            MoodTense::Imperative => 8,
        }
    }
}

/// Une forme verbale finie : son lemme, son mode/temps, sa personne et son
/// nombre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbForm {
    /// Lemme (infinitif).
    pub lemma: String,
    /// Mode et temps.
    pub mood_tense: MoodTense,
    /// Personne.
    pub person: Person,
    /// Nombre.
    pub number: Number,
}

/// Une analyse morphologique possible d'une forme fléchie.
///
/// Une même forme peut admettre plusieurs `Morph` (ambiguïté), d'où le
/// `Vec<Morph>` renvoyé par [`lookup`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Morph {
    /// Lemme (forme canonique).
    pub lemma: String,
    /// Catégorie grammaticale.
    pub category: MorphCategory,
    /// Genre, si pertinent.
    pub gender: Option<Gender>,
    /// Nombre, si pertinent.
    pub number: Option<Number>,
    /// Personne, si pertinente (verbes, pronoms).
    pub person: Option<Person>,
}

impl Morph {
    /// Construit une analyse minimale (catégorie + lemme), sans traits.
    pub fn new(lemma: impl Into<String>, category: MorphCategory) -> Self {
        Morph {
            lemma: lemma.into(),
            category,
            gender: None,
            number: None,
            person: None,
        }
    }
}

// --- Codes de sérialisation, partagés avec `tools/compile-morpho`. ---

fn decode_category(code: u8) -> MorphCategory {
    match code {
        1 => MorphCategory::Noun,
        2 => MorphCategory::Verb,
        3 => MorphCategory::Adjective,
        4 => MorphCategory::Determiner,
        5 => MorphCategory::Pronoun,
        6 => MorphCategory::Adverb,
        7 => MorphCategory::Preposition,
        8 => MorphCategory::Conjunction,
        9 => MorphCategory::Interjection,
        _ => MorphCategory::Unknown,
    }
}

fn decode_gender(code: u8) -> Option<Gender> {
    match code {
        1 => Some(Gender::Masculine),
        2 => Some(Gender::Feminine),
        _ => None,
    }
}

fn decode_number(code: u8) -> Option<Number> {
    match code {
        1 => Some(Number::Singular),
        2 => Some(Number::Plural),
        _ => None,
    }
}

fn decode_person(code: u8) -> Option<Person> {
    match code {
        1 => Some(Person::First),
        2 => Some(Person::Second),
        3 => Some(Person::Third),
        _ => None,
    }
}

/// FST des formes (clé minuscule → `(offset << 8) | count`) et blob d'analyses,
/// embarqués à la compilation.
static MORPHO_FST: &[u8] = include_bytes!("../assets/morpho.fst");
static MORPHO_BIN: &[u8] = include_bytes!("../assets/morpho.bin");
static MORPHO_FREQ: &[u8] = include_bytes!("../assets/morpho.freq.fst");

/// Un enregistrement brut décodé depuis le blob.
struct Raw {
    cat: u8,
    gender: u8,
    number: u8,
    person: u8,
    mt: u8,
    lemma: String,
}

/// Décode les `count` enregistrements situés à `offset` dans `blob`.
///
/// Format d'un enregistrement : `[cat][genre][nombre][personne][mt][len][lemme]`.
fn decode_records(blob: &[u8], mut offset: usize, count: usize) -> Vec<Raw> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 6 > blob.len() {
            break;
        }
        let cat = blob[offset];
        let gender = blob[offset + 1];
        let number = blob[offset + 2];
        let person = blob[offset + 3];
        let mt = blob[offset + 4];
        let len = blob[offset + 5] as usize;
        offset += 6;
        if offset + len > blob.len() {
            break;
        }
        let lemma = std::str::from_utf8(&blob[offset..offset + len])
            .unwrap_or("")
            .to_string();
        offset += len;
        out.push(Raw {
            cat,
            gender,
            number,
            person,
            mt,
            lemma,
        });
    }
    out
}

/// Lexique morphologique chargé en mémoire.
struct Morphology {
    map: fst::Map<&'static [u8]>,
    blob: &'static [u8],
    freq: fst::Map<&'static [u8]>,
    /// Index de génération `(lemme, mt, personne, nombre) → forme`, construit
    /// paresseusement (uniquement si une conjugaison doit être engendrée).
    gen: OnceLock<HashMap<(String, u8, u8, u8), String>>,
    /// Index de déclinaison adjectivale `(lemme, genre, nombre) → forme`,
    /// construit paresseusement (uniquement si un accord d'adjectif doit être
    /// engendré). Les codes valent 0 lorsque le trait est absent.
    decl: OnceLock<HashMap<(String, u8, u8), String>>,
    /// Index des participes passés `(lemme, genre, nombre) → forme`, construit
    /// paresseusement. Le lemme est l'infinitif (« partir » → « partie »).
    part: OnceLock<HashMap<(String, u8, u8), String>>,
}

impl Morphology {
    fn load() -> Result<Self, fst::Error> {
        Ok(Morphology {
            map: fst::Map::new(MORPHO_FST)?,
            blob: MORPHO_BIN,
            freq: fst::Map::new(MORPHO_FREQ)?,
            gen: OnceLock::new(),
            decl: OnceLock::new(),
            part: OnceLock::new(),
        })
    }

    fn records_of(&self, form: &str) -> Vec<Raw> {
        let key = form.to_lowercase();
        let Some(value) = self.map.get(key.as_bytes()) else {
            return Vec::new();
        };
        let count = (value & 0xFF) as usize;
        let offset = (value >> 8) as usize;
        decode_records(self.blob, offset, count)
    }

    fn lookup(&self, form: &str) -> Vec<Morph> {
        self.records_of(form)
            .into_iter()
            .map(|r| Morph {
                lemma: r.lemma,
                category: decode_category(r.cat),
                gender: decode_gender(r.gender),
                number: decode_number(r.number),
                person: decode_person(r.person),
            })
            .collect()
    }

    fn verb_forms(&self, form: &str) -> Vec<VerbForm> {
        self.records_of(form)
            .into_iter()
            .filter_map(|r| {
                if decode_category(r.cat) != MorphCategory::Verb {
                    return None;
                }
                Some(VerbForm {
                    mood_tense: MoodTense::from_code(r.mt)?,
                    person: decode_person(r.person)?,
                    number: decode_number(r.number)?,
                    lemma: r.lemma,
                })
            })
            .collect()
    }

    /// Fréquence (livres, ×100) de la forme, ou 0 si inconnue.
    fn frequency(&self, form: &str) -> u64 {
        self.freq.get(form.to_lowercase().as_bytes()).unwrap_or(0)
    }

    /// Construit l'index de génération en parcourant tout le FST une fois.
    fn build_gen(&self) -> HashMap<(String, u8, u8, u8), String> {
        let mut index = HashMap::new();
        let mut stream = self.map.stream();
        while let Some((key, value)) = stream.next() {
            let Ok(form) = std::str::from_utf8(key) else {
                continue;
            };
            let count = (value & 0xFF) as usize;
            let offset = (value >> 8) as usize;
            for r in decode_records(self.blob, offset, count) {
                if decode_category(r.cat) == MorphCategory::Verb
                    && r.mt != 0
                    && r.person != 0
                    && r.number != 0
                {
                    index
                        .entry((r.lemma, r.mt, r.person, r.number))
                        .or_insert_with(|| form.to_string());
                }
            }
        }
        index
    }

    fn conjugate(
        &self,
        lemma: &str,
        mt: MoodTense,
        person: Person,
        number: Number,
    ) -> Option<String> {
        let number_code = match number {
            Number::Singular => 1,
            Number::Plural => 2,
            Number::Invariable => return None,
        };
        let person_code = match person {
            Person::First => 1,
            Person::Second => 2,
            Person::Third => 3,
        };
        let index = self.gen.get_or_init(|| self.build_gen());
        index
            .get(&(lemma.to_string(), mt.code(), person_code, number_code))
            .cloned()
            .or_else(|| patch_conjugate_static(lemma, mt, person, number))
            .or_else(|| derive_conjugate(lemma, mt, person, number, index))
    }

    /// Construit l'index de déclinaison adjectivale en parcourant le FST une
    /// fois. Clé : `(lemme, genre, nombre)`, les codes valant 0 si le trait est
    /// absent (adjectif épicène ou invariable en nombre).
    fn build_decl(&self) -> HashMap<(String, u8, u8), String> {
        let mut index = HashMap::new();
        let mut stream = self.map.stream();
        while let Some((key, value)) = stream.next() {
            let Ok(form) = std::str::from_utf8(key) else {
                continue;
            };
            let count = (value & 0xFF) as usize;
            let offset = (value >> 8) as usize;
            for r in decode_records(self.blob, offset, count) {
                if decode_category(r.cat) == MorphCategory::Adjective {
                    index
                        .entry((r.lemma, r.gender, r.number))
                        .or_insert_with(|| form.to_string());
                }
            }
        }
        index
    }

    /// Construit l'index des participes passés en deux passes.
    ///
    /// Passe 1 : indexe les formes qui portent directement un enregistrement
    /// verbal sans personne (« mangée », « parties ») et, pour les formes qui
    /// cumulent aussi une lecture adjectivale (p. ex. « lu » = Verb PP lire +
    /// Adj lu), construit le mapping adj_lemme → verb_lemme.
    ///
    /// Passe 2 : les formes dont le Lefff ne stocke l'accord qu'en lecture
    /// adjectivale (p. ex. « lus » = Adj lu Masc Pl) sont indexées comme PP
    /// grâce au mapping établi en passe 1.
    fn build_part(&self) -> HashMap<(String, u8, u8), String> {
        let mut index: HashMap<(String, u8, u8), String> = HashMap::new();
        // adj_lemme → verb_lemme (construit sur les formes ambiguës PP+Adj)
        let mut adj_to_verb: HashMap<String, String> = HashMap::new();

        // Passe 1 : entrées PP directes + mapping adj_lemme → verb_lemme.
        {
            let mut stream = self.map.stream();
            while let Some((key, value)) = stream.next() {
                let Ok(form) = std::str::from_utf8(key) else {
                    continue;
                };
                let count = (value & 0xFF) as usize;
                let offset = (value >> 8) as usize;
                let records = decode_records(self.blob, offset, count);

                let mut pp_lemmas: Vec<String> = vec![];
                let mut adj_lemmas: Vec<String> = vec![];

                for r in &records {
                    if decode_category(r.cat) == MorphCategory::Verb
                        && r.person == 0
                        && r.gender != 0
                        && r.number != 0
                    {
                        index
                            .entry((r.lemma.clone(), r.gender, r.number))
                            .or_insert_with(|| form.to_string());
                        pp_lemmas.push(r.lemma.clone());
                    } else if decode_category(r.cat) == MorphCategory::Adjective {
                        adj_lemmas.push(r.lemma.clone());
                    }
                }

                // Forme à la fois PP verbal et adjectivale → mémoriser le lien.
                for vl in &pp_lemmas {
                    for al in &adj_lemmas {
                        adj_to_verb.entry(al.clone()).or_insert_with(|| vl.clone());
                    }
                }
            }
        }

        // Passe 2 : formes adjectivales seules dont l'adj_lemme est connu.
        {
            let mut stream = self.map.stream();
            while let Some((key, value)) = stream.next() {
                let Ok(form) = std::str::from_utf8(key) else {
                    continue;
                };
                let count = (value & 0xFF) as usize;
                let offset = (value >> 8) as usize;
                for r in decode_records(self.blob, offset, count) {
                    if decode_category(r.cat) == MorphCategory::Adjective
                        && r.gender != 0
                        && r.number != 0
                    {
                        if let Some(verb_lemma) = adj_to_verb.get(&r.lemma) {
                            index
                                .entry((verb_lemma.clone(), r.gender, r.number))
                                .or_insert_with(|| form.to_string());
                        }
                    }
                }
            }
        }

        index
    }

    fn participle(&self, lemma: &str, gender: Gender, number: Number) -> Option<String> {
        let g = match gender {
            Gender::Masculine => 1,
            Gender::Feminine => 2,
            Gender::Epicene => return None,
        };
        let n = match number {
            Number::Singular => 1,
            Number::Plural => 2,
            Number::Invariable => return None,
        };
        let index = self.part.get_or_init(|| self.build_part());
        index.get(&(lemma.to_string(), g, n)).cloned()
    }

    fn decline(&self, lemma: &str, gender: Gender, number: Number) -> Option<String> {
        let g = match gender {
            Gender::Masculine => 1,
            Gender::Feminine => 2,
            Gender::Epicene => 0,
        };
        let n = match number {
            Number::Singular => 1,
            Number::Plural => 2,
            Number::Invariable => 0,
        };
        let index = self.decl.get_or_init(|| self.build_decl());
        // Du plus spécifique au plus permissif : un adjectif peut être
        // invariable en genre (« rouge ») ou en nombre.
        let lemma = lemma.to_string();
        [(g, n), (0, n), (g, 0), (0, 0)]
            .into_iter()
            .find_map(|(gc, nc)| index.get(&(lemma.clone(), gc, nc)).cloned())
    }
}

// ---------------------------------------------------------------------------
// Patch de conjugaison : table statique + dérivation algorithmique
// ---------------------------------------------------------------------------

/// Ajoute un accent circonflexe sur la dernière voyelle (gère les voyelles
/// déjà circonflexées : « û » reste « û »).
fn circumflex_last_vowel(s: &str) -> Option<String> {
    let mut chars: Vec<char> = s.chars().collect();
    for c in chars.iter_mut().rev() {
        let circ = match *c {
            'a' | 'â' => 'â', 'e' | 'ê' => 'ê',
            'i' | 'î' => 'î', 'o' | 'ô' => 'ô',
            'u' | 'û' => 'û',
            _ => continue,
        };
        *c = circ;
        return Some(chars.into_iter().collect());
    }
    None
}

/// Supprime l'accent circonflexe de la dernière voyelle (inverse de
/// `circumflex_last_vowel`).  Utilisé pour le passé simple 1sg/2sg/3pl.
fn plain_last_vowel(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    for c in chars.iter_mut().rev() {
        let plain = match *c {
            'â' => 'a', 'ê' => 'e', 'î' => 'i', 'ô' => 'o', 'û' => 'u',
            'a' | 'e' | 'i' | 'o' | 'u' => break, // déjà sans accent : rien à faire
            _ => continue,
        };
        *c = plain;
        break;
    }
    chars.into_iter().collect()
}

/// Table statique des cas non dérivables algorithmiquement et du paradigme
/// réformé de « assoir/rassoir » (entièrement absent du Lefff).
fn patch_conjugate_static(lemma: &str, mt: MoodTense, person: Person, number: Number) -> Option<String> {
    use MoodTense::*;
    use Person::*;
    use Number::*;
    let form: &str = match (lemma, mt, person, number) {
        // aller — passé simple complet (absent du Lefff)
        ("aller", IndicativePast, First,  Singular) => "allai",
        ("aller", IndicativePast, Second, Singular) => "allas",
        ("aller", IndicativePast, Third,  Singular) => "alla",
        ("aller", IndicativePast, First,  Plural)   => "allâmes",
        ("aller", IndicativePast, Second, Plural)   => "allâtes",
        ("aller", IndicativePast, Third,  Plural)   => "allèrent",
        // mouvoir — passé simple (non dérivable algorithmiquement)
        ("mouvoir", IndicativePast, First,  Singular) => "mus",
        ("mouvoir", IndicativePast, Second, Singular) => "mus",
        ("mouvoir", IndicativePast, Third,  Singular) => "mut",
        ("mouvoir", IndicativePast, First,  Plural)   => "mûmes",
        ("mouvoir", IndicativePast, Second, Plural)   => "mûtes",
        ("mouvoir", IndicativePast, Third,  Plural)   => "murent",
        // prendre / partir — subjonctif présent 3pl (ne se dérive pas du 1pl)
        ("prendre", SubjunctivePresent, Third, Plural) => "prennent",
        ("partir",  SubjunctivePresent, Third, Plural) => "partent",
        // absoudre — IndPres/IndImperf/SubjPres/Impér (tige double : absous-/absolv-)
        ("absoudre", IndicativePresent, First,  Plural)   => "absolvons",
        ("absoudre", IndicativePresent, Second, Plural)   => "absolvez",
        ("absoudre", IndicativeImperfect, First,  Singular) => "absolvais",
        ("absoudre", IndicativeImperfect, Second, Singular) => "absolvais",
        ("absoudre", IndicativeImperfect, Third,  Singular) => "absolvait",
        ("absoudre", IndicativeImperfect, First,  Plural)   => "absolvions",
        ("absoudre", IndicativeImperfect, Second, Plural)   => "absolviez",
        ("absoudre", IndicativeImperfect, Third,  Plural)   => "absolvaient",
        ("absoudre", SubjunctivePresent, First,  Plural)   => "absolvions",
        ("absoudre", SubjunctivePresent, Second, Plural)   => "absolviez",
        ("absoudre", Imperative, First,  Plural)   => "absolvons",
        ("absoudre", Imperative, Second, Plural)   => "absolvez",
        // dissoudre — même paradigme qu'absoudre
        ("dissoudre", IndicativePresent, First,  Plural)   => "dissolvons",
        ("dissoudre", IndicativePresent, Second, Plural)   => "dissolvez",
        ("dissoudre", IndicativeImperfect, First,  Singular) => "dissolvais",
        ("dissoudre", IndicativeImperfect, Second, Singular) => "dissolvais",
        ("dissoudre", IndicativeImperfect, Third,  Singular) => "dissolvait",
        ("dissoudre", IndicativeImperfect, First,  Plural)   => "dissolvions",
        ("dissoudre", IndicativeImperfect, Second, Plural)   => "dissolviez",
        ("dissoudre", IndicativeImperfect, Third,  Plural)   => "dissolvaient",
        ("dissoudre", SubjunctivePresent, First,  Plural)   => "dissolvions",
        ("dissoudre", SubjunctivePresent, Second, Plural)   => "dissolviez",
        ("dissoudre", Imperative, First,  Plural)   => "dissolvons",
        ("dissoudre", Imperative, Second, Plural)   => "dissolvez",
        // clore — IndPres 1pl/2pl (tige « clos- »)
        ("clore", IndicativePresent, First,  Plural)   => "closons",
        ("clore", IndicativePresent, Second, Plural)   => "closez",
        ("clore", SubjunctivePresent, First,  Plural)   => "closions",
        ("clore", SubjunctivePresent, Second, Plural)   => "closiez",
        ("clore", Imperative, First,  Plural)   => "closons",
        ("clore", Imperative, Second, Plural)   => "closez",
        // croître — IndPres 1pl/2pl/Impér (tige « croiss- »)
        ("croître", IndicativePresent, First,  Plural)   => "croissons",
        ("croître", IndicativePresent, Second, Plural)   => "croissez",
        ("croître", IndicativeImperfect, First,  Plural)   => "croissions",
        ("croître", IndicativeImperfect, Second, Singular) => "croissais",
        ("croître", IndicativeImperfect, Second, Plural)   => "croissiez",
        ("croître", SubjunctivePresent, First,  Plural)   => "croissions",
        ("croître", SubjunctivePresent, Second, Plural)   => "croissiez",
        ("croître", Imperative, First,  Plural)   => "croissons",
        ("croître", Imperative, Second, Singular) => "croîs",
        // coudre — tige « cous- » pour les pluriels
        ("coudre", IndicativePresent, First,  Plural)   => "cousons",
        ("coudre", IndicativePresent, Second, Plural)   => "cousez",
        ("coudre", IndicativePresent, Third,  Plural)   => "cousent",
        ("coudre", IndicativeImperfect, First,  Singular) => "cousais",
        ("coudre", IndicativeImperfect, First,  Plural)   => "cousions",
        ("coudre", IndicativeImperfect, Second, Singular) => "cousais",
        ("coudre", SubjunctivePresent, First,  Plural)   => "cousions",
        ("coudre", SubjunctivePresent, Second, Plural)   => "cousiez",
        ("coudre", Imperative, First,  Plural)   => "cousons",
        ("coudre", Imperative, Second, Singular) => "couds",
        // moudre — tige « moul- » pour les pluriels
        ("moudre", IndicativePresent, First,  Plural)   => "moulons",
        ("moudre", IndicativePresent, Second, Plural)   => "moulez",
        ("moudre", IndicativePresent, Third,  Plural)   => "moulent",
        ("moudre", Imperative, First,  Plural)   => "moulons",
        ("moudre", Imperative, Second, Singular) => "mouds",
        // convaincre — tige « convainqu- »
        ("convaincre", IndicativePresent, First,  Plural)   => "convainquons",
        ("convaincre", IndicativeImperfect, First,  Singular) => "convainquais",
        ("convaincre", IndicativeImperfect, First,  Plural)   => "convainquions",
        ("convaincre", IndicativeImperfect, Second, Singular) => "convainquais",
        ("convaincre", SubjunctivePresent, First,  Plural)   => "convainquions",
        ("convaincre", SubjunctivePresent, Second, Plural)   => "convainquiez",
        // revêtir — tige « revêt- »
        ("revêtir", IndicativePresent, First,  Plural)   => "revêtons",
        ("revêtir", IndicativePresent, Second, Plural)   => "revêtez",
        ("revêtir", IndicativeImperfect, First,  Plural)   => "revêtions",
        ("revêtir", IndicativeImperfect, Second, Singular) => "revêtais",
        ("revêtir", IndicativeImperfect, Second, Plural)   => "revêtiez",
        ("revêtir", SubjunctivePresent, First,  Plural)   => "revêtions",
        ("revêtir", SubjunctivePresent, Second, Plural)   => "revêtiez",
        ("revêtir", Imperative, First,  Plural)   => "revêtons",
        // assoir — paradigme réformé complet (absent du Lefff)
        ("assoir", IndicativePresent, First,  Singular) => "assois",
        ("assoir", IndicativePresent, Second, Singular) => "assois",
        ("assoir", IndicativePresent, Third,  Singular) => "assoit",
        ("assoir", IndicativePresent, First,  Plural)   => "assoyons",
        ("assoir", IndicativePresent, Second, Plural)   => "assoyez",
        ("assoir", IndicativePresent, Third,  Plural)   => "assoient",
        ("assoir", IndicativeImperfect, First,  Singular) => "assoyais",
        ("assoir", IndicativeImperfect, Second, Singular) => "assoyais",
        ("assoir", IndicativeImperfect, Third,  Singular) => "assoyait",
        ("assoir", IndicativeImperfect, First,  Plural)   => "assoyions",
        ("assoir", IndicativeImperfect, Second, Plural)   => "assoyiez",
        ("assoir", IndicativeImperfect, Third,  Plural)   => "assoyaient",
        ("assoir", IndicativeFuture, First,  Singular) => "assoirai",
        ("assoir", IndicativeFuture, Second, Singular) => "assoiras",
        ("assoir", IndicativeFuture, Third,  Singular) => "assoira",
        ("assoir", IndicativeFuture, First,  Plural)   => "assoirons",
        ("assoir", IndicativeFuture, Second, Plural)   => "assoirez",
        ("assoir", IndicativeFuture, Third,  Plural)   => "assoiront",
        ("assoir", ConditionalPresent, First,  Singular) => "assoirais",
        ("assoir", ConditionalPresent, Second, Singular) => "assoirais",
        ("assoir", ConditionalPresent, Third,  Singular) => "assoirait",
        ("assoir", ConditionalPresent, First,  Plural)   => "assoirions",
        ("assoir", ConditionalPresent, Second, Plural)   => "assoiriez",
        ("assoir", ConditionalPresent, Third,  Plural)   => "assoiraient",
        ("assoir", SubjunctivePresent, First,  Singular) => "assoie",
        ("assoir", SubjunctivePresent, Second, Singular) => "assoies",
        ("assoir", SubjunctivePresent, Third,  Singular) => "assoie",
        ("assoir", SubjunctivePresent, First,  Plural)   => "assoyions",
        ("assoir", SubjunctivePresent, Second, Plural)   => "assoyiez",
        ("assoir", SubjunctivePresent, Third,  Plural)   => "assoient",
        ("assoir", IndicativePast, First,  Singular) => "assis",
        ("assoir", IndicativePast, Second, Singular) => "assis",
        ("assoir", IndicativePast, Third,  Singular) => "assit",
        ("assoir", IndicativePast, First,  Plural)   => "assîmes",
        ("assoir", IndicativePast, Second, Plural)   => "assîtes",
        ("assoir", IndicativePast, Third,  Plural)   => "assirent",
        ("assoir", SubjunctiveImperfect, First,  Singular) => "assisse",
        ("assoir", SubjunctiveImperfect, Second, Singular) => "assisses",
        ("assoir", SubjunctiveImperfect, Third,  Singular) => "assît",
        ("assoir", SubjunctiveImperfect, First,  Plural)   => "assissions",
        ("assoir", SubjunctiveImperfect, Second, Plural)   => "assissiez",
        ("assoir", SubjunctiveImperfect, Third,  Plural)   => "assissent",
        ("assoir", Imperative, Second, Singular) => "assois",
        ("assoir", Imperative, First,  Plural)   => "assoyons",
        ("assoir", Imperative, Second, Plural)   => "assoyez",
        // rassoir — paradigme réformé (calqué sur assoir)
        ("rassoir", IndicativePresent, First,  Singular) => "rassois",
        ("rassoir", IndicativePresent, Second, Singular) => "rassois",
        ("rassoir", IndicativePresent, Third,  Singular) => "rassoit",
        ("rassoir", IndicativePresent, First,  Plural)   => "rassoyons",
        ("rassoir", IndicativePresent, Second, Plural)   => "rassoyez",
        ("rassoir", IndicativePresent, Third,  Plural)   => "rassoient",
        ("rassoir", IndicativeImperfect, First,  Singular) => "rassoyais",
        ("rassoir", IndicativeImperfect, Second, Singular) => "rassoyais",
        ("rassoir", IndicativeImperfect, Third,  Singular) => "rassoyait",
        ("rassoir", IndicativeImperfect, First,  Plural)   => "rassoyions",
        ("rassoir", IndicativeImperfect, Second, Plural)   => "rassoyiez",
        ("rassoir", IndicativeImperfect, Third,  Plural)   => "rassoyaient",
        ("rassoir", IndicativeFuture, First,  Singular) => "rassoirai",
        ("rassoir", IndicativeFuture, Second, Singular) => "rassoiras",
        ("rassoir", IndicativeFuture, Third,  Singular) => "rassoira",
        ("rassoir", IndicativeFuture, First,  Plural)   => "rassoirons",
        ("rassoir", IndicativeFuture, Second, Plural)   => "rassoirez",
        ("rassoir", IndicativeFuture, Third,  Plural)   => "rassoiront",
        ("rassoir", ConditionalPresent, First,  Singular) => "rassoirais",
        ("rassoir", ConditionalPresent, Second, Singular) => "rassoirais",
        ("rassoir", ConditionalPresent, Third,  Singular) => "rassoirait",
        ("rassoir", ConditionalPresent, First,  Plural)   => "rassoirions",
        ("rassoir", ConditionalPresent, Second, Plural)   => "rassoiriez",
        ("rassoir", ConditionalPresent, Third,  Plural)   => "rassoiraient",
        ("rassoir", SubjunctivePresent, First,  Singular) => "rassoie",
        ("rassoir", SubjunctivePresent, Second, Singular) => "rassoies",
        ("rassoir", SubjunctivePresent, Third,  Singular) => "rassoie",
        ("rassoir", SubjunctivePresent, First,  Plural)   => "rassoyions",
        ("rassoir", SubjunctivePresent, Second, Plural)   => "rassoyiez",
        ("rassoir", SubjunctivePresent, Third,  Plural)   => "rassoient",
        ("rassoir", IndicativePast, First,  Singular) => "rassis",
        ("rassoir", IndicativePast, Second, Singular) => "rassis",
        ("rassoir", IndicativePast, Third,  Singular) => "rassit",
        ("rassoir", IndicativePast, First,  Plural)   => "rassîmes",
        ("rassoir", IndicativePast, Second, Plural)   => "rassîtes",
        ("rassoir", IndicativePast, Third,  Plural)   => "rassirent",
        ("rassoir", SubjunctiveImperfect, First,  Singular) => "rassisse",
        ("rassoir", SubjunctiveImperfect, Second, Singular) => "rassisses",
        ("rassoir", SubjunctiveImperfect, Third,  Singular) => "rassît",
        ("rassoir", SubjunctiveImperfect, First,  Plural)   => "rassissions",
        ("rassoir", SubjunctiveImperfect, Second, Plural)   => "rassissiez",
        ("rassoir", SubjunctiveImperfect, Third,  Plural)   => "rassissent",
        ("rassoir", Imperative, Second, Singular) => "rassois",
        ("rassoir", Imperative, First,  Plural)   => "rassoyons",
        ("rassoir", Imperative, Second, Plural)   => "rassoyez",
        _ => return None,
    };
    Some(form.to_string())
}

/// Récupère le passé simple 3sg depuis l'index du Lefff ou la table statique.
/// Repli composé : pour les familles -faire/-venir/-tenir/etc., préfixe + PS3sg de la base.
fn get_ps3sg(lemma: &str, index: &HashMap<(String, u8, u8, u8), String>) -> Option<String> {
    let key = (lemma.to_string(), MoodTense::IndicativePast.code(), 3u8, 1u8);
    let from_index = index.get(&key).cloned()
        .or_else(|| patch_conjugate_static(lemma, MoodTense::IndicativePast, Person::Third, Number::Singular));
    if from_index.is_some() { return from_index; }
    // Composé : cherche le PS 3sg du verbe de base et préfixe
    let ps = MoodTense::IndicativePast.code();
    for &base in &["faire", "venir", "tenir", "vouloir", "pouvoir", "voir",
                   "valoir", "mouvoir", "courir", "mettre", "prendre",
                   "rompre", "dire", "naître", "connaître"] {
        if lemma.ends_with(base) && lemma.len() > base.len() {
            let prefix = &lemma[..lemma.len() - base.len()];
            let base_key = (base.to_string(), ps, 3u8, 1u8);
            if let Some(base_form) = index.get(&base_key) {
                return Some(format!("{prefix}{base_form}"));
            }
            break;
        }
    }
    None
}

/// Point d'entrée de la dérivation algorithmique (après échec Lefff + table statique).
fn derive_conjugate(
    lemma: &str,
    mt: MoodTense,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use MoodTense::*;
    use Number::*;
    use Person::*;
    match mt {
        IndicativePast => derive_passe_simple(lemma, person, number, index),
        SubjunctiveImperfect => derive_subj_imperf(lemma, person, number, index),
        IndicativePresent => derive_ind_pres(lemma, person, number, index),
        IndicativeImperfect => derive_imparfait(lemma, person, number, index),
        IndicativeFuture => derive_futur(lemma, person, number, index),
        ConditionalPresent => derive_conditionnel(lemma, person, number, index),
        Imperative => derive_imperatif(lemma, person, number, index),
        SubjunctivePresent if matches!((person, number), (Third, Plural)) => {
            derive_subj_pres_3pl(lemma, index)
        }
        SubjunctivePresent if matches!(number, Plural) => {
            derive_subj_pres_1p2p(lemma, person, index)
        }
        SubjunctivePresent => derive_subj_pres_sing(lemma, person, index),
    }
}

/// Tige du IndPres 1pl (ex. « peignons » → « peign ») : cherche d'abord le gen
/// index, puis dérive algorithmiquement (-indre/-uire/-traire), puis par
/// substitution de préfixe sur les composés de verbes irréguliers connus.
fn ind_pres_1pl_stem(lemma: &str, index: &HashMap<(String, u8, u8, u8), String>) -> Option<String> {
    let ip = MoodTense::IndicativePresent.code();
    let lstr = lemma.to_string();

    // 1. Gen index direct
    if let Some(nous) = index.get(&(lstr.clone(), ip, 1u8, 2u8)) {
        if nous.ends_with("ons") {
            return Some(nous[..nous.len() - 3].to_string());
        }
    }

    // 2. -indre / -uire / -traire depuis IndPres 3sg
    if let Some(sg3) = index.get(&(lstr.clone(), ip, 3u8, 1u8)) {
        if let Some(gn) = ind_pres_gn_stem(lemma, sg3) {
            return Some(gn);
        }
    }

    // 3. Composé d'un verbe de base connu : préfixe + tige 1pl de la base.
    //    On ne cherche que si le préfixe est non-vide.
    let base_verbs: &[&str] = &[
        "venir", "tenir", "prendre", "mettre", "courir", "battre",
        "faire", "voir", "vivre", "suivre", "rompre",
        "valoir", "mouvoir", "connaître", "paraître", "naître",
        "plaire", "servir",
    ];
    for &base in base_verbs {
        if lemma.ends_with(base) && lemma.len() > base.len() {
            let prefix = &lemma[..lemma.len() - base.len()];
            let base_key = (base.to_string(), ip, 1u8, 2u8);
            if let Some(base_nous) = index.get(&base_key) {
                if base_nous.ends_with("ons") {
                    let base_stem = &base_nous[..base_nous.len() - 3];
                    return Some(format!("{prefix}{base_stem}"));
                }
            }
            break;
        }
    }
    // Composés à tige 1pl irrégulière : suffix_du_composé → tige_IndPres_1pl
    // (sans « ons », qui sera rajouté par l'appelant).
    // Ordre : du plus spécifique au plus général.
    let suffix_stem: &[(&str, &str)] = &[
        ("écrire", "écriv"),  // décrire, réécrire (prefix court, écriv correct)
        ("scrire",  "scriv"),  // inscrire, transcrire, prescrire, proscrire, souscrire
        ("lire",    "lis"),    // relire, élire
        ("dire",    "dis"),    // contredire, prédire, médire, redire
        ("cevoir",  "cev"),   // apercevoir, concevoir, décevoir, percevoir (recevoir : gen index)
    ];
    for &(suf, stem) in suffix_stem {
        if lemma.ends_with(suf) && lemma.len() > suf.len() {
            // Guard : éviter d'appliquer la règle à la base elle-même
            // ("dire" ne matche pas "dire", "écrire" ne matche pas "écrire")
            let base_same = matches!(
                (suf, lemma),
                ("écrire", "écrire") | ("lire", "lire") | ("dire", "dire")
            );
            if !base_same {
                let prefix = &lemma[..lemma.len() - suf.len()];
                return Some(format!("{prefix}{stem}"));
            }
        }
    }

    None
}

/// Pour les familles -indre, -uire et -traire, calcule la tige « étendue »
/// du présent (1pl) depuis le 3sg. Renvoie None pour les autres familles.
fn ind_pres_gn_stem(lemma: &str, sg3: &str) -> Option<String> {
    if !sg3.ends_with('t') {
        return None;
    }
    let base = &sg3[..sg3.len() - 1]; // strip 't'

    // -indre : « peint » → tige « peign »
    if base.ends_with('n') && !base.ends_with("ien") {
        return Some(format!("{}gn", &base[..base.len() - 1]));
    }
    // -uire : « conduit » → tige « conduis »
    if lemma.ends_with("uire") {
        return Some(format!("{base}s"));
    }
    // -traire : « abstrait » → tige « abstray »
    if lemma.ends_with("traire") && base.ends_with('i') {
        return Some(format!("{}y", &base[..base.len() - 1]));
    }
    None
}

/// Indicatif présent : dérivation pour -indre, -uire, -traire, -dre/-cre,
/// et composés (via tige 1pl connue).
fn derive_ind_pres(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let ip = MoodTense::IndicativePresent.code();
    let lstr = lemma.to_string();

    // Cherche IndPres 3sg pour les familles en « -t ».
    if let Some(sg3) = index.get(&(lstr.clone(), ip, 3u8, 1u8)) {
        if sg3.ends_with('t') {
            let base = &sg3[..sg3.len() - 1];

            // -indre : « pein » → tige « peign »
            if base.ends_with('n') && !base.ends_with("ien") {
                let gn = format!("{}gn", &base[..base.len() - 1]);
                if let Some(r) = match (person, number) {
                    (First | Second, Singular) => Some(format!("{base}s")),
                    (First,  Plural)   => Some(format!("{gn}ons")),
                    (Second, Plural)   => Some(format!("{gn}ez")),
                    (Third,  Plural)   => Some(format!("{gn}ent")),
                    _ => None,
                } { return Some(r); }
            }

            // -uire : « condui » → tige « conduis »
            if lemma.ends_with("uire") {
                if let Some(r) = match (person, number) {
                    (Second, Singular) => Some(format!("{base}s")),
                    (First,  Plural)   => Some(format!("{base}sons")),
                    (Second, Plural)   => Some(format!("{base}sez")),
                    (Third,  Plural)   => Some(format!("{base}sent")),
                    _ => None,
                } { return Some(r); }
            }

            // -traire : « abstrai » → tige « abstray » (avant voyelle)
            if lemma.ends_with("traire") && base.ends_with('i') {
                let y_stem = format!("{}y", &base[..base.len() - 1]);
                if let Some(r) = match (person, number) {
                    (Second, Singular) => Some(format!("{base}s")),
                    (First,  Plural)   => Some(format!("{y_stem}ons")),
                    (Second, Plural)   => Some(format!("{y_stem}ez")),
                    (Third,  Plural)   => Some(format!("{base}ent")),
                    _ => None,
                } { return Some(r); }
            }
        }

        // -dre/-cre : 3sg terminé par 'd' ou 'c' → 1sg = 2sg = 3sg + 's'
        if (sg3.ends_with('d') || sg3.ends_with('c'))
            && matches!((person, number), (First | Second, Singular))
        {
            return Some(format!("{sg3}s"));
        }
    }

    // Repli 2sg = 1sg (vrai pour la majorité des verbes irréguliers).
    if matches!((person, number), (Second, Singular)) {
        if let Some(sg1) = index.get(&(lstr.clone(), ip, 1u8, 1u8)) {
            return Some(sg1.clone());
        }
    }

    // Repli depuis la tige 1pl (connue ou calculée par composé/famille).
    // Ne retourne que pour les formes plurielles ; les sg continuent vers la section suivante.
    if let Some(stem) = ind_pres_1pl_stem(lemma, index) {
        match (person, number) {
            (First,  Plural) => return Some(format!("{stem}ons")),
            (Second, Plural) => return Some(format!("{stem}ez")),
            (Third,  Plural) => return Some(format!("{stem}ent")),
            _ => {}  // sg : continue vers la dérivation composé ci-dessous
        }
    }

    // Composé sg : préfixe + forme sg du verbe de base.
    // Ordre : du plus spécifique au plus général pour éviter les faux positifs.
    if matches!((person, number), (First | Second | Third, Singular)) {
        let person_code = match person { First => 1, Second => 2, Third => 3 };
        // Verbes de base dont les formes sg sont dans le gen index.
        for &base in &["venir", "tenir", "faire", "voir", "mouvoir", "valoir",
                       "dire", "lire", "prendre", "courir", "connaître", "paraître",
                       "plaire", "servir"] {
            if lemma.ends_with(base) && lemma.len() > base.len() {
                let prefix = &lemma[..lemma.len() - base.len()];
                let base_key = (base.to_string(), ip, person_code, 1u8);
                if let Some(base_form) = index.get(&base_key) {
                    return Some(format!("{prefix}{base_form}"));
                }
                break;
            }
        }
        // Famille -crire : stem se termine en « v » → 1sg/2sg = stem[..-1]+"is", 3sg = "it"
        // (ex. « proscr » + « iv » → 1sg « proscris », 3sg « proscrit »)
        if lemma.ends_with("crire") && lemma.len() > 5 {
            let base = &lemma[..lemma.len() - 3]; // strip "ire" → "proscr"
            return Some(match person {
                First | Second => format!("{base}is"),
                Third          => format!("{base}it"),
            });
        }
    }

    None
}

/// Indicatif imparfait : dérivé du IndPres 1pl (réel ou calculé).
fn derive_imparfait(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let stem = ind_pres_1pl_stem(lemma, index)?;
    Some(match (person, number) {
        (First,  Singular) | (Second, Singular) => format!("{stem}ais"),
        (Third,  Singular)                      => format!("{stem}ait"),
        (First,  Plural)                        => format!("{stem}ions"),
        (Second, Plural)                        => format!("{stem}iez"),
        (Third,  Plural)                        => format!("{stem}aient"),
        _ => return None,
    })
}

/// Indicatif futur : dérivé du futur 1sg (« ferai » → stem « fer »)
/// ou du conditionnel 1sg (« ferais » → stem « fer »).
fn derive_futur(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let stem = fut_stem(lemma, index)?;
    Some(match (person, number) {
        (First,  Singular) => format!("{stem}ai"),
        (Second, Singular) => format!("{stem}as"),
        (Third,  Singular) => format!("{stem}a"),
        (First,  Plural)   => format!("{stem}ons"),
        (Second, Plural)   => format!("{stem}ez"),
        (Third,  Plural)   => format!("{stem}ont"),
        _ => return None,
    })
}

/// Conditionnel présent : même radical futur + terminaisons de l'imparfait.
fn derive_conditionnel(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let stem = fut_stem(lemma, index)?;
    Some(match (person, number) {
        (First,  Singular) | (Second, Singular) => format!("{stem}ais"),
        (Third,  Singular)                      => format!("{stem}ait"),
        (First,  Plural)                        => format!("{stem}ions"),
        (Second, Plural)                        => format!("{stem}iez"),
        (Third,  Plural)                        => format!("{stem}aient"),
        _ => return None,
    })
}

/// Radical futur (commun au futur et au conditionnel).
/// Cherche parmi toutes les formes disponibles de futur/conditionnel.
fn fut_stem(lemma: &str, index: &HashMap<(String, u8, u8, u8), String>) -> Option<String> {
    let lstr = lemma.to_string();
    let fc = MoodTense::IndicativeFuture.code();
    let cc = MoodTense::ConditionalPresent.code();
    // Futur 1sg (« ferai » → strip « ai »)
    if let Some(f) = index.get(&(lstr.clone(), fc, 1, 1)) {
        if f.ends_with("ai") { return Some(f[..f.len() - 2].to_string()); }
    }
    // Cond 1sg/2sg (« ferais » → strip « ais »)
    for p in [1u8, 2u8] {
        if let Some(f) = index.get(&(lstr.clone(), cc, p, 1)) {
            if f.ends_with("ais") { return Some(f[..f.len() - 3].to_string()); }
        }
    }
    // Cond 3sg (« ferait » → strip « ait »)
    if let Some(f) = index.get(&(lstr.clone(), cc, 3, 1)) {
        if f.ends_with("ait") { return Some(f[..f.len() - 3].to_string()); }
    }
    // Futur 3sg (« fera » → strip « a »)
    if let Some(f) = index.get(&(lstr.clone(), fc, 3, 1)) {
        if f.ends_with('a') { return Some(f[..f.len() - 1].to_string()); }
    }
    // Futur 3pl (« feront » → strip « ont »)
    if let Some(f) = index.get(&(lstr.clone(), fc, 3, 2)) {
        if f.ends_with("ont") { return Some(f[..f.len() - 3].to_string()); }
    }
    // Cond 3pl (« feraient » → strip « aient »)
    if let Some(f) = index.get(&(lstr, cc, 3, 2)) {
        if f.ends_with("aient") { return Some(f[..f.len() - 5].to_string()); }
    }
    None
}

/// Subjonctif présent 3pl : SubjPres 3sg + « nt » (si -e) ou + « ent » (si -t).
/// Repli : IndPres 3pl (même forme pour la plupart des verbes).
fn derive_subj_pres_3pl(
    lemma: &str,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    // -faire composés : tige SubjPres irrégulière « fass- »
    if lemma.ends_with("faire") && lemma.len() > "faire".len() {
        let prefix = &lemma[..lemma.len() - "faire".len()];
        return Some(format!("{prefix}fassent"));
    }
    let key = (lemma.to_string(), MoodTense::SubjunctivePresent.code(), 3u8, 1u8);
    if let Some(sg3) = index.get(&key) {
        if sg3.ends_with('e') {
            return Some(format!("{sg3}nt"));
        }
        if sg3.ends_with('t') {
            let stem = &sg3[..sg3.len() - 1];
            return Some(format!("{stem}ent"));
        }
    }
    // Repli 1 : IndPres 3pl depuis le Lefff
    let ind3pl_key = (lemma.to_string(), MoodTense::IndicativePresent.code(), 3u8, 2u8);
    if let Some(f) = index.get(&ind3pl_key) { return Some(f.clone()); }
    // Repli 2 : dérivation depuis tige 1pl (composés -crire/-lire/-dire/-traire)
    ind_pres_1pl_stem(lemma, index).map(|stem| format!("{stem}ent"))
}

/// Impératif : 2sg = IndPres 2sg (sans « s » pour les -er) ;
/// 1pl = tige 1pl + « ons » ; 2pl = IndPres 2pl ou tige + « ez »/« sez ».
fn derive_imperatif(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let lstr = lemma.to_string();
    let ip = MoodTense::IndicativePresent.code();
    match (person, number) {
        (Second, Singular) => {
            // Cherche d'abord dans le Lefff
            let ind2sg = index
                .get(&(lstr.clone(), ip, 2, 1))
                .cloned()
                .or_else(|| {
                    // -indre : 2sg = base_stem + "s"
                    let sg3_k = (lstr.clone(), ip, 3u8, 1u8);
                    let sg3 = index.get(&sg3_k)?;
                    if sg3.ends_with('t') {
                        let base = &sg3[..sg3.len() - 1];
                        if base.ends_with('n') && !base.ends_with("ien") {
                            return Some(format!("{base}s"));
                        }
                        if lemma.ends_with("uire") {
                            return Some(format!("{base}s"));
                        }
                        if lemma.ends_with("traire") && base.ends_with('i') {
                            return Some(format!("{base}s"));
                        }
                    }
                    None
                })
                // Repli final : dérive IndPres 2sg algorithmiquement (composés, etc.)
                .or_else(|| derive_ind_pres(lemma, Second, Singular, index))?;
            if lemma.ends_with("er") && ind2sg.ends_with('s') {
                Some(ind2sg[..ind2sg.len() - 1].to_string())
            } else {
                Some(ind2sg)
            }
        }
        (First, Plural) => {
            // Impér 1pl = IndPres 1pl
            index
                .get(&(lstr.clone(), ip, 1, 2))
                .cloned()
                .or_else(|| ind_pres_1pl_stem(lemma, index).map(|s| format!("{s}ons")))
        }
        (Second, Plural) => {
            // Impér 2pl = IndPres 2pl
            index
                .get(&(lstr.clone(), ip, 2, 2))
                .cloned()
                .or_else(|| {
                    let stem = ind_pres_1pl_stem(lemma, index)?;
                    // -uire : tige = "conduis" → 2pl "conduisez"
                    if lemma.ends_with("uire") {
                        Some(format!("{stem}ez"))
                    } else {
                        Some(format!("{stem}ez"))
                    }
                })
        }
        _ => None,
    }
}

/// Passé simple : toutes les formes à partir du 3sg.
///
/// Type -a (aller → alla) : 1sg=-ai, 2sg=-as, 1pl=-âmes, 2pl=-âtes, 3pl=-èrent.
/// Type -t (prit/put/vint) : 1sg=2sg=stem+s, 1pl=2pl=stem̂+mes/tes, 3pl=stem+rent.
fn derive_passe_simple(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    let sg3 = get_ps3sg(lemma, index)?;
    if sg3.ends_with('a') {
        let stem = &sg3[..sg3.len() - 1];
        Some(match (person, number) {
            (First,  Singular) => format!("{stem}ai"),
            (Second, Singular) => format!("{stem}as"),
            (Third,  Singular) => sg3.clone(),
            (First,  Plural)   => format!("{stem}âmes"),
            (Second, Plural)   => format!("{stem}âtes"),
            (Third,  Plural)   => format!("{stem}èrent"),
            _ => return None,
        })
    } else if sg3.ends_with('t') {
        let base = &sg3[..sg3.len() - 1];          // « pri », « crû »
        let base_circ = circumflex_last_vowel(base)?; // « prî », « crû »
        let base_plain = plain_last_vowel(base);      // « pri », « cru »
        Some(match (person, number) {
            (First,  Singular) => format!("{base_plain}s"),
            (Second, Singular) => format!("{base_plain}s"),
            (Third,  Singular) => sg3.clone(),
            (First,  Plural)   => format!("{base_circ}mes"),
            (Second, Plural)   => format!("{base_circ}tes"),
            (Third,  Plural)   => format!("{base_plain}rent"),
            _ => return None,
        })
    } else {
        None
    }
}

/// Subjonctif imparfait : dérivé du passé simple 3sg (même radical).
///
/// Type -a : 1sg=base+sse, 2sg=base+sses, 3sg=stem+ât, 1pl=base+ssions, etc.
/// Type -t : 1sg=stem+sse, 3sg=stem̂+t, 1pl=stem+ssions, etc.
fn derive_subj_imperf(
    lemma: &str,
    person: Person,
    number: Number,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    use Number::*;
    // Repli : si PS 3sg absent, essaie SubjImperf 3sg depuis le Lefff (ex. « concourût »)
    // qui a la même structure (base_circ + « t »).
    let sg3 = get_ps3sg(lemma, index).or_else(|| {
        let key = (lemma.to_string(), MoodTense::SubjunctiveImperfect.code(), 3u8, 1u8);
        index.get(&key).cloned()
    })?;
    if sg3.ends_with('a') {
        let base = sg3.as_str();
        let stem = &base[..base.len() - 1];
        Some(match (person, number) {
            (First,  Singular) => format!("{base}sse"),
            (Second, Singular) => format!("{base}sses"),
            (Third,  Singular) => format!("{stem}ât"),
            (First,  Plural)   => format!("{base}ssions"),
            (Second, Plural)   => format!("{base}ssiez"),
            (Third,  Plural)   => format!("{base}ssent"),
            _ => return None,
        })
    } else if sg3.ends_with('t') {
        let base = &sg3[..sg3.len() - 1];
        let base_circ = circumflex_last_vowel(base)?;
        let base_plain = plain_last_vowel(base);
        Some(match (person, number) {
            (First,  Singular) => format!("{base_plain}sse"),
            (Second, Singular) => format!("{base_plain}sses"),
            (Third,  Singular) => format!("{base_circ}t"),
            (First,  Plural)   => format!("{base_plain}ssions"),
            (Second, Plural)   => format!("{base_plain}ssiez"),
            (Third,  Plural)   => format!("{base_plain}ssent"),
            _ => return None,
        })
    } else {
        None
    }
}

/// Subjonctif présent 1pl/2pl : tige IndPres 1pl + « ions »/« iez ».
fn derive_subj_pres_1p2p(
    lemma: &str,
    person: Person,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    // -faire composés : SubjPres 1pl/2pl irréguliers (« fassions »/« fassiez »)
    if lemma.ends_with("faire") && lemma.len() > "faire".len() {
        let prefix = &lemma[..lemma.len() - "faire".len()];
        return Some(match person {
            Person::First  => format!("{prefix}fassions"),
            Person::Second => format!("{prefix}fassiez"),
            _ => return None,
        });
    }
    let stem = ind_pres_1pl_stem(lemma, index)?;
    Some(match person {
        Person::First  => format!("{stem}ions"),
        Person::Second => format!("{stem}iez"),
        _ => return None,
    })
}

/// Subjonctif présent 1sg/2sg/3sg : dérivé du IndPres 3pl (« peignent » → « peigne »).
fn derive_subj_pres_sing(
    lemma: &str,
    person: Person,
    index: &HashMap<(String, u8, u8, u8), String>,
) -> Option<String> {
    use Person::*;
    // -faire composés : SubjPres 1sg/2sg/3sg irréguliers (tige « fass- »)
    if lemma.ends_with("faire") && lemma.len() > "faire".len() {
        let prefix = &lemma[..lemma.len() - "faire".len()];
        return Some(match person {
            First | Third => format!("{prefix}fasse"),
            Second        => format!("{prefix}fasses"),
        });
    }
    // Cherche IndPres 3pl dans le Lefff, puis dérive via la tige « gn »/« s »,
    // puis via la tige 1pl (pour les composés de -crire/-lire/-dire, etc.).
    let key = (lemma.to_string(), MoodTense::IndicativePresent.code(), 3u8, 2u8);
    // Cherche ou dérive IndPres 3pl (terminaison « -ent » obligatoire)
    let ind3pl_candidate = index.get(&key).cloned();
    let ind3pl = if let Some(f) = ind3pl_candidate.filter(|f| f.ends_with("ent")) {
        f
    } else {
        let sg3_key = (lemma.to_string(), MoodTense::IndicativePresent.code(), 3u8, 1u8);
        if let Some(sg3) = index.get(&sg3_key) {
            if let Some(gn_stem) = ind_pres_gn_stem(lemma, sg3) {
                format!("{gn_stem}ent")
            } else if let Some(stem) = ind_pres_1pl_stem(lemma, index) {
                format!("{stem}ent")
            } else { return None; }
        } else if let Some(stem) = ind_pres_1pl_stem(lemma, index) {
            format!("{stem}ent")
        } else { return None; }
    };
    if !ind3pl.ends_with("ent") {
        return None;
    }
    let stem = &ind3pl[..ind3pl.len() - 3];
    Some(match person {
        First | Third => format!("{stem}e"),
        Second        => format!("{stem}es"),
    })
}

fn instance() -> &'static Morphology {
    static INSTANCE: OnceLock<Morphology> = OnceLock::new();
    INSTANCE.get_or_init(|| Morphology::load().expect("FST morphologique embarqué invalide"))
}

/// Recherche les analyses morphologiques d'une forme (insensible à la casse).
///
/// Retourne un vecteur vide si la forme est inconnue.
pub fn lookup(form: &str) -> Vec<Morph> {
    instance().lookup(form)
}

/// Analyses verbales **finies** d'une forme (indicatif, subjonctif,
/// conditionnel, impératif), avec mode/temps, personne et nombre.
pub fn verb_forms(form: &str) -> Vec<VerbForm> {
    instance().verb_forms(form)
}

/// Engendre la forme d'un lemme verbal pour un mode/temps, une personne et un
/// nombre donnés (ex. `conjugate("manger", IndicativePresent, Third, Plural)`
/// → `Some("mangent")`). Renvoie `None` si la forme est introuvable.
pub fn conjugate(lemma: &str, mt: MoodTense, person: Person, number: Number) -> Option<String> {
    instance().conjugate(lemma, mt, person, number)
}

/// Engendre la forme accordée d'un adjectif pour un genre et un nombre donnés
/// (ex. `decline("content", Feminine, Singular)` → `Some("contente")`).
///
/// Les adjectifs épicènes ou invariables en nombre sont gérés par repli :
/// `decline("rouge", Feminine, Plural)` → `Some("rouges")`. Renvoie `None` si
/// aucune forme n'est trouvée pour ce lemme.
pub fn decline(lemma: &str, gender: Gender, number: Number) -> Option<String> {
    instance().decline(lemma, gender, number)
}

/// Engendre la forme accordée d'un **participe passé** pour un genre et un
/// nombre donnés (ex. `participle("partir", Feminine, Singular)` → `Some("partie")`,
/// `participle("manger", Feminine, Singular)` → `Some("mangée")`). Le `lemma`
/// est l'infinitif. Renvoie `None` si la forme est introuvable.
pub fn participle(lemma: &str, gender: Gender, number: Number) -> Option<String> {
    instance().participle(lemma, gender, number)
}

/// Fréquence lexicale relative d'une forme (occurrences/million × 100, d'après
/// Lexique383). Vaut 0 pour une forme absente du lexique de fréquences.
///
/// Utilisée notamment pour départager les suggestions orthographiques.
pub fn frequency(form: &str) -> u64 {
    instance().frequency(form)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(form: &str, cat: MorphCategory) -> bool {
        lookup(form).iter().any(|m| m.category == cat)
    }

    #[test]
    fn noun_features() {
        // NB : Lexique383 laisse le genre vide pour certains noms courants
        // (« maison », « voiture »…) ; on teste sur un nom au genre renseigné.
        let analyses = lookup("table");
        let noun = analyses
            .iter()
            .find(|m| m.category == MorphCategory::Noun)
            .expect("« table » devrait être un nom");
        assert_eq!(noun.gender, Some(Gender::Feminine));
        assert_eq!(noun.number, Some(Number::Singular));
        assert_eq!(noun.lemma, "table");
    }

    #[test]
    fn plural_noun() {
        let analyses = lookup("chats");
        assert!(analyses
            .iter()
            .any(|m| m.category == MorphCategory::Noun && m.number == Some(Number::Plural)));
    }

    #[test]
    fn determiner_and_verb() {
        assert!(has("une", MorphCategory::Determiner));
        assert!(has("les", MorphCategory::Determiner));
        assert!(has("mange", MorphCategory::Verb));
    }

    #[test]
    fn unknown_form_is_empty() {
        assert!(lookup("xyzzyqwf").is_empty());
    }

    #[test]
    fn verb_form_features() {
        let forms = verb_forms("mangent");
        assert!(forms.iter().any(|v| {
            v.lemma == "manger"
                && v.mood_tense == MoodTense::IndicativePresent
                && v.person == Person::Third
                && v.number == Number::Plural
        }));
        // « mange » peut être 1re ou 3e du singulier (indicatif présent).
        let forms = verb_forms("mange");
        assert!(forms
            .iter()
            .any(|v| v.person == Person::First && v.number == Number::Singular));
        assert!(forms
            .iter()
            .any(|v| v.person == Person::Third && v.number == Number::Singular));
    }

    #[test]
    fn conjugation_generation() {
        assert_eq!(
            conjugate(
                "manger",
                MoodTense::IndicativePresent,
                Person::Third,
                Number::Plural
            )
            .as_deref(),
            Some("mangent")
        );
        assert_eq!(
            conjugate(
                "être",
                MoodTense::IndicativePresent,
                Person::Third,
                Number::Plural
            )
            .as_deref(),
            Some("sont")
        );
        assert_eq!(
            conjugate(
                "manger",
                MoodTense::IndicativePresent,
                Person::Second,
                Number::Singular
            )
            .as_deref(),
            Some("manges")
        );
    }

    #[test]
    fn adjective_declension() {
        assert_eq!(
            decline("content", Gender::Feminine, Number::Singular).as_deref(),
            Some("contente")
        );
        assert_eq!(
            decline("content", Gender::Feminine, Number::Plural).as_deref(),
            Some("contentes")
        );
        assert_eq!(
            decline("content", Gender::Masculine, Number::Plural).as_deref(),
            Some("contents")
        );
        // Adjectif épicène : invariable en genre, mais varie en nombre.
        assert_eq!(
            decline("rouge", Gender::Feminine, Number::Plural).as_deref(),
            Some("rouges")
        );
    }

    #[test]
    fn past_participle_generation() {
        assert_eq!(
            participle("partir", Gender::Feminine, Number::Singular).as_deref(),
            Some("partie")
        );
        assert_eq!(
            participle("manger", Gender::Feminine, Number::Singular).as_deref(),
            Some("mangée")
        );
        assert_eq!(
            participle("aller", Gender::Masculine, Number::Plural).as_deref(),
            Some("allés")
        );
        assert_eq!(
            participle("venir", Gender::Feminine, Number::Plural).as_deref(),
            Some("venues")
        );
    }

    #[test]
    fn morph_builder() {
        let m = Morph::new("chat", MorphCategory::Noun);
        assert_eq!(m.lemma, "chat");
        assert_eq!(m.category, MorphCategory::Noun);
        assert!(m.gender.is_none());
    }

    #[test]
    fn patch_conjugate_covers_lefff_gaps() {
        assert_eq!(
            conjugate("aller", MoodTense::IndicativePast, Person::Second, Number::Plural).as_deref(),
            Some("allâtes")
        );
        assert_eq!(
            conjugate("venir", MoodTense::IndicativePast, Person::First, Number::Plural).as_deref(),
            Some("vînmes")
        );
        assert_eq!(
            conjugate("devoir", MoodTense::IndicativePast, Person::Second, Number::Singular).as_deref(),
            Some("dus")
        );
        assert_eq!(
            conjugate("avoir", MoodTense::IndicativePast, Person::Second, Number::Singular).as_deref(),
            Some("eus")
        );
        assert_eq!(
            conjugate("prendre", MoodTense::SubjunctivePresent, Person::Third, Number::Plural).as_deref(),
            Some("prennent")
        );
    }
}
