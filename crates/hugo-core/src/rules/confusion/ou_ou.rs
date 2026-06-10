//! Règle : confusion **ou / où** — **tranche 3** du moteur de confusions de la
//! phase 6 (cf.
//! [`corpus/confusion-ou-ou.md`](../../../../../corpus/confusion-ou-ou.md)).
//!
//! Mémo (Projet Voltaire) : **ou** (sans accent) est la conjonction d'alternative
//! (= « ou bien ») ; **où** (avec accent) marque le **lieu** ou le **temps**
//! (« le jour **où** », « là **où** »).
//!
//! ## ou → où : le signal séparable
//!
//! La feuille de route notait qu'« ou/où » n'offre « pas de signal séparable ».
//! Il en existe un, étroit mais **fiable** : un **antécédent de lieu/temps**
//! (nom d'une liste fermée, ou un adverbe `là`/`ici`/`partout`) immédiatement
//! suivi de « ou » **puis d'un pronom sujet** (`je/tu/il…`) introduit une
//! **proposition relative** → « où » (« le jour ou je suis né » → « le jour
//! **où** je suis né »). L'alternative « ou » coordonne au contraire deux
//! groupes nominaux : « le jour **ou** la nuit » (suivi d'un déterminant, pas
//! d'un sujet) n'est donc **pas** touché.
//!
//! ## où → ou : gap **structurel** assumé
//!
//! La direction inverse (« thé où café » → « ou ») n'a pas de signal séparable :
//! « où » relatif/interrogatif peut être suivi d'un déterminant comme d'un sujet
//! (« le pays où les gens vivent »). On ne la traite pas (cf. corpus).

use super::{match_case, normalize};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte la confusion « ou » / « où » (direction ou → où).
pub struct OuConfusion;

const RULE_ID: &str = "confusion_ou";

/// Antécédents de **lieu/temps** : un « ou » qui les suit, devant un sujet, est
/// le relatif « où ». Liste **fermée** (reconstituée pour ce projet), gage de
/// précision. Inclut quelques adverbes de lieu (`là`, `ici`, `partout`).
const PLACE_TIME_ANTECEDENTS: &[&str] = &[
    "jour",
    "jours",
    "journée",
    "journées",
    "matin",
    "soir",
    "soirée",
    "nuit",
    "moment",
    "moments",
    "instant",
    "instants",
    "heure",
    "heures",
    "minute",
    "seconde",
    "an",
    "ans",
    "année",
    "années",
    "époque",
    "période",
    "semaine",
    "semaines",
    "mois",
    "siècle",
    "endroit",
    "endroits",
    "lieu",
    "lieux",
    "place",
    "ville",
    "villes",
    "pays",
    "région",
    "régions",
    "maison",
    "pièce",
    "salle",
    "coin",
    "cas",
    "fois",
    "âge",
    "là",
    "ici",
    "partout",
];

/// Pronoms sujets : « ou » suivi de l'un d'eux (après un antécédent de lieu/
/// temps) introduit une relative → « où ».
const SUBJECT_PRONOUNS: &[&str] = &[
    "je", "j", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles",
];

/// Correction d'un « ou » en « où » : antécédent de lieu/temps + ou + sujet.
fn correction(sentence: &[(usize, &Token)], i: usize) -> Option<&'static str> {
    if i == 0 {
        return None;
    }
    let prev = normalize(sentence[i - 1].1.text.as_str());
    let next = normalize(sentence.get(i + 1)?.1.text.as_str());
    (PLACE_TIME_ANTECEDENTS.contains(&prev.as_str()) && SUBJECT_PRONOUNS.contains(&next.as_str()))
        .then_some("où")
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « ou »/« où » : « {} » devrait être « {} » (lieu/temps).",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for OuConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                if normalize(sentence[i].1.text.as_str()) == "ou" {
                    if let Some(c) = correction(&sentence, i) {
                        suggestions.push(suggestion(sentence[i].1, c));
                    }
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « ou » / « où »"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn first(text: &str) -> Option<String> {
        OuConfusion
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        OuConfusion.check(&tokenize(text)).len()
    }

    // --- ou → où ---

    #[test]
    fn ou_to_grave_relative_after_place_time() {
        assert_eq!(first("le jour ou je suis né").as_deref(), Some("où"));
        assert_eq!(first("le pays ou nous vivons").as_deref(), Some("où"));
        assert_eq!(first("le moment ou tu es parti").as_deref(), Some("où"));
        assert_eq!(first("la ville ou il habite").as_deref(), Some("où"));
    }

    #[test]
    fn ou_to_grave_after_locative_adverb() {
        assert_eq!(first("là ou je vis, il pleut").as_deref(), Some("où"));
        assert_eq!(first("partout ou il passe").as_deref(), Some("où"));
    }

    #[test]
    fn lowercase_is_preserved() {
        // Le « ou » fautif est minuscule en milieu de phrase : « où » l'est aussi,
        // même après la majuscule de début portée par l'adverbe « Là ».
        assert_eq!(first("Là ou je vis").as_deref(), Some("où"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "le jour ou la nuit",         // alternative (suivi d'un déterminant)
            "tu veux du thé ou du café",  // alternative
            "le jour où je suis né",      // « où » déjà correct
            "rouge ou vert",              // alternative simple
            "il vient ou il reste",       // coordination de propositions
            "le pays où les gens vivent", // « où » relatif correct (gap où→ou)
            "vrai ou faux",               // alternative
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn ou_grave_to_ou_is_a_known_gap() {
        // « où » → « ou » n'a pas de signal séparable : non traité.
        assert_eq!(count("tu préfères le thé où le café"), 0);
    }
}
