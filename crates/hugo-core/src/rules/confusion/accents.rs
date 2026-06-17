//! Règle : confusions de **paires accentuées** — tranche 6.
//!
//! Plusieurs paires de mots sont homophones et ne se distinguent que par un
//! accent : `du/dû`, `sur/sûr`, `notre/nôtre`, `votre/vôtre`, `mur/mûr`,
//! `cru/crû`.
//!
//! | Sans accent | Avec accent | Distinction |
//! |---|---|---|
//! | `du` (art. contracté ou partitif) | `dû` (pp de *devoir*, ou adj.) | — |
//! | `sur` (préposition) | `sûr` (adjectif = certain) | après copule + sans COD → `sûr` |
//! | `notre` (déterminant poss.) | `nôtre` (pronom poss.) | article `le/la/les` + `nôtre/nôtres` |
//! | `votre` (déterminant poss.) | `vôtre` (pronom poss.) | idem |
//! | `mur` (n.m. paroi) | `mûr` (adj. = arrivé à maturité) | après copule → `mûr` |
//! | `cru` (adj. = non cuit, ou pp de *croire*) | `crû` (pp de *croître*) | rare |
//!
//! ## du → dû
//!
//! Signal : `du` suivi de la préposition `à` + infinitif (ou « au fait que ») :
//! *«c'est du à la chaleur»* → *«dû à la chaleur»*. L'article/partitif `du`
//! ne s'emploie jamais devant `à`.
//!
//! ## notre/votre → nôtre/vôtre
//!
//! Signal : article défini `le/la/les` + `notre/votre` → pronom possessif →
//! accent circonflexe requis.
//!
//! ## sur → sûr
//!
//! Déjà partiellement géré par la règle `attribute` (garde ADP). On complète
//! ici : copule + `sur` + adjectif/adverbe (ou fin de phrase) → `sûr`.
//!
//! ## mur → mûr
//!
//! Signal : copule + `mur` → l'adjectif *mûr* (avec accent).

use super::{normalize, upos};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct AccentsConfusion;

const RULE_ID: &str = "confusion_accents";

/// Copules.
const COPULAS: &[&str] = &[
    "suis", "es", "est", "sommes", "êtes", "sont",
    "étais", "était", "étions", "étiez", "étaient",
    "serai", "seras", "sera", "serons", "serez", "seront",
    "sembles", "semble", "semblent",
    "parais", "paraît", "paraissent",
];

/// Articles définis (pour le/la/les + notre/votre → nôtre/vôtre).
const DEF_ARTICLES: &[&str] = &["le", "la", "les"];

