//! Règle : confusion **sans / s'en** (/ c'en) — **tranche 4** du moteur de
//! confusions de la phase 6 (cf.
//! [`corpus/confusion-sans-sen.md`](../../../../../corpus/confusion-sans-sen.md)).
//!
//! Adossée au CRF ([`crate::pos`]). Mémo (Projet Voltaire) :
//! - **sans** = préposition de privation (« **sans** toi », « **sans** manger ») ;
//! - **s'en** = pronom réfléchi « se » + « en », **préverbal** (« il **s'en**
//!   va », « elle **s'en** souvient ») ;
//! - **c'en** = « ce » + « en », quasi exclusivement « **c'en** est… » (rare).
//!
//! ## sans → s'en (« il sans va » → « il **s'en** va »)
//!
//! La préposition « sans » ne s'insère jamais entre un **sujet de 3ᵉ personne**
//! (`il/elle/on/ils/elles`, relatif `qui`, ou un nom) et un **verbe conjugué**
//! (« ne » sauté). Coincé là, « sans » est le pronominal « s'en ».
//!
//! ## s'en → sans (« réussir s'en effort » → « réussir **sans** effort »)
//!
//! Inversement, « s'en » (élision « s' » + « en ») suivi d'un **nom ou d'un
//! adjectif** (étiquette CRF) est impossible : « en » y est pronom et réclame un
//! verbe. C'est la préposition « sans ». On fusionne les deux jetons en « sans ».
//!
//! Limite assumée : **c'en** (« c'en est trop ») est trop rare pour un signal
//! fiable et n'est pas traité.

use super::{is_finite_verb, is_infinitive, normalize, upos};
use crate::morpho::{self, MorphCategory};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Span;
use crate::Suggestion;

/// Détecte les confusions « sans » / « s'en ».
pub struct SansConfusion;

const RULE_ID: &str = "confusion_sans";

/// Sujets de 3ᵉ personne (pronoms forts) licenciant « s'en ».
const THIRD_PERSON_PRONOUNS: &[&str] = &["il", "elle", "on", "ils", "elles"];

/// Vrai si le jeton est une élision (se termine par une apostrophe).
fn is_elided(text: &str) -> bool {
    text.ends_with('\'') || text.ends_with('\u{2019}')
}

/// Vrai si `form` est une tête **nominale** (nom/adjectif) qui n'est **pas**
/// aussi un verbe (fini ou infinitif). À la Grammalecte, on consulte les lectures
/// possibles du lexique plutôt que l'unique tag CRF, souvent leurré par l'erreur
/// (« s'en effort » fait étiqueter « effort » VERB). On écarte les homographes
/// nom/verbe (« il s'en va », « de s'en aller ») où « s'en » est légitime.
fn is_nominal_head(form: &str) -> bool {
    let has_nominal = morpho::lookup(form)
        .iter()
        .any(|m| matches!(m.category, MorphCategory::Noun | MorphCategory::Adjective));
    has_nominal && !is_finite_verb(form) && !is_infinitive(form)
}

/// Position du sujet candidat à gauche de `i`, en sautant la négation « ne »/« n' ».
fn subject_before(sentence: &[(usize, &Token)], i: usize) -> Option<usize> {
    let mut k = i;
    while k > 0 {
        k -= 1;
        match normalize(sentence[k].1.text.as_str()).as_str() {
            "ne" | "n" => continue,
            _ => return Some(k),
        }
    }
    None
}

/// Correction d'un « sans » (mot plein) en « s'en » : sujet 3ᵉ pers. + sans +
/// verbe conjugué.
fn correction_sans(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<Suggestion> {
    let next_is_verb = sentence
        .get(i + 1)
        .is_some_and(|(_, t)| is_finite_verb(&t.text));
    if !next_is_verb {
        return None;
    }
    let subj = subject_before(sentence, i)?;
    let subj_norm = normalize(sentence[subj].1.text.as_str());
    let is_third = THIRD_PERSON_PRONOUNS.contains(&subj_norm.as_str())
        || subj_norm == "qui"
        || matches!(upos(sentence, subj, tags), Upos::Noun | Upos::Propn);
    if !is_third {
        return None;
    }
    let token = sentence[i].1;
    Some(Suggestion {
        span: token.span,
        message: format!(
            "Confusion « sans »/« s'en » : « {} » devrait être « s'en ».",
            token.text
        ),
        replacements: vec![super::match_case(&token.text, "s'en")],
        rule_id: RULE_ID,
    })
}

/// Correction de l'élision « s' » + « en » en « sans » : devant un nom/adjectif.
/// Fusionne les deux jetons (« s' en » → « sans »).
fn correction_sen(sentence: &[(usize, &Token)], i: usize) -> Option<Suggestion> {
    let cur = sentence[i].1;
    if !is_elided(&cur.text) || normalize(&cur.text) != "s" {
        return None;
    }
    let (_, en_tok) = sentence.get(i + 1)?;
    if normalize(&en_tok.text) != "en" {
        return None;
    }
    // Le jeton suivant « en » doit être une tête nominale : « en » pronom
    // appellerait un verbe, jamais un nom.
    let (_, head) = sentence.get(i + 2)?;
    if !is_nominal_head(&head.text) {
        return None;
    }
    let span = Span::new(cur.span.start, en_tok.span.end);
    let original = format!("{}{}", cur.text, en_tok.text);
    Some(Suggestion {
        span,
        message: "Confusion « sans »/« s'en » : « s'en » devrait être « sans ».".to_string(),
        replacements: vec![super::match_case(&original, "sans")],
        rule_id: RULE_ID,
    })
}

impl Rule for SansConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let s = match normalize(sentence[i].1.text.as_str()).as_str() {
                    "sans" => correction_sans(&sentence, i, tags),
                    "s" => correction_sen(&sentence, i),
                    _ => None,
                };
                if let Some(s) = s {
                    suggestions.push(s);
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « sans » / « s'en »"
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
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SansConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SansConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- sans → s'en ---

    #[test]
    fn sans_to_sen_between_subject_and_verb() {
        assert_eq!(first("il sans va sans rien dire").as_deref(), Some("s'en"));
        assert_eq!(first("elle sans souvient encore").as_deref(), Some("s'en"));
        assert_eq!(first("ils sans vont demain").as_deref(), Some("s'en"));
        assert_eq!(first("il ne sans souvient pas").as_deref(), Some("s'en"));
    }

    #[test]
    fn sans_to_sen_with_nominal_subject() {
        assert_eq!(
            first("le voleur sans alla aussitôt").as_deref(),
            Some("s'en")
        );
    }

    // --- s'en → sans ---

    #[test]
    fn sen_to_sans_before_noun() {
        assert_eq!(first("il a réussi s'en effort").as_deref(), Some("sans"));
        assert_eq!(
            first("il agit s'en scrupule aucun").as_deref(),
            Some("sans")
        );
    }

    #[test]
    fn sen_to_sans_before_adjective() {
        assert_eq!(first("un repas s'en gros sel").as_deref(), Some("sans"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "il s'en va sans rien dire", // « s'en » + verbe correct
            "elle s'en souvient",        // « s'en » correct
            "il part sans toi",          // « sans » + pronom correct
            "il réussit sans effort",    // « sans » + nom correct
            "manger sans pain",          // « sans » + nom correct
            "il décide de s'en aller",   // « s'en » + infinitif correct
            "sans le savoir il a gagné", // « sans » en tête correct
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Il sans va").as_deref(), Some("s'en"));
        assert_eq!(first("S'en effort").as_deref(), Some("Sans"));
    }
}
