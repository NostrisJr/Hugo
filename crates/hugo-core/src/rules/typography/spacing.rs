//! Règle : **espaces surnuméraires et manquants**.
//!
//! Espaces **surnuméraires** :
//! - suite de plusieurs espaces ordinaires → une seule ;
//! - espace **avant** `,` `.` `)` → supprimée ;
//! - espace **après** `(` → supprimée.
//!
//! Espaces **manquants** :
//! - `,` ou `;` immédiatement collés au mot suivant → insertion d'une espace.
//!
//! Gardes (précision > rappel) : on n'agit que sur l'espace **ordinaire**
//! (U+0020), jamais sur les sauts de ligne ou tabulations ; les nombres décimaux
//! et les milliers (`3,14`, `12 000`) sont épargnés (virgule encadrée de
//! chiffres). Le `:` et le `.` (deux-points, point) ne sont **pas** traités pour
//! l'espace manquante — trop d'homographes en contexte technique (URL `http://`,
//! `fichier.txt`, code `a:b`) ; ils relèveront d'une passe ultérieure.

use super::super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

const RULE_ID: &str = "typo_space";

/// Signes devant lesquels aucune espace ne doit figurer.
const NO_SPACE_BEFORE: &[&str] = &[",", ".", ")"];

/// Signes après lesquels une espace manquante est fautive (cf. gardes module).
const SPACE_AFTER: &[&str] = &[",", ";"];

/// Signes qui réclament une espace **insécable** : la nature de l'espace qui les
/// précède relève de [`super::nbsp`], pas de la réduction des surnuméraires.
const NBSP_BEFORE: &[&str] = &[";", ":", "!", "?", "\u{00BB}"];

/// Cf. [module](self).
pub struct SpacingRule;

/// Vrai si le jeton est une suite d'espaces **ordinaires** (U+0020 uniquement).
fn is_plain_spaces(token: &Token) -> bool {
    token.kind == TokenKind::Whitespace
        && !token.text.is_empty()
        && token.text.bytes().all(|b| b == b' ')
}

fn is_punct(token: &Token, set: &[&str]) -> bool {
    token.kind == TokenKind::Punctuation && set.contains(&token.text.as_str())
}

impl Rule for SpacingRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            // --- Espaces surnuméraires (jetons d'espace) ---
            if is_plain_spaces(token) {
                let next_no_space = tokens
                    .get(i + 1)
                    .is_some_and(|t| is_punct(t, NO_SPACE_BEFORE));
                let prev_open_paren = i
                    .checked_sub(1)
                    .and_then(|p| tokens.get(p))
                    .is_some_and(|t| t.kind == TokenKind::Punctuation && t.text == "(");

                if next_no_space || prev_open_paren {
                    let message = if next_no_space {
                        "Pas d'espace avant ce signe de ponctuation.".to_string()
                    } else {
                        "Pas d'espace après une parenthèse ouvrante.".to_string()
                    };
                    suggestions.push(Suggestion {
                        span: token.span,
                        message,
                        replacements: vec![String::new()],
                        rule_id: RULE_ID,
                    });
                } else if token.text.len() > 1
                    && !tokens.get(i + 1).is_some_and(|t| is_punct(t, NBSP_BEFORE))
                {
                    suggestions.push(Suggestion {
                        span: token.span,
                        message: "Espaces surnuméraires : une seule espace suffit.".to_string(),
                        replacements: vec![" ".to_string()],
                        rule_id: RULE_ID,
                    });
                }
                continue;
            }

            // --- Espace manquante après « , » / « ; » ---
            if is_punct(token, SPACE_AFTER) {
                let Some(next) = tokens.get(i + 1) else {
                    continue;
                };
                if !matches!(next.kind, TokenKind::Word | TokenKind::Number) {
                    continue;
                }
                // Virgule encadrée de chiffres : nombre décimal / milliers.
                let prev_is_number = i
                    .checked_sub(1)
                    .and_then(|p| tokens.get(p))
                    .is_some_and(|t| t.kind == TokenKind::Number);
                if token.text == "," && prev_is_number && next.kind == TokenKind::Number {
                    continue;
                }
                suggestions.push(Suggestion {
                    span: token.span,
                    message: format!("Espace manquante après « {} ».", token.text),
                    replacements: vec![format!("{} ", token.text)],
                    rule_id: RULE_ID,
                });
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Espaces surnuméraires et manquants"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn repls(text: &str) -> Vec<(String, String)> {
        SpacingRule
            .check(&tokenize(text))
            .into_iter()
            .map(|s| {
                (
                    text[s.span.start..s.span.end].to_string(),
                    s.replacements[0].clone(),
                )
            })
            .collect()
    }

    #[test]
    fn collapses_double_space() {
        assert_eq!(repls("le  chat"), vec![("  ".to_string(), " ".to_string())]);
    }

    #[test]
    fn removes_space_before_comma_and_paren() {
        assert_eq!(
            repls("le chat , lui"),
            vec![(" ".to_string(), String::new())]
        );
        assert_eq!(repls("fin )"), vec![(" ".to_string(), String::new())]);
    }

    #[test]
    fn removes_space_after_open_paren() {
        assert_eq!(repls("( a)"), vec![(" ".to_string(), String::new())]);
    }

    #[test]
    fn inserts_missing_space_after_comma() {
        assert_eq!(
            repls("rouge,vert"),
            vec![(",".to_string(), ", ".to_string())]
        );
    }

    #[test]
    fn decimal_comma_is_spared() {
        assert!(repls("il a payé 3,14 euros").is_empty());
    }

    #[test]
    fn single_space_is_fine() {
        assert!(repls("le chat dort").is_empty());
    }

    #[test]
    fn newline_is_not_collapsed() {
        // Un saut de ligne suivi d'indentation n'est pas une espace surnuméraire.
        assert!(repls("ligne\n  suite").is_empty());
    }
}
