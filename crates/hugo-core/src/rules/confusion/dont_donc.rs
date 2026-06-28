//! Règle : confusion **dont / donc** — tranche complémentaire du moteur de
//! confusions.
//!
//! ## donc → dont
//!
//! Signal : **NOM + `donc` (sans virgule entre les deux) + PRONOM_SUJET** →
//! la structure est celle d'une proposition relative, pas d'une consécutive.
//!
//! - `le film donc j'ai parlé` → `dont j'ai parlé`
//! - `la raison donc elle est venue` → `dont elle est venue`
//! - `le sujet donc nous discutons` → `dont nous discutons`
//!
//! **Garde clé** : si un verbe fini (VERB/AUX) précède le nom dans la même
//! phrase, le nom est probablement objet d'un verbe principal et `donc` est
//! une conjonction consécutive légitime (`il a fini le travail donc il part`).
//! On ne déclenche que quand le nom est en **position initiale** (sans verbe
//! antérieur dans la phrase).
//!
//! ## dont → donc
//!
//! Gap structurel : `, dont SUJET + VERBE` peut être une relative valide
//! (`l'auteur dont il parle`) ou une erreur (`il était tard, dont il est
//! parti`). Signal non séparable — non traité.

use super::{normalize, upos};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

pub struct DontDoncConfusion;

const RULE_ID: &str = "confusion_dont_donc";

/// Pronoms sujets reconnus (formes normalisées, apostrophe ôtée).
const SUBJECT_PRONOUNS: &[&str] = &[
    "je", "j", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles",
];

impl Rule for DontDoncConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                if normalize(sentence[i].1.text.as_str()) != "donc" {
                    continue;
                }
                // Précédent lexical : doit être un nom ou un nom propre.
                if i == 0 {
                    continue;
                }
                if !matches!(
                    upos(&sentence, i - 1, tags),
                    Upos::Noun | Upos::Propn
                ) {
                    continue;
                }
                // Pas de virgule entre le nom et « donc » dans les tokens bruts.
                let prev_raw = sentence[i - 1].0;
                let curr_raw = sentence[i].0;
                let has_comma = (prev_raw + 1..curr_raw).any(|j| {
                    tokens[j].kind == TokenKind::Punctuation && tokens[j].text == ","
                });
                if has_comma {
                    continue;
                }
                // Pas de verbe fini avant le nom dans la même phrase : si oui,
                // le nom est objet d'un verbe principal et « donc » est consécutif.
                let has_prior_verb = (0..i - 1)
                    .any(|k| matches!(upos(&sentence, k, tags), Upos::Verb | Upos::Aux));
                if has_prior_verb {
                    continue;
                }
                // Suivant lexical : doit être un pronom sujet.
                let Some(&(next_raw, next_tok)) = sentence.get(i + 1) else {
                    continue;
                };
                if !matches!(tags[next_raw].upos, Upos::Pron) {
                    continue;
                }
                if !SUBJECT_PRONOUNS.contains(&normalize(next_tok.text.as_str()).as_str()) {
                    continue;
                }

                let tok = sentence[i].1;
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Confusion « dont »/« donc » : « {} » introduit une relative — écrivez « dont ».",
                        tok.text
                    ),
                    replacements: vec!["dont".to_string()],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « dont » / « donc »"
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
        DontDoncConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        DontDoncConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn donc_to_dont_pronoun_subject() {
        assert_eq!(first("le film donc j'ai parlé").as_deref(), Some("dont"));
        assert_eq!(first("la raison donc elle est venue").as_deref(), Some("dont"));
        assert_eq!(first("le sujet donc nous discutons").as_deref(), Some("dont"));
        assert_eq!(first("le problème donc il faut parler").as_deref(), Some("dont"));
    }

    #[test]
    fn no_false_positive_comma_before_donc() {
        // Virgule avant « donc » → consécutive, ne pas toucher.
        assert_eq!(count("je suis fatigué, donc je dors"), 0);
        assert_eq!(count("il a fini, donc il part"), 0);
    }

    #[test]
    fn no_false_positive_prior_verb() {
        // Verbe avant le nom → « donc » est consécutif.
        assert_eq!(count("il a vu le film donc il part"), 0);
        assert_eq!(count("elle a fini son travail donc elle se repose"), 0);
    }

    #[test]
    fn no_false_positive_dont_already_correct() {
        assert_eq!(count("le film dont j'ai parlé"), 0);
        assert_eq!(count("la raison dont elle est venue"), 0);
    }

    #[test]
    fn no_false_positive_object_pronoun_after_donc() {
        // « donc » + pronom objet (pas sujet) → pas de relative.
        assert_eq!(count("le film donc lui plaisait"), 0);
    }
}
