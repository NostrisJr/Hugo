//! Règle : majuscule en début de phrase, après une ponctuation terminale.

use super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Signale un mot en minuscule juste après une ponctuation terminale
/// (`.`, `!`, `?`, `…`) et propose sa capitalisation.
///
/// Ne se déclenche pas après une abréviation courante (`M.`, `Mme`, `etc.`…)
/// ni après un point appartenant à un nombre décimal.
pub struct CapitalizationAfterPeriod;

const RULE_ID: &str = "capitalization_after_period";

/// Ponctuations considérées comme terminant une phrase.
const TERMINAL_PUNCT: &[&str] = &[".", "!", "?", "…"];

/// Abréviations fréquentes après lesquelles le point ne termine pas la phrase.
/// Comparaison insensible à la casse.
const ABBREVIATIONS: &[&str] = &[
    "m", "mm", "mme", "mlle", "mr", "dr", "pr", "me", "etc", "cf", "ex", "p", "vol", "no", "art",
    "fig", "av", "bd", "st", "ste",
];

fn is_abbreviation(word: &str) -> bool {
    let lower = word.to_lowercase();
    ABBREVIATIONS.contains(&lower.as_str())
}

fn first_char_is_lower(word: &str) -> Option<char> {
    let c = word.chars().next()?;
    if c.is_alphabetic() && c.is_lowercase() {
        Some(c)
    } else {
        None
    }
}

impl Rule for CapitalizationAfterPeriod {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Index du dernier mot lexical vu avant la ponctuation courante.
        let mut last_word: Option<&Token> = None;
        // Vrai si une ponctuation terminale est « en attente » d'un mot.
        let mut terminal_pending = false;
        // Profondeur de parenthèses ouvertes : une ponctuation terminale à
        // l'intérieur d'une parenthèse (« (perte, vol, virus…) ») n'est pas une
        // fin de phrase — la phrase porteuse se poursuit après la parenthèse.
        let mut paren_depth: usize = 0;

        for token in tokens {
            match token.kind {
                TokenKind::Whitespace => {}
                TokenKind::Punctuation => {
                    match token.text.as_str() {
                        "(" => paren_depth += 1,
                        ")" => paren_depth = paren_depth.saturating_sub(1),
                        _ => {}
                    }
                    if paren_depth == 0 && TERMINAL_PUNCT.contains(&token.text.as_str()) {
                        // Ne pas déclencher si le mot précédent est une
                        // abréviation courante.
                        let after_abbrev =
                            last_word.map(|w| is_abbreviation(&w.text)).unwrap_or(false);
                        terminal_pending = !after_abbrev;
                    }
                    // Toute autre ponctuation laisse l'état inchangé.
                }
                TokenKind::Word | TokenKind::Elision | TokenKind::Number => {
                    if terminal_pending {
                        if let Some(c) = first_char_is_lower(&token.text) {
                            let mut corrected = String::new();
                            corrected.extend(c.to_uppercase());
                            corrected.push_str(&token.text[c.len_utf8()..]);
                            suggestions.push(Suggestion {
                                span: token.span,
                                message: format!(
                                    "Majuscule attendue en début de phrase : « {} ».",
                                    token.text
                                ),
                                replacements: vec![corrected],
                                rule_id: RULE_ID,
                            });
                        }
                        terminal_pending = false;
                    }
                    last_word = Some(token);
                }
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Majuscule en début de phrase"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_lowercase_after_period() {
        let tokens = tokenize("fin. il repart");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].replacements, vec!["Il"]);
        assert_eq!(suggestions[0].rule_id, "capitalization_after_period");
    }

    #[test]
    fn test_already_capitalized() {
        let tokens = tokenize("Fin. Il repart");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_abbreviation_not_triggered() {
        let tokens = tokenize("M. Dupont arrive");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_exclamation_and_question() {
        let tokens = tokenize("Quoi! tu pars? oui");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        // « tu » après « ! » et « oui » après « ? ».
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn test_accented_first_letter() {
        let tokens = tokenize("Fini. élève suivant");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].replacements, vec!["Élève"]);
    }

    #[test]
    fn test_etc_abbreviation() {
        let tokens = tokenize("des pommes, etc. puis on part");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_ellipsis_inside_parentheses_is_not_terminal() {
        // « … » dans une parenthèse ne ferme pas la phrase porteuse : « perdu »
        // ne réclame pas de majuscule.
        let tokens =
            tokenize("un employé ayant, pour X raison (perte, vol, virus…) perdu son poste");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_ellipsis_outside_parentheses_still_triggers() {
        // Hors parenthèse, « … » reste une fin de phrase.
        let tokens = tokenize("il hésita… puis partit");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].replacements, vec!["Puis"]);
    }

    #[test]
    fn test_terminal_punct_inside_parentheses_is_ignored() {
        // Un point/point d'exclamation à l'intérieur d'une parenthèse n'impose
        // pas de majuscule au mot qui suit la parenthèse.
        let tokens = tokenize("le projet (livré tard !) avance bien");
        let suggestions = CapitalizationAfterPeriod.check(&tokens);
        assert_eq!(suggestions.len(), 0);
    }
}
