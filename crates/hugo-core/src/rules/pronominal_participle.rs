//! Règle : accord du participe passé des **verbes pronominaux** (auxiliaire
//! « être »).
//!
//! Dans « sujet + pronom réfléchi + être + participe », le participe s'accorde
//! le plus souvent avec le sujet : « elle s'est levé » → « levée », « ils se
//! sont assis » est correct, « elles se sont trompé » → « trompées ».
//!
//! Garde essentielle — le **COD postposé** : quand un objet direct suit le
//! participe (« elle s'est lavé **les mains** »), c'est lui le complément, le
//! pronom réfléchi est indirect, et le participe **reste invariable**. La règle
//! s'abstient alors (repérage : un déterminant ou un nom suit le participe).
//!
//! Approche prudente (comme l'accord de l'attribut) : sujet à **genre connu**
//! (pronoms `il/elle/ils/elles` ou nom au genre lexical déterminé) ; les pronoms
//! `je/tu/on/nous/vous`, au genre indéterminé, sont écartés.

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Accord du participe passé des verbes pronominaux.
pub struct PronominalParticiple;

const RULE_ID: &str = "pronominal_participle";

/// Minuscules + apostrophe finale ôtée.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Calque la casse initiale de `original` sur `replacement`.
fn match_case(original: &str, replacement: &str) -> String {
    if !original.chars().next().is_some_and(|c| c.is_uppercase()) {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => replacement.to_string(),
    }
}

/// Valeur unique d'un trait à travers des analyses, ou `None` si absente ou
/// contradictoire.
fn consensus<T: PartialEq + Copy>(values: impl Iterator<Item = Option<T>>) -> Option<T> {
    let mut found: Option<T> = None;
    for v in values.flatten() {
        match found {
            None => found = Some(v),
            Some(prev) if prev == v => {}
            Some(_) => return None,
        }
    }
    found
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « être ».
fn is_etre(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "être")
}

/// Vrai si le jeton est un pronom réfléchi objet préverbal.
fn is_reflexive(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "se" | "s" | "me" | "m" | "te" | "t" | "nous" | "vous"
    )
}

/// Vrai si le jeton est la négation « ne ».
fn is_ne(text: &str) -> bool {
    matches!(normalize(text).as_str(), "ne" | "n")
}

/// Adverbe pouvant s'intercaler entre l'auxiliaire et le participe.
fn is_skippable_adverb(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "pas"
            | "jamais"
            | "plus"
            | "déjà"
            | "bien"
            | "toujours"
            | "encore"
            | "vraiment"
            | "souvent"
            | "vite"
            | "enfin"
    )
}

/// Genre et nombre d'un sujet à genre connu (pronom ou nom au genre lexical sûr).
fn subject_features(token: &Token) -> Option<(Gender, Number)> {
    match normalize(&token.text).as_str() {
        "il" => return Some((Gender::Masculine, Number::Singular)),
        "elle" => return Some((Gender::Feminine, Number::Singular)),
        "ils" => return Some((Gender::Masculine, Number::Plural)),
        "elles" => return Some((Gender::Feminine, Number::Plural)),
        // Pronoms au genre indéterminé : écartés.
        "je" | "j" | "tu" | "on" | "nous" | "vous" => return None,
        _ => {}
    }
    let nouns: Vec<_> = morpho::lookup(&token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if nouns.is_empty() {
        return None;
    }
    let gender = consensus(nouns.iter().map(|m| m.gender))?;
    let number = consensus(nouns.iter().map(|m| m.number))?;
    (gender != Gender::Epicene).then_some((gender, number))
}

/// Analyses « participe passé » d'une forme.
fn participles(text: &str) -> Vec<morpho::Morph> {
    morpho::lookup(text)
        .into_iter()
        .filter(|m| {
            m.category == MorphCategory::Verb
                && m.person.is_none()
                && m.gender.is_some()
                && m.number.is_some()
        })
        .collect()
}

/// Vrai si le jeton (POS) introduit un objet direct postposé (déterminant ou
/// nom) — auquel cas le participe pronominal reste invariable.
fn opens_object(tag: Upos) -> bool {
    matches!(tag, Upos::Det | Upos::Noun | Upos::Propn)
}

impl Rule for PronominalParticiple {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for c in 0..lex.len() {
                // Auxiliaire « être ».
                if !is_etre(&lex[c].1.text) {
                    continue;
                }
                // Pronom réfléchi immédiatement avant (la négation « ne » se place
                // avant le réfléchi : « il ne s'est pas levé »).
                if c == 0 || !is_reflexive(&lex[c - 1].1.text) {
                    continue;
                }
                // Sujet : avant le réfléchi, en sautant une éventuelle négation.
                let mut s = c - 1;
                while s > 0 && is_ne(&lex[s - 1].1.text) {
                    s -= 1;
                }
                if s == 0 {
                    continue;
                }
                let Some((gender, number)) = subject_features(lex[s - 1].1) else {
                    continue;
                };

                // Participe : après l'auxiliaire, en sautant les adverbes.
                let mut p = c + 1;
                while p < lex.len() && is_skippable_adverb(&lex[p].1.text) {
                    p += 1;
                }
                let Some(&(_, part_token)) = lex.get(p) else {
                    continue;
                };
                let parts = participles(&part_token.text);
                if parts.is_empty() {
                    continue;
                }

                // Garde COD postposé : un déterminant ou un nom après le participe
                // signale un objet direct → participe invariable, on s'abstient.
                if let Some(&(next_idx, _)) = lex.get(p + 1) {
                    if opens_object(tags[next_idx].upos) {
                        continue;
                    }
                }

                // Déjà accordé ?
                let agrees = parts
                    .iter()
                    .any(|m| m.gender == Some(gender) && m.number == Some(number));
                if agrees {
                    continue;
                }
                let mut lemmas = parts.iter().map(|m| m.lemma.as_str());
                let lemma = lemmas.next().unwrap();
                if !lemmas.all(|l| l == lemma) {
                    continue; // lemme ambigu
                }
                let Some(corrected) = morpho::participle(lemma, gender, number) else {
                    continue;
                };
                if corrected.eq_ignore_ascii_case(&part_token.text) {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: part_token.span,
                    message: format!(
                        "Accord du participe passé pronominal : « {} » doit s'accorder avec le sujet.",
                        part_token.text
                    ),
                    replacements: vec![match_case(&part_token.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord du participe passé pronominal"
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
        PronominalParticiple
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        PronominalParticiple.check(&tokenize(text)).len()
    }

    #[test]
    fn feminine_subject() {
        assert_eq!(first("elle s'est levé").as_deref(), Some("levée"));
    }

    #[test]
    fn plural_subjects() {
        assert_eq!(first("ils se sont levé").as_deref(), Some("levés"));
        assert_eq!(first("elles se sont trompé").as_deref(), Some("trompées"));
    }

    #[test]
    fn negation_is_handled() {
        assert_eq!(first("elle ne s'est pas levé").as_deref(), Some("levée"));
    }

    #[test]
    fn postposed_object_blocks_agreement() {
        // « elle s'est lavé les mains » : COD postposé → participe invariable.
        assert_eq!(count("elle s'est lavé les mains"), 0);
    }

    #[test]
    fn correct_agreement_is_silent() {
        for ok in [
            "elle s'est levée",
            "ils se sont levés",
            "il s'est levé",
            "elles se sont trompées",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn unknown_gender_subject_is_ignored() {
        // « je me suis levé » : genre du locuteur inconnu → aucune correction.
        assert_eq!(count("je me suis levé"), 0);
    }
}
