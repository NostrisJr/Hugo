//! Phase 10 — **Accords des numéraux** : *quatre-vingts*, *deux cents*, trait
//! d'union numéral (réforme orthographique 1990).
//!
//! ## Règle des numéraux invariables
//!
//! - **`vingt`** prend un **s** quand il est multiple entier et en fin de
//!   numéral : *quatre-vingts* (80) mais *quatre-vingt-un* (81).
//! - **`cent`** prend un **s** de même : *deux cents* (200) mais *deux cent
//!   trois* (203).
//!
//! ## Trait d'union (réforme 1990)
//!
//! La réforme orthographique de 1990 recommande de **toujours** lier les
//! composants numéraux par un trait d'union : *vingt-et-un*, *cent-deux*…
//! On signale uniquement les cas les plus fréquents et non ambigus.
//!
//! ## Implémentation
//!
//! On travaille sur les tokens bruts (mots + ponctuation) plutôt que sur les
//! tokens lexicaux seuls, car le trait d'union est représenté par la
//! tokenisation (les mots composés sont un seul token).
//!
//! Stratégie : on identifie `vingt`/`cent` **suivis d'un chiffre/numéral**
//! (accord absent requis) ou **en fin de groupe numéral** (accord requis).

use super::Rule;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

const RULE_ID: &str = "numeraux";

/// Numéraux qui peuvent suivre `vingt` ou `cent` et leur interdisent l'accord.
const UNITS: &[&str] = &[
    "un", "une", "deux", "trois", "quatre", "cinq", "six", "sept", "huit",
    "neuf", "dix", "onze", "douze", "treize", "quatorze", "quinze", "seize",
    "dix-sept", "dix-huit", "dix-neuf",
];

/// Vrai si `form` (en minuscules) est un numéral (unité, dizaine, centaine).
fn is_numeral(form: &str) -> bool {
    UNITS.contains(&form)
        || matches!(
            form,
            "vingt" | "vingts" | "trente" | "quarante" | "cinquante"
                | "soixante" | "cent" | "cents" | "mille" | "million"
                | "millions" | "milliard" | "milliards" | "et"
        )
}

pub struct NumerauxRule;

impl Rule for NumerauxRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let n = tokens.len();

        for i in 0..n {
            let tok = &tokens[i];
            if tok.kind != TokenKind::Word {
                continue;
            }
            let lower = tok.text.to_lowercase();

            // --- Tokens composés avec trait d'union : « quatre-vingts-un » → « quatre-vingt-un » ---
            if lower.contains("-vingts-") {
                // Chercher si ce qui suit "-vingts-" est une unité
                if let Some(after) = lower.split("-vingts-").nth(1) {
                    let unit = after.split('-').next().unwrap_or(after);
                    if UNITS.contains(&unit) {
                        let corrected = lower.replace("-vingts-", "-vingt-");
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: "Accord numéral : «\u{a0}vingts\u{a0}» perd son «\u{a0}s\u{a0}» quand il est suivi d'un autre chiffre.".to_string(),
                            replacements: vec![corrected],
                            rule_id: RULE_ID,
                        });
                        continue;
                    }
                }
            }

            match lower.as_str() {
                // --- vingts (accordé) suivi d'un numéral → supprimer le s ---
                "vingts" => {
                    if next_word(tokens, i)
                        .is_some_and(|w| UNITS.contains(&w.to_lowercase().as_str()))
                    {
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: "Accord numéral : «\u{a0}vingts\u{a0}» perd son «\u{a0}s\u{a0}» quand il est suivi d'un autre chiffre.".to_string(),
                            replacements: vec!["vingt".to_string()],
                            rule_id: RULE_ID,
                        });
                    }
                }

                // --- cent (non accordé) en fin de syntagme → ajouter le s ---
                "cent" => {
                    // Précédé d'un numéral (multiplicateur) et non suivi d'un numéral.
                    // Utilise prev_word() pour sauter les espaces.
                    let prev_lower = prev_word(tokens, i)
                        .map(|w| w.to_lowercase());
                    let prev_is_num = prev_lower
                        .as_deref()
                        .is_some_and(|p| is_numeral(p) && p != "et" && p != "mille");
                    let next_is_num = next_word(tokens, i)
                        .is_some_and(|w| is_numeral(&w.to_lowercase()));
                    if prev_is_num && !next_is_num {
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: "Accord numéral : «\u{a0}cent\u{a0}» prend un «\u{a0}s\u{a0}» quand il est multiple et en fin de numéral.".to_string(),
                            replacements: vec!["cents".to_string()],
                            rule_id: RULE_ID,
                        });
                    }
                }

                // --- cents (accordé) suivi d'un numéral → supprimer le s ---
                "cents" => {
                    if next_word(tokens, i)
                        .is_some_and(|w| is_numeral(&w.to_lowercase()))
                    {
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: "Accord numéral : «\u{a0}cents\u{a0}» perd son «\u{a0}s\u{a0}» quand il est suivi d'un autre chiffre.".to_string(),
                            replacements: vec!["cent".to_string()],
                            rule_id: RULE_ID,
                        });
                    }
                }

                _ => {}
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord des numéraux (vingt, cent)"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

/// Renvoie le texte du prochain token Word (en sautant les espaces).
fn next_word<'a>(tokens: &'a [Token], i: usize) -> Option<&'a str> {
    tokens[i + 1..]
        .iter()
        .find(|t| t.kind == TokenKind::Word)
        .map(|t| t.text.as_str())
}

/// Renvoie le texte du token Word précédent (en sautant les espaces).
fn prev_word<'a>(tokens: &'a [Token], i: usize) -> Option<&'a str> {
    tokens[..i]
        .iter()
        .rev()
        .find(|t| t.kind == TokenKind::Word)
        .map(|t| t.text.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn repls(text: &str) -> Vec<String> {
        NumerauxRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    fn count(text: &str) -> usize {
        NumerauxRule.check(&tokenize(text)).len()
    }

    #[test]
    fn vingts_before_unit_loses_s_compound() {
        // Tokens composés avec trait d'union
        assert_eq!(repls("quatre-vingts-un"), vec!["quatre-vingt-un"]);
        assert_eq!(repls("quatre-vingts-deux"), vec!["quatre-vingt-deux"]);
    }

    #[test]
    fn vingts_before_unit_loses_s_spaced() {
        // Tokens séparés par des espaces (erreur fréquente)
        assert_eq!(repls("quatre vingts un"), vec!["vingt"]);
    }

    #[test]
    fn vingts_alone_no_trigger() {
        // « quatre-vingts » seul = correct (80)
        assert_eq!(count("quatre-vingts euros"), 0);
        assert_eq!(count("quatre vingts euros"), 0);
    }

    #[test]
    fn cent_multiple_end_gets_s() {
        // « deux cent » → « deux cents » (200)
        assert_eq!(repls("deux cent euros"), vec!["cents"]);
        assert_eq!(repls("cinq cent kilomètres"), vec!["cents"]);
    }

    #[test]
    fn cent_before_unit_no_s() {
        // « deux cents trois » → « deux cent trois » (déjà fait → signaler l'erreur inverse)
        assert_eq!(repls("deux cents trois"), vec!["cent"]);
    }

    #[test]
    fn mille_no_accord() {
        // « mille » est invariable
        assert_eq!(count("deux mille euros"), 0);
    }
}
