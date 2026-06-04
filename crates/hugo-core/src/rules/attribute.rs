//! Règle : accord de l'adjectif attribut du sujet (genre et nombre).
//!
//! On traite la construction *sujet + verbe attributif + adjectif*, où
//! l'adjectif s'accorde avec le sujet : « elle est content » → « contente »,
//! « ils sont content » → « contents ».
//!
//! Approche prudente, pour limiter les faux positifs :
//!
//! - le **verbe attributif** appartient à une liste fermée (`être`, `sembler`,
//!   `paraître`, `devenir`, `rester`, `demeurer`) ;
//! - le **sujet** doit livrer un genre **et** un nombre sans ambiguïté :
//!   pronoms `il/elle/ils/elles`, ou groupe nominal dont le nom porte un genre
//!   connu dans Lexique (les pronoms `je/tu/nous/vous` sont écartés, leur genre
//!   dépendant du locuteur) ;
//! - l'**attribut** doit être analysé comme adjectif ; la forme corrigée est
//!   engendrée par [`morpho::decline`]. Si elle est introuvable, on n'émet rien.

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord en genre et en nombre de l'adjectif attribut avec son sujet.
pub struct AttributeAdjectiveAgreement;

const RULE_ID: &str = "attribute_adjective_agreement";

/// Lemmes des verbes attributifs (copules) déclencheurs.
const COPULAS: &[&str] = &[
    "être", "sembler", "paraître", "devenir", "rester", "demeurer",
];

/// Fenêtre maximale (en jetons lexicaux) explorée de part et d'autre du verbe.
const MAX_WINDOW: usize = 3;

/// Genre et nombre fournis par un pronom personnel sujet, le cas échéant.
/// Seuls les pronoms qui fixent le genre sont retenus.
fn pronoun_features(text: &str) -> Option<(Gender, Number)> {
    match normalize(text).as_str() {
        "il" => Some((Gender::Masculine, Number::Singular)),
        "elle" => Some((Gender::Feminine, Number::Singular)),
        "ils" => Some((Gender::Masculine, Number::Plural)),
        "elles" => Some((Gender::Feminine, Number::Plural)),
        _ => None,
    }
}

/// Minuscules + apostrophe finale ôtée (« qu' » → « qu »).
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Vrai si le jeton est un clitique préverbal (négation ou pronom).
fn is_clitic(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "ne" | "n" | "me" | "m" | "te" | "t" | "se" | "s" | "le" | "la" | "les" | "lui" | "leur"
            | "y" | "en"
    )
}

/// Vrai si le jeton est un adverbe d'intensité/négation pouvant s'intercaler
/// entre la copule et l'attribut (« elle est très content »).
fn is_intensifier(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "très"
            | "si"
            | "plus"
            | "moins"
            | "aussi"
            | "bien"
            | "trop"
            | "fort"
            | "peu"
            | "assez"
            | "plutôt"
            | "vraiment"
            | "extrêmement"
            | "particulièrement"
            | "pas"
            | "jamais"
    )
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

