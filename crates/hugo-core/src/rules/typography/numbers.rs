//! Règle : **abréviations ordinales**.
//!
//! Corrige les abréviations ordinales mal formées accolées à un nombre :
//! `1ère` → `1re`, `2ème` → `2e`, `2ième` → `2e`, et leurs pluriels (`2èmes` →
//! `2es`). Les abréviations correctes (`1er`, `1re`, `2e`, `2es`, `2d`, `2nd`…)
//! ne déclenchent rien.
//!
//! Le tokenizer scinde « 1ère » en un nombre (`1`) suivi d'un mot (`ère`) ; on
//! ne réécrit que le **suffixe**, lorsqu'il est immédiatement précédé d'un
//! nombre.

use super::super::Rule;
use super::recase;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

const RULE_ID: &str = "typo_ordinal";

/// Cf. [module](self).
pub struct OrdinalRule;

/// Suffixe ordinal mal formé (minuscule, sans accent normalisé) → forme correcte.
fn fix_suffix(lower: &str) -> Option<&'static str> {
    Some(match lower {
        "ère" | "ere" | "ière" | "iere" => "re",
        "ères" | "eres" | "ières" | "ieres" => "res",
        "ème" | "eme" | "ième" | "ieme" => "e",
        "èmes" | "emes" | "ièmes" | "iemes" => "es",
        _ => return None,
    })
}

impl Rule for OrdinalRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::Word {
                continue;
            }
            // Doit suivre immédiatement un nombre.
            let after_number = i
                .checked_sub(1)
                .and_then(|p| tokens.get(p))
                .is_some_and(|t| t.kind == TokenKind::Number);
            if !after_number {
                continue;
            }
            let Some(fixed) = fix_suffix(&token.text.to_lowercase()) else {
                continue;
            };
            suggestions.push(Suggestion {
                span: token.span,
                message: format!("Abréviation ordinale mal formée : « {} ».", token.text),
                replacements: vec![recase(&token.text, fixed)],
                rule_id: RULE_ID,
            });
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Abréviation ordinale"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn repls(text: &str) -> Vec<String> {
        OrdinalRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    #[test]
    fn malformed_ordinals_are_fixed() {
        assert_eq!(repls("la 1ère fois"), vec!["re"]);
        assert_eq!(repls("le 2ème jour"), vec!["e"]);
        assert_eq!(repls("le 2ième jour"), vec!["e"]);
        assert_eq!(repls("les 3èmes places"), vec!["es"]);
    }

    #[test]
    fn correct_ordinals_untouched() {
        assert!(repls("le 1er jour et la 1re fois").is_empty());
        assert!(repls("la 2e place, les 2es places").is_empty());
        assert!(repls("la 2nde guerre").is_empty());
    }

    #[test]
    fn suffix_not_after_number_is_ignored() {
        // « ère » mot autonome (« l'ère industrielle ») : pas précédé d'un nombre.
        assert!(repls("une ère nouvelle").is_empty());
    }
}
