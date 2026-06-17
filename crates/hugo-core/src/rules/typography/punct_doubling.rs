//! Règle : **doublons de ponctuation**.
//!
//! Une suite d'un **même** signe parmi `, ; ! ?` (et `..`, faute de frappe d'un
//! point) est ramenée à un signe unique : `!!` → `!`, `,,` → `,`, `..` → `.`.
//!
//! Gardes :
//! - les suites de **trois points ou plus** relèvent des
//!   [points de suspension](super::ellipsis) (`…`), pas du doublon ;
//! - on n'agit que sur des signes **identiques contigus** : `!?` et `?!`
//!   (signes différents) sont des combinaisons légitimes et ne déclenchent rien.

use super::super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::{Span, Suggestion};

const RULE_ID: &str = "typo_punct_doubling";

/// Signes dont la répétition immédiate est fautive.
const DOUBLABLE: &[&str] = &[",", ";", "!", "?", "."];

/// Cf. [module](self).
pub struct PunctDoublingRule;

impl Rule for PunctDoublingRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            let t = &tokens[i];
            if t.kind == TokenKind::Punctuation && DOUBLABLE.contains(&t.text.as_str()) {
                let sign = t.text.as_str();
                let start = i;
                while i < tokens.len()
                    && tokens[i].kind == TokenKind::Punctuation
                    && tokens[i].text == sign
                {
                    i += 1;
                }
                let count = i - start;
                // « ... » (3+ points) : laissé aux points de suspension.
                let is_ellipsis = sign == "." && count >= 3;
                if count >= 2 && !is_ellipsis {
                    suggestions.push(Suggestion {
                        span: Span::new(tokens[start].span.start, tokens[i - 1].span.end),
                        message: format!("Ponctuation redoublée : « {sign} » suffit."),
                        replacements: vec![sign.to_string()],
                        rule_id: RULE_ID,
                    });
                }
            } else {
                i += 1;
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Doublon de ponctuation"
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
        PunctDoublingRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    #[test]
    fn doubled_signs_are_collapsed() {
        assert_eq!(repls("quoi!!"), vec!["!"]);
        assert_eq!(repls("vraiment??"), vec!["?"]);
        assert_eq!(repls("mot,,suite"), vec![","]);
        assert_eq!(repls("fin.."), vec!["."]);
    }

    #[test]
    fn ellipsis_is_not_a_doubling() {
        assert!(repls("attends...").is_empty());
    }

    #[test]
    fn mixed_signs_are_legitimate() {
        assert!(repls("quoi !? vraiment ?!").is_empty());
    }

    #[test]
    fn single_sign_untouched() {
        assert!(repls("oui ! non ?").is_empty());
    }
}
