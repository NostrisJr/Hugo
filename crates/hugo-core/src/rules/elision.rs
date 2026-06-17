//! Phase 8 — **Élisions et contractions obligatoires**.
//!
//! En français, certains mots se **contractent** (perdent leur voyelle finale)
//! devant un mot commençant par une voyelle ou un *h* muet. Cette règle détecte
//! les élisions manquantes (*«le arbre»* → *«l'arbre»*) et les élisions fautives
//! devant un *h* aspiré (*«l'héros»* → *«le héros»*).
//!
//! ## Élisions traitées
//!
//! | Forme pleine | Forme élidée | Contexte |
//! |---|---|---|
//! | `le/la + voyelle/h-muet` | `l'` | article défini |
//! | `de + voyelle/h-muet` | `d'` | préposition |
//! | `je + voyelle/h-muet` | `j'` | pronom sujet |
//! | `me/te/se + voyelle/h-muet` | `m'/t'/s'` | pronoms clitiques |
//! | `ne + voyelle/h-muet` | `n'` | négation |
//! | `que + voyelle/h-muet` | `qu'` | conjonction / relatif |
//! | `ce + voyelle/h-muet` | `cet` | déterminant démonstratif |
//! | `si + il/ils` | `s'il/s'ils` | conjonction conditionnelle |
//! | `lorsque/puisque/quoique + voyelle` | forme élidée | conjonctions |
//!
//! ## H aspiré (élision inverse)
//!
//! *«l'héros»* → *«le héros»* : l'élision est fautive devant un h aspiré.
//! On consulte la liste [`H_ASPIRES`] ; si le mot suivant est dans la liste,
//! on propose de défaire l'élision.

use super::Rule;
use crate::morpho;
use crate::tokenizer::{Token, TokenKind};
use crate::Span;
use crate::Suggestion;

const RULE_ID: &str = "elision";

/// Mots à *h aspiré* : élision et liaison **interdites**.
/// Liste originale curée pour Hugo (non copiée de Grammalecte ni de LT).
const H_ASPIRES: &[&str] = &[
    // hache, haie, hall…
    "hache",
    "haches",
    "haie",
    "haies",
    "hall",
    "halls",
    "halte",
    "haltes",
    "hameau",
    "hameaux",
    "hamster",
    "hamsters",
    "hanche",
    "hanches",
    "handicap",
    "handicaps",
    "hangar",
    "hangars",
    "haras",
    "hareng",
    "harengs",
    "haricot",
    "haricots",
    "harpe",
    "harpes",
    "hasard",
    "hasards",
    "hâte",
    "hausse",
    "hausses",
    "haut",
    "haute",
    "hauts",
    "hautes",
    "hauteur",
    "hauteurs",
    "havre",
    "havres",
    // héros (mais pas héroïne, héroïsme — h muet)
    "héros",
    // hibou, hockey…
    "hibou",
    "hiboux",
    "hockey",
    // honte
    "honte",
    "hontes",
    // horde, hors
    "horde",
    "hordes",
    "hors",
    // housse
    "housse",
    "housses",
    // hutte
    "hutte",
    "huttes",
    // hyène
    "hyène",
    "hyènes",
];

/// Vrai si `word` commence par une **voyelle** ou un **h muet** (donc requiert
/// élision de l'article/préposition précédent).
pub(crate) fn starts_with_vowel_or_mute_h(word: &str) -> bool {
    let lower = word.to_lowercase();
    let lower = lower.as_str();
    let first = lower.chars().next();
    match first {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'â' | 'à' | 'ê' | 'è' | 'é' | 'î' | 'ô' | 'û' | 'ù' | 'ü' | 'ï' | 'œ') => true,
        Some('h') => {
            // h muet sauf si dans la liste des h aspirés
            let base = lower.split('-').next().unwrap_or(lower);
            !H_ASPIRES.contains(&base)
        }
        _ => false,
    }
}

/// Vrai si tous les tokens entre les indices `i1+1` et `i2-1` (inclus) dans la
/// tranche complète sont des espaces (pas de ponctuation intercalée).
fn only_whitespace_between(tokens: &[Token], i1: usize, i2: usize) -> bool {
    if i2 <= i1 + 1 {
        return true;
    }
    tokens[i1 + 1..i2]
        .iter()
        .all(|t| t.kind == TokenKind::Whitespace)
}

