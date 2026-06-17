//! Règle : confusion **et / est** — **tranche 6** du moteur de confusions.
//!
//! Mémo : *«et»* est une conjonction de coordination ; *«est»* est la 3ᵉ
//! personne du singulier du verbe *être*. On peut tester en remplaçant par
//! *«était»* : si ça tient, c'est *«est»*.
//!
//! ## et → est
//!
//! **Sujet singulier de 3ᵉ personne + `et` + attribut/adjectif/PP** :
//! - `il et content` → `est`
//! - `elle et belle` → `est`
//! - `le chat et gris` → `est`
//!
//! On exige que « et » soit précédé d'un sujet singulier et suivi d'un attribut
//! (adjectif, participe passé ou nom sans déterminant indéfini). La coordination
//! de verbes (`mange et dort`) ou de noms (`le chat et le chien`) reste intacte.
//!
//! ## est → et (plus rare)
//!
//! Non traité : la direction « est → et » n'a pas de signal séparable fiable
//! (« elle est grande et belle » — le « est » correct ressemble à une copule).

use super::{is_past_participle, normalize, upos};
use crate::morpho::{self, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct EtEstConfusion;

const RULE_ID: &str = "confusion_et_est";

/// Pronoms sujets singuliers de 3ᵉ personne.
const SING_3P_PRONOUNS: &[&str] = &["il", "elle", "on", "ce", "c"];

/// Vrai si la forme est un adjectif ou un participe passé (attribut possible).
fn is_attributive(form: &str) -> bool {
    let morphs = morpho::lookup(form);
    if morphs.iter().any(|m| m.category == MorphCategory::Adjective) {
        return true;
    }
    is_past_participle(form)
}

/// Vrai si le jeton est un sujet singulier de 3ᵉ personne (pronom ou nom sg).
fn is_singular_3rd_subject(sentence: &[(usize, &Token)], k: usize, tags: &[Tagged]) -> bool {
    let form = normalize(sentence[k].1.text.as_str());
    if SING_3P_PRONOUNS.contains(&form.as_str()) {
        return true;
    }
    // Nom singulier étiqueté NOUN
    if !matches!(upos(sentence, k, tags), Upos::Noun | Upos::Propn) {
        return false;
    }
    // Vérifier le nombre singulier dans le lexique
    let morphs = morpho::lookup(sentence[k].1.text.as_str());
    if morphs.is_empty() {
        return true; // inconnu → supposer singulier
    }
    morphs.iter().any(|m| {
        m.category == MorphCategory::Noun
            && m.number == Some(Number::Singular)
            && m.person.is_none()
    })
}

impl Rule for EtEstConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                if normalize(sentence[i].1.text.as_str()) != "et" {
                    continue;
                }
                // Doit être précédé d'un sujet singulier de 3ᵉ pers.
                if i == 0 {
                    continue;
                }
                if !is_singular_3rd_subject(&sentence, i - 1, tags) {
                    continue;
                }
                // Doit être suivi d'un attribut (adj/PP/nom sans article)
                let Some((_, next_tok)) = sentence.get(i + 1) else {
                    continue;
                };
                let next_upos = upos(&sentence, i + 1, tags);
                let next_form = next_tok.text.as_str();
                let attributive = matches!(next_upos, Upos::Adj)
                    || is_attributive(next_form)
                    || matches!(next_upos, Upos::Noun);
                if !attributive {
                    continue;
                }
                // Pas de coordination de noms/verbes multiples (virgule avant)
                let tok = sentence[i].1;
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Confusion «\u{a0}et\u{a0}»/«\u{a0}est\u{a0}» : «\u{a0}{}\u{a0}» est peut-être le verbe «\u{a0}être\u{a0}».",
                        tok.text
                    ),
                    replacements: vec!["est".to_string()],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « et » / « est »"
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
        EtEstConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        EtEstConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn et_to_est_pronoun_adj() {
        assert_eq!(first("il et content"), Some("est".into()));
        assert_eq!(first("elle et belle"), Some("est".into()));
    }

    #[test]
    fn et_to_est_pronoun_participle() {
        assert_eq!(first("il et parti"), Some("est".into()));
        assert_eq!(first("elle et arrivée"), Some("est".into()));
    }

    #[test]
    fn no_false_positive_conjunction() {
        // Coordination de verbes : ne pas toucher
        assert_eq!(count("il mange et dort"), 0);
        // Coordination de noms
        assert_eq!(count("le chat et le chien"), 0);
        // « est » correct
        assert_eq!(count("il est content"), 0);
    }

    #[test]
    fn no_false_positive_plural() {
        // Sujet pluriel : ne pas toucher
        assert_eq!(count("ils et contents"), 0);
    }
}
