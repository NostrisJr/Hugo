//! Règle : confusions **sa/ça**, **ma/m'a**, **ta/t'a** — tranche 6.
//!
//! Trois familles distinctes, toutes liées à la confusion entre un
//! déterminant/pronom et une contraction sujet+auxiliaire.
//!
//! ## sa → ça
//!
//! `sa` est un **déterminant possessif** féminin singulier ; `ça` est un
//! **pronom démonstratif** (= cela). Quand `sa` est suivi d'un verbe
//! conjugué (sans nom interposé), c'est forcément `ça` :
//! - *«sa ne va pas»* → *«ça ne va pas»*
//!
//! ## ça → sa (inverse rare)
//!
//! Moins fréquent ; non traité (précision > rappel).
//!
//! ## ma → m'a
//!
//! `ma` est un **déterminant possessif** féminin singulier ; `m'a` est la
//! contraction *me* + *a* (3ᵉ pers. avoir). Quand `ma` est précédé d'un
//! sujet de 3ᵉ personne et suivi d'un participe, c'est `m'a` :
//! - *«il ma dit»* → *«il m'a dit»*
//!
//! ## ta → t'a
//!
//! Idem pour *te* + *a* : *«il ta vu»* → *«il t'a vu»*.

use super::{is_past_participle, normalize, upos};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct SaCaConfusion;

const RULE_ID: &str = "confusion_sa_ca";

/// Pronoms sujets de 3ᵉ personne licenciant m'a / t'a.
const THIRD_PERSON: &[&str] = &["il", "elle", "on", "ils", "elles"];

/// Vrai si `form` est une forme verbale finie (conjuguée).
fn is_verb(form: &str) -> bool {
    super::is_finite_verb(form)
}

impl Rule for SaCaConfusion {
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
                    // --- sa → ça : suivi d'un verbe conjugué ---
                    "sa" => {
                        // Le prochain token doit être un verbe (ou « ne »)
                        let next_is_verb = sentence.get(i + 1).is_some_and(|(_, t)| {
                            let n = normalize(&t.text);
                            n == "ne" || n == "n" || is_verb(&t.text)
                        });
                        // Pas de nom entre sa et le verbe (sinon c'est le DET possessif)
                        let prev_is_verb_or_start = i == 0
                            || matches!(
                                upos(&sentence, i - 1, tags),
                                Upos::Verb | Upos::Aux | Upos::Punct | Upos::Adv
                            );
                        if next_is_verb && prev_is_verb_or_start {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}sa\u{a0}»/«\u{a0}ça\u{a0}» : possessif «\u{a0}sa\u{a0}» utilisé à la place du pronom «\u{a0}ça\u{a0}».".to_string(),
                                replacements: vec![super::match_case(&tok.text, "ça")],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- ma → m'a : sujet 3ᵉ pers. + ma + participe passé ---
                    "ma" => {
                        // Précédé d'un sujet de 3ᵉ personne
                        let subj_ok = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            THIRD_PERSON.contains(&prev.as_str())
                                || matches!(upos(&sentence, i - 1, tags), Upos::Noun | Upos::Propn)
                        };
                        // Suivi d'un participe passé : accepte aussi le tag CRF VERB
                        // (certains participes comme « dit » ne sont pas dans le lexique
                        // comme participes mais sont bien étiquetés VERB par le CRF).
                        let next_is_pp = sentence.get(i + 1).is_some_and(|(_, t)| {
                            is_past_participle(&t.text)
                                || matches!(upos(&sentence, i + 1, tags), Upos::Verb | Upos::Aux)
                        });
                        if subj_ok && next_is_pp {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}ma\u{a0}»/«\u{a0}m'a\u{a0}» : déterminant «\u{a0}ma\u{a0}» à la place de la contraction «\u{a0}m'a\u{a0}».".to_string(),
                                replacements: vec!["m'a".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- ta → t'a : idem avec te ---
                    "ta" => {
                        let subj_ok = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            THIRD_PERSON.contains(&prev.as_str())
                                || matches!(upos(&sentence, i - 1, tags), Upos::Noun | Upos::Propn)
                        };
                        let next_is_pp = sentence.get(i + 1).is_some_and(|(_, t)| {
                            is_past_participle(&t.text)
                                || matches!(upos(&sentence, i + 1, tags), Upos::Verb | Upos::Aux)
                        });
                        if subj_ok && next_is_pp {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}ta\u{a0}»/«\u{a0}t'a\u{a0}» : déterminant «\u{a0}ta\u{a0}» à la place de la contraction «\u{a0}t'a\u{a0}».".to_string(),
                                replacements: vec!["t'a".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusions « sa/ça », « ma/m'a », « ta/t'a »"
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
        SaCaConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SaCaConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn sa_to_ca_before_verb() {
        assert_eq!(first("sa ne va pas"), Some("ça".into()));
        assert_eq!(first("sa marche bien"), Some("ça".into()));
    }

    #[test]
    fn sa_correct_det() {
        // « sa » déterminant possessif correct
        assert_eq!(count("sa maison est belle"), 0);
        assert_eq!(count("j'aime sa voix"), 0);
    }

    #[test]
    fn ma_to_ma_contracted() {
        assert_eq!(first("il ma dit"), Some("m'a".into()));
        assert_eq!(first("elle ma vu"), Some("m'a".into()));
    }

    #[test]
    fn ma_correct_det() {
        // « ma » déterminant possessif correct
        assert_eq!(count("ma mère est là"), 0);
    }

    #[test]
    fn ta_to_ta_contracted() {
        assert_eq!(first("il ta vu"), Some("t'a".into()));
    }

    #[test]
    fn ta_correct_det() {
        assert_eq!(count("ta voiture est là"), 0);
    }
}
