//! Règle : confusions **ni/n'y** et **si/s'y** — tranche 6.
//!
//! ## ni → n'y
//!
//! `ni` est une conjonction de négation (coordonne des éléments négatifs) ;
//! `n'y` est la contraction *ne* + *y* (pronom de lieu ou indirect).
//!
//! - *«il ni pense pas»* → *«il n'y pense pas»*
//! - *«je ni vais plus»* → *«je n'y vais plus»*
//!
//! Signal : pronom sujet + `ni` + verbe conjugué. La conjonction `ni` ne
//! s'insère jamais entre un sujet et son verbe.
//!
//! ## si → s'y
//!
//! `si` est une conjonction conditionnelle ou un adverbe ; `s'y` est la
//! contraction *se* + *y*.
//!
//! - *«je si rends»* → *«je m'y rends»* (1ᵉ pers. sg)
//! - *«il si rend»* → *«il s'y rend»* (3ᵉ pers. sg)
//!
//! Signal : pronom sujet + `si` + verbe conjugué dont le lemme admet un
//! pronominal réfléchi.
//!
//! Limite : `si` conditionnel (*«si il vient»*) est pris en charge par
//! [`crate::rules::elision`] ; ici on traite uniquement le `si` erroné entre
//! sujet et verbe.

use super::{is_finite_verb, normalize, upos};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct NiNyConfusion;

const RULE_ID: &str = "confusion_ni_ny";

/// Pronoms sujets (toutes personnes) licenciant n'y / s'y.
const SUBJECT_PRONOUNS: &[&str] = &["je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles"];

/// Pronoms sujets de 1ᵉ et 2ᵉ pers. → remplacer par m'y / t'y.
fn se_y_form(subject: &str) -> &'static str {
    match subject {
        "je" => "m'y",
        "tu" => "t'y",
        _ => "s'y",
    }
}

impl Rule for NiNyConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                let form = normalize(sentence[i].1.text.as_str());

                match form.as_str() {
                    // --- ni → n'y : sujet + ni + verbe ---
                    "ni" => {
                        let prev_is_subj = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            SUBJECT_PRONOUNS.contains(&prev.as_str())
                                || matches!(upos(&sentence, i - 1, tags), Upos::Noun | Upos::Propn)
                        };
                        let next_is_verb = sentence
                            .get(i + 1)
                            .is_some_and(|(_, t)| is_finite_verb(&t.text));
                        if prev_is_subj && next_is_verb {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}ni\u{a0}»/«\u{a0}n'y\u{a0}» : la conjonction «\u{a0}ni\u{a0}» ne s'insère pas entre un sujet et son verbe.".to_string(),
                                replacements: vec!["n'y".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- si → s'y / m'y / t'y : sujet + si + verbe ---
                    "si" => {
                        // Ne pas déclencher avant il/ils (géré par l'élision)
                        let next_is_il = sentence.get(i + 1).is_some_and(|(_, t)| {
                            matches!(normalize(&t.text).as_str(), "il" | "ils")
                        });
                        if next_is_il {
                            continue;
                        }
                        let prev_subject = if i > 0 {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            if SUBJECT_PRONOUNS.contains(&prev.as_str()) {
                                Some(prev)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let next_is_verb = sentence
                            .get(i + 1)
                            .is_some_and(|(_, t)| is_finite_verb(&t.text));
                        if let Some(subj) = prev_subject {
                            if next_is_verb {
                                let repl = se_y_form(&subj).to_string();
                                let tok = sentence[i].1;
                                suggestions.push(Suggestion {
                                    span: tok.span,
                                    message: format!(
                                        "Confusion «\u{a0}si\u{a0}»/«\u{a0}{repl}\u{a0}» : «\u{a0}si\u{a0}» ne s'insère pas entre un pronom sujet et son verbe."
                                    ),
                                    replacements: vec![repl],
                                    rule_id: RULE_ID,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusions « ni/n'y » et « si/s'y »"
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
        NiNyConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        NiNyConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn ni_to_ny_before_verb() {
        assert_eq!(first("il ni pense pas"), Some("n'y".into()));
        assert_eq!(first("je ni vais plus"), Some("n'y".into()));
    }

    #[test]
    fn ni_correct_conjunction() {
        // ni conjonction correcte
        assert_eq!(count("ni lui ni elle"), 0);
        assert_eq!(count("sans bruit ni fureur"), 0);
    }

    #[test]
    fn si_to_sy_before_verb() {
        assert_eq!(first("il si rend"), Some("s'y".into()));
        assert_eq!(first("je si rends"), Some("m'y".into()));
        assert_eq!(first("tu si mets"), Some("t'y".into()));
    }

    #[test]
    fn si_before_il_no_trigger() {
        // Géré par la règle d'élision
        assert_eq!(count("si il vient"), 0);
        assert_eq!(count("si ils arrivent"), 0);
    }

    #[test]
    fn si_conditional_correct() {
        assert_eq!(count("si tu viens demain"), 0);
        assert_eq!(count("si elle part"), 0);
    }
}
