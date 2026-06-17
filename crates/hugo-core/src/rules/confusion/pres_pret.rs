//! Règle : confusion **près / prêt** — tranche 6.
//!
//! - **`près`** = adverbe/préposition de lieu ou de proximité
//!   (*«il habite près de la gare»*) ;
//! - **`prêt`** (prête, prêts, prêtes) = adjectif *préparé, disposé à*
//!   (*«je suis prêt à partir»*).
//!
//! ## près → prêt
//!
//! *«il est près à partir»* → *«prêt à partir»*
//!
//! Signal : copule (être, sembler, paraître, se sentir…) + `près` + `à` +
//! infinitif. Le sujet donne le genre/nombre pour l'accord
//! (`prêt/prête/prêts/prêtes`).
//!
//! ## prêt → près
//!
//! Non traité : la direction inverse n'a pas de signal séparable fiable
//! (*«il est prêt de la gare»* — *prêt* attribut + préposition ressemble
//! à *près de*). Documenté comme gap.

use super::{normalize, upos};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct PresPreConfusion;

const RULE_ID: &str = "confusion_pres_pret";

/// Copules qui peuvent précéder « prêt ».
const COPULAS: &[&str] = &[
    "suis", "es", "est", "sommes", "êtes", "sont",
    "étais", "était", "étions", "étiez", "étaient",
    "serai", "seras", "sera", "serons", "serez", "seront",
    "sembles", "semble", "semblent",
    "parais", "paraît",
    "sens", "sent",
];

/// Accord de l'adjectif prêt selon le genre/nombre du sujet.
fn accord_pret(gender: Option<Gender>, number: Option<Number>) -> &'static str {
    match (gender, number) {
        (Some(Gender::Feminine), Some(Number::Plural)) => "prêtes",
        (Some(Gender::Feminine), _) => "prête",
        (_, Some(Number::Plural)) => "prêts",
        _ => "prêt",
    }
}

/// Cherche le sujet à gauche d'une copule et renvoie son genre/nombre.
fn subject_gender_number(
    sentence: &[(usize, &Token)],
    copula_pos: usize,
    tags: &[Tagged],
) -> (Option<Gender>, Option<Number>) {

    if copula_pos == 0 {
        return (None, None);
    }
    // On remonte à la recherche d'un pronom/nom sujet
    for k in (0..copula_pos).rev() {
        let form = normalize(sentence[k].1.text.as_str());
        match form.as_str() {
            "il" | "ce" | "c" => return (Some(Gender::Masculine), Some(Number::Singular)),
            "elle" => return (Some(Gender::Feminine), Some(Number::Singular)),
            "ils" => return (Some(Gender::Masculine), Some(Number::Plural)),
            "elles" => return (Some(Gender::Feminine), Some(Number::Plural)),
            "je" | "tu" | "on" => return (None, Some(Number::Singular)),
            "nous" => return (None, Some(Number::Plural)),
            "vous" => return (None, Some(Number::Plural)),
            _ => {
                // Nom : récupérer son genre/nombre depuis la morphologie
                if matches!(upos(sentence, k, tags), Upos::Noun | Upos::Propn) {
                    let morphs = morpho::lookup(sentence[k].1.text.as_str());
                    let gender = morphs.iter().find_map(|m| {
                        if m.category == MorphCategory::Noun {
                            m.gender
                        } else {
                            None
                        }
                    });
                    let number = morphs.iter().find_map(|m| {
                        if m.category == MorphCategory::Noun {
                            m.number
                        } else {
                            None
                        }
                    });
                    return (gender, number);
                }
            }
        }
    }
    (None, None)
}

impl Rule for PresPreConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                if normalize(sentence[i].1.text.as_str()) != "près" {
                    continue;
                }
                // Doit être précédé d'une copule
                let copula_pos = if i > 0 {
                    let prev = normalize(sentence[i - 1].1.text.as_str());
                    if COPULAS.contains(&prev.as_str()) {
                        Some(i - 1)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let Some(cop_pos) = copula_pos else { continue };

                let next_norm = sentence
                    .get(i + 1)
                    .map(|(_, t)| normalize(&t.text));

                // Signal 1 : copule + « près » + « à » + infinitif
                // « il est près à partir » → « prêt »
                let signal1 = next_norm.as_deref() == Some("à")
                    && sentence.get(i + 2).is_some_and(|(_, t)| super::is_infinitive(&t.text));

                // Signal 2 : copule + « près » + « pour »
                // « tout est près pour l'anniversaire » → « prêt »
                // « près pour » n'existe pas en français ; « prêt pour » est la
                // seule forme correcte.
                let signal2 = next_norm.as_deref() == Some("pour");

                // Signal 3 : copule + « près » en fin de clause (pas de
                // complément de lieu « de »/« d' » qui suivrait)
                let signal3 = match next_norm.as_deref() {
                    None => true,           // fin de phrase
                    Some("de" | "d") => false, // « près de » = correct
                    Some(_) => false,       // autre suite : trop ambigu
                };

                if !signal1 && !signal2 && !signal3 {
                    continue;
                }
                // Accord avec le sujet
                let (gender, number) = subject_gender_number(&sentence, cop_pos, tags);
                let correction = accord_pret(gender, number);
                let tok = sentence[i].1;
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: format!(
                        "Confusion «\u{a0}près\u{a0}»/«\u{a0}prêt\u{a0}» : l'adjectif «\u{a0}prêt\u{a0}» (disposé à) s'écrit avec un accent circonflexe."
                    ),
                    replacements: vec![correction.to_string()],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « près » / « prêt »"
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
        PresPreConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        PresPreConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn pres_to_pret_masc_sg() {
        assert_eq!(first("il est près à partir"), Some("prêt".into()));
        assert_eq!(first("il semble près à commencer"), Some("prêt".into()));
    }

    #[test]
    fn pres_to_pret_fem_sg() {
        assert_eq!(first("elle est près à partir"), Some("prête".into()));
    }

    #[test]
    fn pres_to_pret_plural() {
        assert_eq!(first("ils sont près à partir"), Some("prêts".into()));
    }

    #[test]
    fn pres_to_pret_before_pour() {
        // « copule + près + pour » → « prêt/prête… »
        assert_eq!(first("tout est près pour l'anniversaire"), Some("prêt".into()));
        assert_eq!(first("elle est près pour la compétition"), Some("prête".into()));
        assert_eq!(first("ils sont près pour le départ"), Some("prêts".into()));
    }

    #[test]
    fn pres_to_pret_at_end_of_clause() {
        // « copule + près » en fin de phrase sans complément de lieu.
        assert_eq!(first("nous sommes près"), Some("prêts".into()));
        assert_eq!(first("je suis près"), Some("prêt".into()));
    }

    #[test]
    fn pres_correct_adverb() {
        // « près » adverbe de lieu (suivi de « de »)
        assert_eq!(count("il habite près de la gare"), 0);
        assert_eq!(count("reste près de moi"), 0);
        assert_eq!(count("il est près de réussir"), 0);
    }

    #[test]
    fn pret_correct_adjective() {
        assert_eq!(count("il est prêt à partir"), 0);
        assert_eq!(count("elle est prête"), 0);
        assert_eq!(count("ils sont prêts pour le départ"), 0);
    }
}