/// Minuscules normalisées (apostrophe finale retirée).
fn norm(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Ajoute une suggestion d'élision manquante (fusion de deux tokens + apostrophe).
///
/// `elided_prefix` : préfixe élidé sans apostrophe (ex. `"l"`, `"d"`, `"j"`).
fn suggest_elision(
    tokens: &[Token],
    idx1: usize,
    idx2: usize,
    elided_prefix: &str,
    original: &str,
) -> Suggestion {
    let word = &tokens[idx2].text;
    let replacement = format!("{}'{}", elided_prefix, word);
    let span = Span::new(tokens[idx1].span.start, tokens[idx2].span.end);
    Suggestion {
        span,
        message: format!(
            "Élision manquante : «\u{a0}{}\u{a0}» devrait s'écrire «\u{a0}{}\u{a0}».",
            original, replacement
        ),
        replacements: vec![replacement],
        rule_id: RULE_ID,
    }
}

/// Règle d'élision obligatoire.
pub struct ElisionRule;

impl Rule for ElisionRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let n = tokens.len();

        for i in 0..n {
            let tok = &tokens[i];
            if tok.kind != TokenKind::Word {
                continue;
            }
            let lower = norm(&tok.text);

            // Trouver le prochain token lexical (non-espace).
            let Some(j) = (i + 1..n).find(|&k| {
                tokens[k].kind != TokenKind::Whitespace
            }) else {
                continue;
            };
            let next = &tokens[j];
            if next.kind != TokenKind::Word && next.kind != TokenKind::Number {
                continue;
            }
            // Seul de l'espace entre les deux (pas de ponctuation).
            if !only_whitespace_between(tokens, i, j) {
                continue;
            }

            let next_lower = next.text.to_lowercase();
            let original = format!("{} {}", tok.text, next.text);

            match lower.as_str() {
                // ------- le / la → l' -------
                "le" | "la" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "l", &original));
                }
                // ------- de → d' -------
                "de" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "d", &original));
                }
                // ------- je → j' -------
                "je" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "j", &original));
                }
                // ------- me → m' -------
                "me" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "m", &original));
                }
                // ------- te → t' -------
                "te" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "t", &original));
                }
                // ------- se → s' -------
                "se" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "s", &original));
                }
                // ------- ne → n' -------
                "ne" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "n", &original));
                }
                // ------- que → qu' -------
                "que" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "qu", &original));
                }
                // ------- si + il/ils → s'il/s'ils -------
                "si" if next_lower == "il" || next_lower == "ils" => {
                    suggestions.push(suggest_elision(tokens, i, j, "s", &original));
                }
                // ------- lorsque/puisque/quoique → forme élidée -------
                "lorsque" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "lorsqu", &original));
                }
                "puisque" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "puisqu", &original));
                }
                "quoique" if starts_with_vowel_or_mute_h(&next.text) => {
                    suggestions.push(suggest_elision(tokens, i, j, "quoiqu", &original));
                }
                // ------- ce + voyelle → cet -------
                "ce" if starts_with_vowel_or_mute_h(&next.text) => {
                    // Vérifier que le mot suivant est nominal (nom/adj) pour
                    // éviter "ce à quoi" → "cet à quoi".
                    let morphs = morpho::lookup(&next.text);
                    let is_nominal = morphs.iter().any(|m| {
                        matches!(
                            m.category,
                            morpho::MorphCategory::Noun | morpho::MorphCategory::Adjective
                        )
                    });
                    if is_nominal || morphs.is_empty() {
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: format!(
                                "Déterminant démonstratif : «\u{a0}ce\u{a0}» devant une voyelle s'écrit «\u{a0}cet\u{a0}»."
                            ),
                            replacements: vec!["cet".to_string()],
                            rule_id: RULE_ID,
                        });
                    }
                }
                _ => {}
            }
        }

        // ------- Élision fautive : l' + h aspiré → le/la -------
        for i in 0..n {
            let tok = &tokens[i];
            if tok.kind != TokenKind::Elision {
                continue;
            }
            let lower = norm(&tok.text);
            if lower != "l" {
                continue;
            }
            // Prochain token lexical
            let Some(j) = (i + 1..n).find(|&k| tokens[k].kind != TokenKind::Whitespace) else {
                continue;
            };
            let next = &tokens[j];
            if next.kind != TokenKind::Word {
                continue;
            }
            let next_base = next.text.to_lowercase();
            let next_base = next_base.split('-').next().unwrap_or(&next_base);
            if !H_ASPIRES.contains(&next_base) {
                continue;
            }
            // Déterminer le genre du nom pour proposer le ou la
            let morphs = morpho::lookup(&next.text);
            let mut replacements = Vec::new();
            let has_masc = morphs.iter().any(|m| m.gender == Some(morpho::Gender::Masculine));
            let has_fem = morphs.iter().any(|m| m.gender == Some(morpho::Gender::Feminine));
            if has_masc || (!has_masc && !has_fem) {
                replacements.push(format!("le {}", next.text));
            }
            if has_fem {
                replacements.push(format!("la {}", next.text));
            }
            if replacements.is_empty() {
                replacements.push(format!("le {}", next.text));
            }
            let span = Span::new(tok.span.start, next.span.end);
            suggestions.push(Suggestion {
                span,
                message: format!(
                    "Élision fautive : «\u{a0}{}{}'\u{a0}» — «\u{a0}{}\u{a0}» a un h aspiré.",
                    tok.text, next.text, next.text
                ),
                replacements,
                rule_id: RULE_ID,
            });
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Élision obligatoire"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn first_replacement(text: &str) -> Option<String> {
        ElisionRule
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        ElisionRule.check(&tokenize(text)).len()
    }

    // --- le/la → l' ---

    #[test]
    fn le_before_vowel() {
        assert_eq!(first_replacement("le arbre"), Some("l'arbre".into()));
        assert_eq!(first_replacement("le enfant"), Some("l'enfant".into()));
    }

    #[test]
    fn la_before_vowel() {
        assert_eq!(first_replacement("la école"), Some("l'école".into()));
        assert_eq!(first_replacement("la amie"), Some("l'amie".into()));
    }

    #[test]
    fn le_before_mute_h() {
        assert_eq!(first_replacement("le homme"), Some("l'homme".into()));
        assert_eq!(first_replacement("le hôpital"), Some("l'hôpital".into()));
    }

    // --- de → d' ---

    #[test]
    fn de_before_vowel() {
        assert_eq!(first_replacement("un verre de eau"), Some("d'eau".into()));
        assert_eq!(first_replacement("de argent"), Some("d'argent".into()));
    }

    // --- je → j' ---

    #[test]
    fn je_before_vowel() {
        assert_eq!(first_replacement("je ai faim"), Some("j'ai".into()));
        assert_eq!(first_replacement("je arrive"), Some("j'arrive".into()));
    }

    // --- si + il → s'il ---

    #[test]
    fn si_il() {
        assert_eq!(first_replacement("si il vient"), Some("s'il".into()));
        assert_eq!(first_replacement("si ils viennent"), Some("s'ils".into()));
    }

    // --- que → qu' ---

    #[test]
    fn que_before_vowel() {
        assert_eq!(first_replacement("il dit que il"), Some("qu'il".into()));
        assert_eq!(first_replacement("que on sache"), Some("qu'on".into()));
    }

    // --- ce → cet ---

    #[test]
    fn ce_before_vowel_nominal() {
        assert_eq!(first_replacement("ce arbre"), Some("cet".into()));
        assert_eq!(first_replacement("ce homme"), Some("cet".into()));
    }

    // --- h aspiré : pas d'élision ---

    #[test]
    fn no_elision_before_aspire() {
        assert_eq!(count("le héros"), 0, "le héros correct");
        assert_eq!(count("le hibou"), 0, "le hibou correct");
        assert_eq!(count("le haricot"), 0, "le haricot correct");
        assert_eq!(count("le hasard"), 0, "le hasard correct");
    }

    // --- élision fautive avant h aspiré ---

    #[test]
    fn wrong_elision_before_aspire() {
        let repls: Vec<String> = ElisionRule
            .check(&tokenize("l'héros"))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect();
        assert!(
            repls.iter().any(|r| r == "le héros"),
            "doit proposer 'le héros', got {repls:?}"
        );
    }

    // --- pas de faux positifs sur formes déjà élidées ---

    #[test]
    fn already_elided_no_trigger() {
        assert_eq!(count("l'arbre"), 0);
        assert_eq!(count("d'eau"), 0);
        assert_eq!(count("j'arrive"), 0);
        assert_eq!(count("s'il vient"), 0);
        assert_eq!(count("qu'il dise"), 0);
    }

    // --- ne → n' ---

    #[test]
    fn ne_before_vowel() {
        assert_eq!(first_replacement("je ne ai pas"), Some("n'ai".into()));
    }

    // --- me → m' ---

    #[test]
    fn me_before_vowel() {
        assert_eq!(first_replacement("il me a dit"), Some("m'a".into()));
    }
}
