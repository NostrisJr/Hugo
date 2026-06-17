//! Règle : majuscule en début de phrase, après une ponctuation terminale.

use super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Signale un mot en minuscule juste après une ponctuation terminale
/// (`.`, `!`, `?`) et propose sa capitalisation.
///
/// Ne se déclenche pas :
/// - après une abréviation courante (`M.`, `Mme`, `etc.`…) ni après un point
///   appartenant à un nombre décimal ;
/// - à l'intérieur d'une parenthèse (« le projet (livré tard !) avance ») : la
///   phrase porteuse se poursuit après la parenthèse ;
/// - après des **points de suspension** suivis d'une **minuscule** : « … » est
///   un terminateur ambigu ; une minuscule signale une suspension intra-phrase
///   (« il hésita… puis partit »), pas une nouvelle phrase. On présume alors la
///   continuation (précision > rappel).
pub struct CapitalizationAfterPeriod;

const RULE_ID: &str = "capitalization_after_period";

/// Terminateurs **fermes** : imposent une majuscule au mot suivant.
const HARD_TERMINAL_PUNCT: &[&str] = &[".", "!", "?"];

/// Points de suspension : terminateur **ambigu** (cf. [`Pending::Ellipsis`]).
const ELLIPSIS: &str = "…";

/// État de fin de phrase « en attente » d'un mot.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// Aucun terminateur en attente.
    None,
    /// Terminateur ferme (`.`/`!`/`?`) : une minuscule suivante est fautive.
    Hard,
    /// Points de suspension : une minuscule suivante est présumée continuation.
    Ellipsis,
}

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
        // Terminateur « en attente » d'un mot (cf. [`Pending`]).
        let mut pending = Pending::None;
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
                    if paren_depth == 0 {
                        if HARD_TERMINAL_PUNCT.contains(&token.text.as_str()) {
                            // Ne pas déclencher si le mot précédent est une
                            // abréviation courante.
                            let after_abbrev =
                                last_word.map(|w| is_abbreviation(&w.text)).unwrap_or(false);
                            pending = if after_abbrev {
                                Pending::None
                            } else {
                                Pending::Hard
                            };
                        } else if token.text == ELLIPSIS {
                            pending = Pending::Ellipsis;
                        }
                    }
                    // Toute autre ponctuation laisse l'état inchangé.
                }
                TokenKind::Word | TokenKind::Elision | TokenKind::Number => {
                    // Seul un terminateur ferme impose une majuscule ; après des
                    // points de suspension, une minuscule est une continuation.
                    if pending == Pending::Hard {
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
                    }
                    pending = Pending::None;
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
    fn test_ellipsis_followed_by_lowercase_is_continuation() {
        // « … » est un terminateur ambigu : suivi d'une minuscule, c'est une
        // suspension intra-phrase (continuation), pas une nouvelle phrase. On
        // n'exige donc pas de majuscule (précision > rappel).
        for text in [
            "il hésita… puis partit",
            "j'ai des pommes, des poires… et des bananes",
            "Bref… voici la suite",
        ] {
            let tokens = tokenize(text);
            let suggestions = CapitalizationAfterPeriod.check(&tokens);
            assert!(
                suggestions.is_empty(),
                "faux positif sur « {text} » : {:?}",
                suggestions
                    .iter()
                    .map(|s| &s.replacements)
                    .collect::<Vec<_>>()
            );
        }
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
