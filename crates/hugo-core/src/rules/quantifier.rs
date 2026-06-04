//! Règle : accord du prédéterminant *tout* (tout / toute / tous / toutes).
//!
//! *Tout* placé devant un groupe nominal introduit par un déterminant s'accorde
//! en genre et en nombre avec ce groupe : « toute les jours » → « tous les
//! jours », « tout les semaines » → « toutes les semaines ».
//!
//! Approche fermée et prudente :
//!
//! - on ne se déclenche que sur la séquence *tout* + **déterminant** + … + nom,
//!   ce qui écarte l'emploi adverbial (« tout petit », « tout à coup ») ;
//! - le **nombre** vient du déterminant interne (fiable) ; le **genre** vient du
//!   déterminant s'il en porte un (`le`/`la`…), sinon du nom tête ;
//! - on n'émet rien si le genre ou le nombre cible reste indéterminé.

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord du prédéterminant *tout* avec le groupe nominal qu'il
/// quantifie.
pub struct ToutAgreement;

const RULE_ID: &str = "tout_agreement";

/// Nombre maximal d'adjectifs sautés entre le déterminant interne et le nom.
const MAX_SKIP: usize = 3;

/// Minuscules + apostrophe finale ôtée.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Forme attendue de *tout* pour un genre et un nombre donnés.
fn tout_form(gender: Gender, number: Number) -> Option<&'static str> {
    Some(match (gender, number) {
        (Gender::Masculine, Number::Singular) => "tout",
        (Gender::Feminine, Number::Singular) => "toute",
        (Gender::Masculine, Number::Plural) => "tous",
        (Gender::Feminine, Number::Plural) => "toutes",
        _ => return None,
    })
}

/// Genre (éventuel) et nombre portés par un déterminant susceptible de suivre
/// *tout*. `None` de genre signifie « non porté par le déterminant ».
fn inner_determiner(text: &str) -> Option<(Option<Gender>, Number)> {
    use Gender::*;
    use Number::*;
    Some(match normalize(text).as_str() {
        "le" | "un" | "ce" | "cet" | "mon" | "ton" | "son" => (Some(Masculine), Singular),
        "la" | "une" | "cette" | "ma" | "ta" | "sa" => (Some(Feminine), Singular),
        "l" | "notre" | "votre" | "leur" => (None, Singular),
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => (None, Plural),
        _ => return None,
    })
}

/// Valeur unique d'un trait à travers des analyses, ou `None`.
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

/// Genre du nom tête trouvé à partir de l'index `start` (en sautant les
/// adjectifs antéposés). `None` si aucun nom n'est atteint ou si son genre est
/// indéterminé.
fn head_noun_gender(lex: &[(usize, &Token)], start: usize) -> Option<Gender> {
    let mut k = start;
    let mut steps = 0;
    while k < lex.len() && steps <= MAX_SKIP {
        let analyses = morpho::lookup(&lex[k].1.text);
        let nouns: Vec<_> = analyses
            .iter()
            .filter(|m| m.category == MorphCategory::Noun)
            .collect();
        if !nouns.is_empty() {
            let has_adj = analyses
                .iter()
                .any(|m| m.category == MorphCategory::Adjective);
            // Homographe nom/adjectif suivi d'un nom : adjectif antéposé.
            let next_is_noun = lex.get(k + 1).is_some_and(|(_, t)| {
                morpho::lookup(&t.text)
                    .iter()
                    .any(|m| m.category == MorphCategory::Noun)
            });
            if has_adj && next_is_noun {
                k += 1;
                steps += 1;
                continue;
            }
            return consensus(nouns.iter().map(|m| m.gender));
        }
        if analyses
            .iter()
            .any(|m| m.category == MorphCategory::Adjective)
        {
            k += 1;
            steps += 1;
            continue;
        }
        return None;
    }
    None
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

impl Rule for ToutAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for lex in lexical_sentences(tokens) {
            for i in 0..lex.len() {
                let tout = lex[i].1;
                if !matches!(
                    normalize(&tout.text).as_str(),
                    "tout" | "toute" | "tous" | "toutes"
                ) {
                    continue;
                }

                // Le jeton suivant doit être un déterminant (sinon « tout » est
                // adverbe ou pronom : « tout petit », « tout à coup »).
                let Some((det_gender, number)) = lex.get(i + 1).and_then(|(_, t)| inner_determiner(&t.text))
                else {
                    continue;
                };

                // Genre cible : celui du déterminant, sinon celui du nom tête.
                let gender = match det_gender {
                    Some(g) => g,
                    None => match head_noun_gender(&lex, i + 2) {
                        Some(g) if g != Gender::Epicene => g,
                        _ => continue,
                    },
                };

                let Some(expected) = tout_form(gender, number) else {
                    continue;
                };
                if expected.eq_ignore_ascii_case(&normalize(&tout.text)) {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: tout.span,
                    message: format!(
                        "Accord de « {} » : la forme attendue est « {} ».",
                        tout.text, expected
                    ),
                    replacements: vec![match_case(&tout.text, expected)],
                    rule_id: RULE_ID,
                });
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord de « tout »"
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
        ToutAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        ToutAgreement.check(&tokenize(text)).len()
    }

    #[test]
    fn number_from_determiner() {
        // « toute les jours » → « tous » (les = pluriel, jours = masculin).
        assert_eq!(first("toute les jours").as_deref(), Some("tous"));
    }

    #[test]
    fn feminine_plural() {
        // « tout les semaines » → « toutes » (semaine = féminin).
        assert_eq!(first("tout les semaines").as_deref(), Some("toutes"));
    }

    #[test]
    fn gender_from_determiner_singular() {
        // « tous la journée » → « toute » (la = féminin singulier).
        assert_eq!(first("tous la journée").as_deref(), Some("toute"));
    }

    #[test]
    fn correct_forms_are_silent() {
        for ok in [
            "tout le monde",
            "toute la journée",
            "tous les jours",
            "toutes les semaines",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn adverbial_tout_is_ignored() {
        // « tout » adverbe (pas suivi d'un déterminant) → aucune correction.
        assert_eq!(count("tout petit"), 0);
        assert_eq!(count("tout à coup"), 0);
        assert_eq!(count("tout de suite"), 0);
    }

    #[test]
    fn capitalization_preserved() {
        assert_eq!(first("Toute les jours").as_deref(), Some("Tous"));
    }
}
