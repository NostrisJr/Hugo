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

    /// Construit l'index des participes passés en parcourant le FST une fois.
    /// Un participe passé est un enregistrement verbal **sans personne** mais
    /// porteur d'un genre et d'un nombre (« mangée », « parties »).
    fn build_part(&self) -> HashMap<(String, u8, u8), String> {
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
                    && r.person == 0
                    && r.gender != 0
                    && r.number != 0
                {
                    index
                        .entry((r.lemma, r.gender, r.number))
                        .or_insert_with(|| form.to_string());
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
}
