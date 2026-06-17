//! Règle : **points de suspension**.
//!
//! Une suite de **trois points ou plus** (`...`, `....`) est remplacée par le
//! caractère unique points de suspension `…` (U+2026). Le tokenizer émettant un
//! jeton par point, on repère une suite de jetons « . » **contigus** (sans
//! espace entre eux).
//!
//! Le cas `..` (exactement deux points) n'est **pas** traité ici : il relève du
//! [doublon de ponctuation](super::punct_doubling) (faute de frappe d'un point
//! simple), tandis que trois points ou plus dénotent une suspension.

use super::super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::{Span, Suggestion};

const RULE_ID: &str = "typo_ellipsis";

/// Cf. [module](self).
pub struct EllipsisRule;

fn is_dot(token: &Token) -> bool {
    token.kind == TokenKind::Punctuation && token.text == "."
}

impl Rule for EllipsisRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if is_dot(&tokens[i]) {
                let start = i;
                while i < tokens.len() && is_dot(&tokens[i]) {
                    i += 1;
                }
                let count = i - start;
                if count >= 3 {
                    suggestions.push(Suggestion {
                        span: Span::new(tokens[start].span.start, tokens[i - 1].span.end),
                        message: "Points de suspension : utilisez le caractère « … ».".to_string(),
                        replacements: vec!["\u{2026}".to_string()],
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
        "Points de suspension"
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
        EllipsisRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    #[test]
    fn three_dots_become_ellipsis() {
        assert_eq!(repls("attends..."), vec!["\u{2026}"]);
        assert_eq!(repls("et puis.... voilà"), vec!["\u{2026}"]);
    }

    #[test]
    fn span_covers_whole_run() {
        let tokens = tokenize("fin...");
        let s = &EllipsisRule.check(&tokens)[0];
        assert_eq!(&"fin..."[s.span.start..s.span.end], "...");
    }

    #[test]
    fn two_dots_left_to_doubling_rule() {
        assert!(repls("fin..").is_empty());
    }

    #[test]
    fn single_dot_untouched() {
        assert!(repls("fin. Suite").is_empty());
    }

    #[test]
    fn spaced_dots_are_not_merged() {
        // « . . . » séparés par des espaces ne forment pas une suite contiguë.
        assert!(repls("a . b").is_empty());
    }
}
