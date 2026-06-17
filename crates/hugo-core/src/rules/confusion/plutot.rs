//! Règle : confusions de **mots composés soudés** — tranche 6.
//!
//! Plusieurs adverbes ou locutions existent en **version soudée** (sens
//! particulier) et en **version séparée** (sens littéral). La règle traite
//! les cas où la version soudée est utilisée à la place de la version séparée
//! ou vice-versa.
//!
//! ## Familles traitées
//!
//! | Soudé | Séparé | Distinction |
//! |---|---|---|
//! | `plutôt` (de préférence) | `plus tôt` (avant le moment prévu) | tester avec «plutôt que»/«de préférence» |
//! | `bientôt` (prochainement) | `bien tôt` (très tôt) | rare en séparé |
//! | `aussitôt` (immédiatement) | `aussi tôt` (aussi early) | suivi de «que» → soudé |
//! | `davantage` (plus) | `d'avantage` (de l'avantage) | davantage ne prend jamais d' |
//!
//! ## plutôt → plus tôt
//!
//! *«il est arrivé plutôt que prévu»* → *«plus tôt que prévu»* (sens temporel).
//!
//! Signal : `plutôt` + `que` → sens temporel → `plus tôt que`.
//! Dans tous les autres contextes, `plutôt` seul est correct.
//!
//! ## plus tôt → plutôt
//!
//! *«je préfère plus tôt ça»* → `plutôt`. Signal difficile ; non traité.
//!
//! ## d'avantage → davantage
//!
//! *«j'en veux d'avantage»* → *«davantage»* : `d'` + `avantage` fusionnés.
//! La forme correcte ne prend jamais d'apostrophe.

use super::normalize;
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Span;
use crate::Suggestion;

pub struct PlutotConfusion;

const RULE_ID: &str = "confusion_plutot";

impl Rule for PlutotConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                let form = normalize(sentence[i].1.text.as_str());
                match form.as_str() {
                    // --- plutôt + que → plus tôt que ---
                    "plutôt" | "plutot" => {
                        let next_is_que = sentence
                            .get(i + 1)
                            .is_some_and(|(_, t)| normalize(&t.text) == "que");
                        if next_is_que {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}plutôt\u{a0}»/«\u{a0}plus tôt\u{a0}» : devant «\u{a0}que\u{a0}» de comparaison temporelle, on écrit «\u{a0}plus tôt\u{a0}».".to_string(),
                                replacements: vec!["plus tôt".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- d' + avantage → davantage ---
                    "d" => {
                        // Token élidé « d' » suivi de « avantage »
                        let next_is_avantage = sentence
                            .get(i + 1)
                            .is_some_and(|(_, t)| normalize(&t.text) == "avantage");
                        if next_is_avantage {
                            let tok = sentence[i].1;
                            let next_tok = sentence[i + 1].1;
                            let span = Span::new(tok.span.start, next_tok.span.end);
                            // Vérifier que le token courant est bien une élision
                            let is_elided = tok.text.ends_with('\'') || tok.text.ends_with('\u{2019}');
                            if is_elided {
                                suggestions.push(Suggestion {
                                    span,
                                    message: "Confusion «\u{a0}d'avantage\u{a0}»/«\u{a0}davantage\u{a0}» : l'adverbe «\u{a0}davantage\u{a0}» (= plus) ne prend jamais d'apostrophe.".to_string(),
                                    replacements: vec!["davantage".to_string()],
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
        "Confusions « plutôt/plus tôt » et « d'avantage/davantage »"
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
        PlutotConfusion
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        PlutotConfusion.check(&tokenize(text)).len()
    }

    #[test]
    fn plutot_que_temporal() {
        assert_eq!(
            first("il est arrivé plutôt que prévu"),
            Some("plus tôt".into())
        );
    }

    #[test]
    fn plutot_alone_correct() {
        // « plutôt » seul = de préférence
        assert_eq!(count("plutôt du rouge"), 0);
        assert_eq!(count("je préfère plutôt rester"), 0);
    }

    #[test]
    fn davantage_no_apostrophe() {
        assert_eq!(
            first("j'en veux d'avantage"),
            Some("davantage".into())
        );
        assert_eq!(first("il faut d'avantage travailler"), Some("davantage".into()));
    }

    #[test]
    fn davantage_correct() {
        assert_eq!(count("je veux davantage"), 0);
        assert_eq!(count("davantage de temps"), 0);
    }

    #[test]
    fn de_avantage_correct_genitive() {
        // « de l'avantage concurrentiel » — « avantage » suivi d'un GN : pas un adverbe
        // Ici "de" n'est pas élidé donc pas de suggestion.
        assert_eq!(count("il profite de l'avantage"), 0);
    }
}
