//! Règle : **ligatures `œ`**.
//!
//! Remplace la suite `oe` par la ligature `œ` dans une **liste curée** de mots
//! où elle est obligatoire (`coeur` → `cœur`, `oeuf` → `œuf`, `soeur` →
//! `sœur`…). L'approche par liste fermée écarte par construction les mots où
//! `oe` n'est **pas** une ligature (`coexister`, `coefficient`, `moelle`,
//! `goéland`, `poêle`…), qui n'y figurent pas.
//!
//! La casse est respectée : `Coeur` → `Cœur`, `COEUR` → `CŒUR`.

use super::super::Rule;
use super::recase;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

const RULE_ID: &str = "typo_ligature";

/// Cf. [module](self).
pub struct LigatureRule;

/// Formes (minuscules) où `oe` est une ligature `œ` obligatoire. Liste
/// **originale** reconstituée pour ce projet, à enrichir au fil de l'eau.
const OE_WORDS: &[&str] = &[
    "coeur",
    "coeurs",
    "soeur",
    "soeurs",
    "oeuf",
    "oeufs",
    "oeuvre",
    "oeuvres",
    "oeuvrer",
    "voeu",
    "voeux",
    "noeud",
    "noeuds",
    "boeuf",
    "boeufs",
    "oeil",
    "oeillet",
    "oeillets",
    "oeillere",
    "oeilleres",
    "oeillère",
    "oeillères",
    "manoeuvre",
    "manoeuvres",
    "manoeuvrer",
    "oedeme",
    "oedemes",
    "oedème",
    "oedèmes",
    "oesophage",
    "oesophages",
    "oestrogene",
    "oestrogenes",
    "oestrogène",
    "oestrogènes",
    "foetus",
    "coeliaque",
    "coeliaques",
    "oecumenique",
    "oecuméniques",
    "oecuménique",
    "oenologie",
    "oenologue",
];

impl Rule for LigatureRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for token in tokens {
            if token.kind != TokenKind::Word {
                continue;
            }
            let lower = token.text.to_lowercase();
            if !OE_WORDS.contains(&lower.as_str()) {
                continue;
            }
            let corrected = recase(&token.text, &lower.replace("oe", "\u{0153}"));
            suggestions.push(Suggestion {
                span: token.span,
                message: format!("Ligature « œ » attendue : « {} ».", token.text),
                replacements: vec![corrected],
                rule_id: RULE_ID,
            });
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Ligature œ"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn repls(text: &str) -> Vec<String> {
        LigatureRule
            .check(&tokenize(text))
            .into_iter()
            .flat_map(|s| s.replacements)
            .collect()
    }

    #[test]
    fn common_words_are_ligatured() {
        assert_eq!(repls("mon coeur"), vec!["c\u{0153}ur"]);
        assert_eq!(repls("un oeuf"), vec!["\u{0153}uf"]);
        assert_eq!(repls("ma soeur"), vec!["s\u{0153}ur"]);
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(repls("Coeur"), vec!["C\u{0153}ur"]);
        assert_eq!(repls("COEUR"), vec!["C\u{0152}UR"]);
    }

    #[test]
    fn non_ligature_oe_is_spared() {
        assert!(repls("coexister avec la moelle").is_empty());
        assert!(repls("le coefficient").is_empty());
    }
}