/// Genre et nombre d'un jeton candidat sujet (pronom ou nom à genre connu).
fn subject_features(token: &Token) -> Option<(Gender, Number)> {
    if let Some(f) = pronoun_features(&token.text) {
        return Some(f);
    }
    // Pronoms personnels au genre indéterminé : on s'arrête là (Lexique leur
    // prête parfois une analyse nominale parasite, ex. « je » → nom féminin).
    if matches!(
        normalize(&token.text).as_str(),
        "je" | "j" | "tu" | "nous" | "vous" | "on" | "me" | "m" | "te" | "t" | "se" | "s"
    ) {
        return None;
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
    // Un genre épicène ne permet pas de choisir la forme de l'adjectif.
    if gender == Gender::Epicene {
        return None;
    }
    Some((gender, number))
}

/// Lemme de la copule présente sur ce jeton, le cas échéant.
fn copula_lemma(token: &Token) -> Option<String> {
    morpho::verb_forms(&token.text)
        .into_iter()
        .map(|v| v.lemma)
        .find(|l| COPULAS.contains(&l.as_str()))
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

impl Rule for AttributeAdjectiveAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for lex in lexical_sentences(tokens) {
            for k in 0..lex.len() {
                let Some(_lemma) = copula_lemma(lex[k].1) else {
                    continue;
                };

                // --- Sujet : reculer en sautant clitiques et adjectifs. ---
                let mut s = k;
                let mut steps = 0;
                let subject = loop {
                    if s == 0 || steps > MAX_WINDOW {
                        break None;
                    }
                    s -= 1;
                    steps += 1;
                    let tok = lex[s].1;
                    if let Some(f) = subject_features(tok) {
                        break Some(f);
                    }
                    // Sauter clitiques et adjectifs antéposés ; sinon abandonner.
                    let is_adj = morpho::lookup(&tok.text)
                        .iter()
                        .any(|m| m.category == MorphCategory::Adjective);
                    if is_clitic(&tok.text) || is_adj {
                        continue;
                    }
                    break None;
                };
                let Some((gender, number)) = subject else {
                    continue;
                };

                // --- Attribut : avancer en sautant clitiques et adverbes. ---
                let mut a = k + 1;
                let mut steps = 0;
                while a < lex.len()
                    && steps < MAX_WINDOW
                    && (is_clitic(&lex[a].1.text) || is_intensifier(&lex[a].1.text))
                {
                    a += 1;
                    steps += 1;
                }
                if a >= lex.len() {
                    continue;
                }
                let adj_token = lex[a].1;

                let adjectives: Vec<_> = morpho::lookup(&adj_token.text)
                    .into_iter()
                    .filter(|m| m.category == MorphCategory::Adjective)
                    .collect();
                if adjectives.is_empty() {
                    continue;
                }

                // Déjà accordé ? (une analyse compatible suffit)
                let agrees = adjectives.iter().any(|m| {
                    m.gender
                        .map_or(true, |g| g == gender || g == Gender::Epicene)
                        && m.number
                            .map_or(true, |n| n == number || n == Number::Invariable)
                });
                if agrees {
                    continue;
                }

                // Lemme unique (sinon ambiguïté lexicale).
                let mut lemmas: Vec<&str> = adjectives.iter().map(|m| m.lemma.as_str()).collect();
                lemmas.sort_unstable();
                lemmas.dedup();
                if lemmas.len() != 1 {
                    continue;
                }
                let lemma = adjectives[0].lemma.clone();

                let Some(corrected) = morpho::decline(&lemma, gender, number) else {
                    continue;
                };
                if corrected.eq_ignore_ascii_case(&adj_token.text) {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: adj_token.span,
                    message: format!(
                        "Accord de l'attribut : « {} » ne s'accorde pas avec le sujet.",
                        adj_token.text
                    ),
                    replacements: vec![match_case(&adj_token.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord de l'adjectif attribut"
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
        AttributeAdjectiveAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        AttributeAdjectiveAgreement.check(&tokenize(text)).len()
    }

    #[test]
    fn feminine_subject_pronoun() {
        assert_eq!(first("elle est content").as_deref(), Some("contente"));
    }

    #[test]
    fn plural_subject_pronoun() {
        assert_eq!(first("ils sont content").as_deref(), Some("contents"));
        assert_eq!(first("elles sont content").as_deref(), Some("contentes"));
    }

    #[test]
    fn intensifier_is_skipped() {
        assert_eq!(first("elle est très content").as_deref(), Some("contente"));
    }

    #[test]
    fn other_copulas() {
        assert_eq!(first("elle semble content").as_deref(), Some("contente"));
        assert_eq!(first("elle paraît content").as_deref(), Some("contente"));
        assert_eq!(first("elle devient content").as_deref(), Some("contente"));
    }

    #[test]
    fn nominal_subject_with_known_gender() {
        // « table » est féminin : « la table est content » → « contente ».
        assert_eq!(first("la table est content").as_deref(), Some("contente"));
    }

    #[test]
    fn correct_agreement_yields_nothing() {
        for ok in [
            "elle est contente",
            "il est content",
            "ils sont contents",
            "elles sont contentes",
            "elle est rouge",
            "ils sont rouges",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn ambiguous_gender_pronoun_is_ignored() {
        // « je suis content » : genre du locuteur inconnu → aucune correction.
        assert_eq!(count("je suis content"), 0);
        assert_eq!(count("vous êtes content"), 0);
    }

    #[test]
    fn noun_attribute_is_ignored() {
        // « elle est professeur » : attribut nominal, pas adjectival.
        assert_eq!(count("elle est professeur"), 0);
    }

    #[test]
    fn capitalization_preserved() {
        assert_eq!(first("Elle est Content").as_deref(), Some("Contente"));
    }
}