/// Vrai si le mot suivant la copule justifie de lire « sur » comme adjectif
/// (fin de phrase ou suivi d'un adverbe/adjectif mais pas d'un nom).
fn sur_is_adjective_context(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> bool {
    // Fin de phrase : « il est sûr. »
    if i + 1 >= sentence.len() {
        return true;
    }
    let next_upos = upos(sentence, i + 1, tags);
    // Suivi d'un adverbe ou d'une ponctuation
    if matches!(next_upos, Upos::Adv | Upos::Punct) {
        return true;
    }
    // Suivi de « que » → « il est sûr que… »
    let next_form = normalize(sentence[i + 1].1.text.as_str());
    if next_form == "que" || next_form == "qu" {
        return true;
    }
    false
}

impl Rule for AccentsConfusion {
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
                    // --- du → dû devant « à » ---
                    "du" => {
                        let next_is_a = sentence
                            .get(i + 1)
                            .is_some_and(|(_, t)| normalize(&t.text) == "à");
                        if next_is_a {
                            // Vérifier qu'il n'y a pas d'article défini avant (l'article
                            // contracté « du = de+le » peut précéder à, très rare mais possible)
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}du\u{a0}»/«\u{a0}dû\u{a0}» : devant «\u{a0}à\u{a0}», il s'agit du participe «\u{a0}dû\u{a0}» (de *devoir*).".to_string(),
                                replacements: vec!["dû".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- sur → sûr après copule (contexte adjectival) ---
                    "sur" => {
                        let prev_is_copula = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            COPULAS.contains(&prev.as_str())
                        };
                        if prev_is_copula && sur_is_adjective_context(&sentence, i, tags) {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}sur\u{a0}»/«\u{a0}sûr\u{a0}» : l'adjectif «\u{a0}sûr\u{a0}» (= certain) prend un accent circonflexe.".to_string(),
                                replacements: vec!["sûr".to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- notre → nôtre / votre → vôtre après article défini ---
                    "notre" | "votre" => {
                        let prev_is_article = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            DEF_ARTICLES.contains(&prev.as_str())
                        };
                        if prev_is_article {
                            let accent = if form == "notre" { "nôtre" } else { "vôtre" };
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: format!(
                                    "Confusion «\u{a0}{form}\u{a0}»/«\u{a0}{accent}\u{a0}» : après un article défini, le pronom possessif prend un accent circonflexe."
                                ),
                                replacements: vec![accent.to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- nos → nôtres / vos → vôtres (pluriel pronom poss.) ---
                    "notres" | "votres" => {
                        let prev_is_article = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            prev == "les"
                        };
                        if prev_is_article {
                            let accent = if form == "notres" { "nôtres" } else { "vôtres" };
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: format!(
                                    "Confusion pluriel : le pronom possessif s'écrit «\u{a0}{accent}\u{a0}»."
                                ),
                                replacements: vec![accent.to_string()],
                                rule_id: RULE_ID,
                            });
                        }
                    }

                    // --- mur → mûr après copule ---
                    "mur" => {
                        let prev_is_copula = i > 0 && {
                            let prev = normalize(sentence[i - 1].1.text.as_str());
                            COPULAS.contains(&prev.as_str())
                        };
                        if prev_is_copula {
                            let tok = sentence[i].1;
                            suggestions.push(Suggestion {
                                span: tok.span,
                                message: "Confusion «\u{a0}mur\u{a0}»/«\u{a0}mûr\u{a0}» : l'adjectif «\u{a0}mûr\u{a0}» (= arrivé à maturité) prend un accent.".to_string(),
                                replacements: vec!["mûr".to_string()],
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
        "Confusions de paires accentuées (du/dû, sur/sûr, notre/nôtre…)"
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
        AccentsConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AccentsConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn du_to_du_accent_before_a() {
        assert_eq!(first("c'est du à la chaleur"), Some("dû".into()));
        assert_eq!(first("le retard est du à la grève"), Some("dû".into()));
    }

    #[test]
    fn du_correct_partitive() {
        assert_eq!(count("je bois du café"), 0);
        assert_eq!(count("il a du talent"), 0);
    }

    #[test]
    fn sur_to_sur_accent_after_copula() {
        assert_eq!(first("il est sur que"), Some("sûr".into()));
        assert_eq!(first("elle est sur"), Some("sûr".into()));
    }

    #[test]
    fn sur_correct_preposition() {
        // « sur » préposition ne doit pas déclencher
        assert_eq!(count("il est sur la table"), 0);
        assert_eq!(count("la nappe est sur la table"), 0);
    }

    #[test]
    fn notre_to_notre_accent_after_article() {
        assert_eq!(first("c'est le notre"), Some("nôtre".into()));
        assert_eq!(first("c'est la notre"), Some("nôtre".into()));
    }

    #[test]
    fn notre_correct_det() {
        assert_eq!(count("notre maison est belle"), 0);
        assert_eq!(count("votre voiture est là"), 0);
    }

    #[test]
    fn votre_to_votre_accent() {
        assert_eq!(first("c'est le votre"), Some("vôtre".into()));
    }

    #[test]
    fn mur_to_mur_accent_after_copula() {
        assert_eq!(first("ce fruit est mur"), Some("mûr".into()));
    }

    #[test]
    fn mur_correct_noun() {
        assert_eq!(count("il repeint le mur"), 0);
        assert_eq!(count("un mur en pierre"), 0);
    }
}
