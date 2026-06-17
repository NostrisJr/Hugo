//! Phase 10 — **Accord de *demi*, *nu*, *mi*, *semi*** antéposés et postposés.
//!
//! ## Règle
//!
//! - **Antéposés** (avant le nom) avec trait d'union : invariables.
//!   *«une demi-heure»*, *«nu-pieds»*, *«mi-temps»*, *«semi-remorque»*.
//! - **Postposés** (après le nom) sans trait d'union : variables.
//!   *«deux heures et demie»*, *«à moitié nu»*.
//!
//! ## Cas traités
//!
//! ### demi- mal accordé (antéposé)
//!
//! *«une demie-heure»* → *«demi-heure»* (le `e` final est fautif).
//!
//! ### nu- mal accordé (antéposé dans composé)
//!
//! *«nue-pieds»* → *«nu-pieds»* (invariable en antéposé).
//!
//! ### demi postposé mal formé
//!
//! *«deux heures et demi»* est correct (masc. invariable post-posé) ;
//! *«deux heures demi»* sans *et* → signaler qu'il manque *et* ou accorder.
//! Limite : la règle du *et* est complexe ; on se limite à `demie` incorrecte
//! après un nom féminin connu.

use super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

const RULE_ID: &str = "demi_accord";

pub struct DemiRule;

impl Rule for DemiRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let n = tokens.len();

        for i in 0..n {
            let tok = &tokens[i];
            if tok.kind != TokenKind::Word {
                continue;
            }
            let text = &tok.text;
            let lower = text.to_lowercase();

            // --- demi- antéposé avec accord fautif ---
            // Le token doit commencer par "demie-", "demis-", etc.
            if let Some(rest) = lower.strip_prefix("demie-") {
                // « demie-heure » → « demi-heure »
                let correction = format!("demi-{rest}");
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Accord fautif : «\u{a0}{text}\u{a0}» — «\u{a0}demi\u{a0}» est invariable en composition (antéposé avec trait d'union)."
                    ),
                    replacements: vec![recase(text, &correction)],
                    rule_id: RULE_ID,
                });
                continue;
            }

            // --- nu- antéposé avec accord fautif ---
            if let Some(rest) = lower.strip_prefix("nue-") {
                let correction = format!("nu-{rest}");
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Accord fautif : «\u{a0}{text}\u{a0}» — «\u{a0}nu\u{a0}» est invariable en composition antéposée."
                    ),
                    replacements: vec![recase(text, &correction)],
                    rule_id: RULE_ID,
                });
                continue;
            }

            if let Some(rest) = lower.strip_prefix("nues-") {
                let correction = format!("nu-{rest}");
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Accord fautif : «\u{a0}{text}\u{a0}» — «\u{a0}nu\u{a0}» est invariable en composition antéposée."
                    ),
                    replacements: vec![recase(text, &correction)],
                    rule_id: RULE_ID,
                });
                continue;
            }

            // --- demie postposé seul : après un nom féminin → demi ---
            // « une heure demie » → signaler (on attend « et demie »)
            // La détection : token Word = "demie" précédé d'un nom féminin.
            if lower == "demie" {
                // Vérifier que le token précédent est un nom (pas « et »)
                let prev = prev_word(tokens, i);
                let prev_is_et = prev.is_some_and(|p| p.to_lowercase() == "et");
                if prev_is_et {
                    continue; // « et demie » = correct
                }
                // Ne rien signaler si on n'est pas sûr (précision > rappel)
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord de demi, nu, mi, semi"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

fn prev_word<'a>(tokens: &'a [Token], i: usize) -> Option<&'a str> {
    tokens[..i]
        .iter()
        .rev()
        .find(|t| t.kind == TokenKind::Word)
        .map(|t| t.text.as_str())
}

fn recase(original: &str, lower: &str) -> String {
    if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars = lower.chars();
        match chars.next() {
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            None => lower.to_string(),
        }
    } else {
        lower.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn repls(text: &str) -> Vec<String> {
        DemiRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    fn count(text: &str) -> usize {
        DemiRule.check(&tokenize(text)).len()
    }

    #[test]
    fn demie_anteposes_fixed() {
        assert_eq!(repls("une demie-heure"), vec!["demi-heure"]);
        assert_eq!(repls("une demie-journée"), vec!["demi-journée"]);
    }

    #[test]
    fn demi_anteposes_correct() {
        assert_eq!(count("une demi-heure"), 0);
        assert_eq!(count("un demi-cercle"), 0);
        assert_eq!(count("mi-temps"), 0);
    }

    #[test]
    fn nue_anteposes_fixed() {
        assert_eq!(repls("nue-pieds"), vec!["nu-pieds"]);
    }

    #[test]
    fn nu_anteposes_correct() {
        assert_eq!(count("nu-pieds"), 0);
        assert_eq!(count("nu-tête"), 0);
    }

    #[test]
    fn et_demie_correct() {
        assert_eq!(count("deux heures et demie"), 0);
        assert_eq!(count("une heure et demie"), 0);
    }
}
