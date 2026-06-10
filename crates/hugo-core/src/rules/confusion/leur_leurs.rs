//! Règle : confusion **leur / leurs** — **tranche 3** du moteur de confusions de
//! la phase 6 (cf.
//! [`corpus/confusion-leur-leurs.md`](../../../../../corpus/confusion-leur-leurs.md)).
//!
//! Adossée au CRF ([`crate::pos`]). Mémo (Projet Voltaire) :
//! - **leur** devant un **verbe** est le pronom personnel (= « à eux »),
//!   **toujours invariable** : « je **leur** parle » (jamais « leurs »).
//! - **leur** / **leurs** devant un **nom** est le déterminant possessif, qui
//!   **s'accorde en nombre** avec le nom : « **leur** livre », « **leurs**
//!   livres ».
//!
//! ## leur → leurs (« leur livres » → « **leurs** livres »)
//!
//! Possessif devant un nom **pluriel** (étiqueté nom par le CRF, pluriel sans
//! lecture singulière au lexique ; adjectifs antéposés sautés).
//!
//! ## leurs → leur
//!
//! - devant un **verbe** (pronom invariable) : « je leurs parle » → « **leur**
//!   parle » ;
//! - possessif devant un nom **singulier** non ambigu : « leurs livre » →
//!   « **leur** livre ».
//!
//! Limite assumée : les noms invariables ou non marqués en nombre au lexique
//! (« prix », « livre »…) ne tranchent pas la direction du possessif singulier ;
//! on s'abstient alors plutôt que de sur-corriger.

use super::{is_finite_verb, match_case, normalize, upos};
use crate::morpho::{self, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions « leur » / « leurs ».
pub struct LeurConfusion;

const RULE_ID: &str = "confusion_leur";

/// Nombre tranché d'un nom d'après ses **lectures nominales** au lexique :
/// `Some(Plural)` s'il n'a qu'une lecture plurielle, `Some(Singular)` s'il n'a
/// qu'une lecture singulière, `None` si ambigu, non marqué ou non nominal.
fn noun_number(form: &str) -> Option<Number> {
    let numbers: Vec<Number> = morpho::lookup(form)
        .iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .filter_map(|m| m.number)
        .collect();
    if numbers.is_empty() {
        return None;
    }
    let all_plural = numbers.iter().all(|&n| n == Number::Plural);
    let all_singular = numbers.iter().all(|&n| n == Number::Singular);
    match (all_plural, all_singular) {
        (true, false) => Some(Number::Plural),
        (false, true) => Some(Number::Singular),
        _ => None,
    }
}

/// Position de la **tête nominale** à droite du possessif `i`, en sautant les
/// adjectifs antéposés (« leur grandes maisons » → tête « maisons »). Renvoie
/// `None` si l'on ne tombe pas immédiatement sur un nom (verbe, etc.).
fn noun_head(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<usize> {
    let mut k = i + 1;
    while k < sentence.len() && upos(sentence, k, tags) == Upos::Adj {
        k += 1;
    }
    (k < sentence.len() && upos(sentence, k, tags) == Upos::Noun).then_some(k)
}

/// Correction d'un « leur »/« leurs », d'après son emploi (pronom/possessif).
fn correction(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<&'static str> {
    let cur = normalize(sentence[i].1.text.as_str());
    let next = sentence.get(i + 1)?;

    // Pronom personnel devant un verbe : invariable → « leur ». Seul « leurs »
    // est à corriger (« leur » est déjà correct).
    if upos(sentence, i + 1, tags) == Upos::Verb && is_finite_verb(&next.1.text) {
        return (cur == "leurs").then_some("leur");
    }

    // Possessif : accord en nombre avec la tête nominale.
    let head = noun_head(sentence, i, tags)?;
    match noun_number(sentence[head].1.text.as_str()) {
        Some(Number::Plural) if cur == "leur" => Some("leurs"),
        Some(Number::Singular) if cur == "leurs" => Some("leur"),
        _ => None,
    }
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « leur »/« leurs » : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for LeurConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                if matches!(
                    normalize(sentence[i].1.text.as_str()).as_str(),
                    "leur" | "leurs"
                ) {
                    if let Some(c) = correction(&sentence, i, tags) {
                        suggestions.push(suggestion(sentence[i].1, c));
                    }
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « leur » / « leurs »"
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
        LeurConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        LeurConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- leur → leurs ---

    #[test]
    fn leur_to_leurs_before_plural_noun() {
        assert_eq!(first("leur livres sont neufs").as_deref(), Some("leurs"));
        assert_eq!(first("ils rangent leur affaires").as_deref(), Some("leurs"));
        assert_eq!(first("leur grandes maisons").as_deref(), Some("leurs"));
    }

    // --- leurs → leur ---

    #[test]
    fn leurs_to_leur_before_verb() {
        assert_eq!(first("je leurs parle souvent").as_deref(), Some("leur"));
        assert_eq!(first("il leurs donne un livre").as_deref(), Some("leur"));
    }

    #[test]
    fn leurs_to_leur_before_singular_noun() {
        assert_eq!(first("leurs maison est grande").as_deref(), Some("leur"));
        assert_eq!(first("ils aiment leurs enfant").as_deref(), Some("leur"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "leur livre est neuf",        // possessif singulier correct
            "leurs livres sont neufs",    // possessif pluriel correct
            "je leur parle",              // pronom invariable correct
            "il leur donne un cadeau",    // pronom invariable correct
            "leurs grandes maisons",      // possessif pluriel + adjectif
            "leur grande maison",         // possessif singulier + adjectif
            "ils rangent leurs affaires", // possessif pluriel correct
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn invariable_noun_is_a_known_gap() {
        // Noms non marqués/invariables en nombre au lexique : on s'abstient.
        assert_eq!(count("leur prix sont élevés"), 0);
        assert_eq!(count("leurs prix est élevé"), 0);
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Leur livres sont neufs").as_deref(), Some("Leurs"));
    }
}
