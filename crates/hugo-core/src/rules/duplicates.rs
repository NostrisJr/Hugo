//! Règle : détection de mots doublés (`il il` → `il`).

use super::{lexical_tokens, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte deux mots identiques consécutifs (à la casse près) et propose de
/// supprimer le doublon.
///
/// Les mots d'un seul caractère sont ignorés (`a a`, `y y`) car ils mènent
/// souvent à des faux positifs (« a a-t-il », initiales…). La comparaison est
/// insensible à la casse : « Le le » est bien un doublon.
pub struct DuplicateWord;

const RULE_ID: &str = "duplicate_word";

impl Rule for DuplicateWord {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let lex = lexical_tokens(tokens);
        let mut suggestions = Vec::new();

        for pair in lex.windows(2) {
            let (_, prev) = pair[0];
            let (_, cur) = pair[1];

            // On ne compare que des mots pleins, pas les élisions ni les mots
            // d'une seule lettre.
            if prev.text.chars().count() <= 1 {
                continue;
            }

            if prev.text.eq_ignore_ascii_case_unicode(&cur.text) {
                suggestions.push(Suggestion {
                    span: crate::Span::new(cur.span.start, cur.span.end),
                    message: format!("Mot répété : « {} ».", cur.text),
                    replacements: vec![String::new()],
                    rule_id: RULE_ID,
                });
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Mot doublé"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

/// Petite extension pour comparer deux chaînes sans tenir compte de la casse,
/// y compris pour les caractères accentués (contrairement à
/// `eq_ignore_ascii_case`).
trait UnicodeCaseEq {
    fn eq_ignore_ascii_case_unicode(&self, other: &str) -> bool;
}

impl UnicodeCaseEq for str {
    fn eq_ignore_ascii_case_unicode(&self, other: &str) -> bool {
        self.to_lowercase() == other.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_duplicate_word() {
        let tokens = tokenize("il il mange");
        let rule = DuplicateWord;
        let suggestions = rule.check(&tokens);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].rule_id, "duplicate_word");
    }

    #[test]
    fn test_no_false_positive() {
        let tokens = tokenize("il mange il dort");
        let rule = DuplicateWord;
        let suggestions = rule.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_case_insensitive() {
        let tokens = tokenize("Le le chat");
        let suggestions = DuplicateWord.check(&tokens);
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn test_single_char_ignored() {
        // « a a » ne doit pas être signalé (mots d'une lettre).
        let tokens = tokenize("il y a a manger");
        let suggestions = DuplicateWord.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_accented_case() {
        let tokens = tokenize("Été été chaud");
        let suggestions = DuplicateWord.check(&tokens);
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn test_suggestion_span_points_to_second_word() {
        let input = "il il mange";
        let suggestions = DuplicateWord.check(&tokenize(input));
        let s = &suggestions[0];
        assert_eq!(&input[s.span.start..s.span.end], "il");
        // Le span doit viser le SECOND « il » (offset 3), pas le premier.
        assert_eq!(s.span.start, 3);
    }
}
