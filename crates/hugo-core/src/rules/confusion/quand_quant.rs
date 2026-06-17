//! Règle : confusion **quand / quant** (/ qu'en) — **tranche 4** du moteur de
//! confusions de la phase 6 (cf.
//! [`corpus/confusion-quand-quant.md`](../../../../../corpus/confusion-quand-quant.md)).
//!
//! Mémo (Projet Voltaire) :
//! - **quand** = conjonction/adverbe de **temps** (« **quand** il pleut »,
//!   « **quand** pars-tu ? ») ; remplaçable par « lorsque ».
//! - **quant** n'existe **que** dans la locution « **quant à / au / aux** »
//!   (« **quant à** moi ») ; remplaçable par « en ce qui concerne ».
//! - **qu'en** = « que/qu' » + « en » (« **qu'en** penses-tu ? »).
//!
//! ## quand → quant (« quand à moi » → « **quant à** moi »)
//!
//! « quand » immédiatement suivi de « à »/« au »/« aux » est en réalité « quant » :
//! la conjonction de temps n'introduit jamais ce complément. La direction est
//! **séparable** car « quand à/au/aux » est toujours fautif.
//!
//! ## quant → quand (« quant il pleut » → « **quand** il pleut »)
//!
//! Inversement, « quant » qui **n'est pas** suivi de « à »/« au »/« aux » est mal
//! orthographié : la locution « quant à » est sa seule emploi licite. On corrige
//! vers « quand » (la confusion la plus fréquente) ; le rare « qu'en » reste un
//! **gap structurel** (« quand »/« qu'en » sont tous deux suivis d'un verbe).
//!
//! Limite assumée : **qu'en** (« qu'en penses-tu » ↔ « quand penses-tu ») n'a pas
//! de signal séparable et n'est pas traité.

use super::{match_case, normalize};
use crate::pos::Tagged;
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Détecte les confusions « quand » / « quant ».
pub struct QuandConfusion;

const RULE_ID: &str = "confusion_quand";

/// Mots introduits par « quant » (et jamais par « quand ») : la locution figée.
const QUANT_TAILS: &[&str] = &["à", "au", "aux"];

/// Vrai si une ponctuation sépare les jetons d'origine `a` et `b` (a < b) dans la
/// tranche complète : on refuse alors le rattachement « quand » → « à » (« quand,
/// à Paris, … »).
fn punctuation_between(tokens: &[Token], a: usize, b: usize) -> bool {
    tokens[a + 1..b]
        .iter()
        .any(|t| t.kind == TokenKind::Punctuation)
}

/// Correction d'un « quand »/« quant » d'après le mot qui le suit.
fn correction(sentence: &[(usize, &Token)], tokens: &[Token], i: usize) -> Option<&'static str> {
    let next = sentence.get(i + 1);
    let next_is_tail =
        next.is_some_and(|(_, t)| QUANT_TAILS.contains(&normalize(&t.text).as_str()));

    match normalize(sentence[i].1.text.as_str()).as_str() {
        // quand → quant : « quand à/au/aux » (sans ponctuation intercalée).
        "quand" if next_is_tail => {
            let (next_idx, _) = next.unwrap();
            (!punctuation_between(tokens, sentence[i].0, *next_idx)).then_some("quant")
        }
        // quant → quand : « quant » non suivi de « à/au/aux ».
        "quant" if !next_is_tail => Some("quand"),
        _ => None,
    }
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « quand »/« quant » : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for QuandConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], _tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                if let Some(c) = correction(&sentence, tokens, i) {
                    suggestions.push(suggestion(sentence[i].1, c));
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « quand » / « quant »"
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
        QuandConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        QuandConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- quand → quant ---

    #[test]
    fn quand_to_quant_before_a() {
        assert_eq!(first("quand à moi je reste").as_deref(), Some("quant"));
        assert_eq!(first("quand au reste tant pis").as_deref(), Some("quant"));
        assert_eq!(
            first("quand aux enfants ils dorment").as_deref(),
            Some("quant")
        );
    }

    // --- quant → quand ---

    #[test]
    fn quant_to_quand_elsewhere() {
        assert_eq!(first("quant il pleut je lis").as_deref(), Some("quand"));
        assert_eq!(first("je rentre quant tu pars").as_deref(), Some("quand"));
        assert_eq!(first("quant viendras-tu").as_deref(), Some("quand"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "quand il pleut je lis",   // conjonction de temps
            "quand pars-tu",           // adverbe interrogatif
            "quant à moi je reste",    // locution correcte
            "quant au reste tant pis", // locution correcte
            "quant aux enfants",       // locution correcte
            "dis-moi quand tu veux",   // « quand » correct
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn comma_blocks_quand_to_quant() {
        // « quand, à Paris, … » : la virgule interdit le rattachement.
        assert_eq!(count("quand, à Paris, il arrivera"), 0);
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Quant il pleut je lis").as_deref(), Some("Quand"));
        assert_eq!(first("Quand à moi je reste").as_deref(), Some("Quant"));
    }
}
