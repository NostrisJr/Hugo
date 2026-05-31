//! Analyse morphologique.
//!
//! Ce module définit les structures décrivant les traits morphologiques d'une
//! forme fléchie (catégorie, genre, nombre, personne, lemme), ainsi que le
//! point d'entrée [`lookup`] qui — à terme — interrogera le FST compilé depuis
//! le Lefff (`lefff.fst`, ~8 MB, voir `tools/compile-lefff`).
//!
//! Pour l'instant, [`lookup`] est un **stub** : il renvoie toujours un résultat
//! vide. La structure est néanmoins figée pour que le moteur de règles puisse
//! déjà être écrit contre elle.

/// Catégorie grammaticale (partie du discours).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MorphCategory {
    /// Nom commun.
    Noun,
    /// Verbe.
    Verb,
    /// Adjectif.
    Adjective,
    /// Déterminant (article, possessif, démonstratif…).
    Determiner,
    /// Pronom.
    Pronoun,
    /// Adverbe.
    Adverb,
    /// Préposition.
    Preposition,
    /// Conjonction.
    Conjunction,
    /// Interjection.
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

/// Une analyse morphologique possible d'une forme fléchie.
///
/// Une même forme de surface peut admettre plusieurs `Morph` (ambiguïté), d'où
/// le `Vec<Morph>` renvoyé par [`lookup`].
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

/// Recherche les analyses morphologiques d'une forme.
///
/// **Stub** : renverra les entrées du FST Lefff une fois celui-ci compilé et
/// chargé. Aujourd'hui, retourne systématiquement `Vec::new()`.
pub fn lookup(_form: &str) -> Vec<Morph> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_stub_is_empty() {
        assert!(lookup("chat").is_empty());
    }

    #[test]
    fn morph_builder() {
        let m = Morph::new("chat", MorphCategory::Noun);
        assert_eq!(m.lemma, "chat");
        assert_eq!(m.category, MorphCategory::Noun);
        assert!(m.gender.is_none());
    }
}
