//! Règle : accord déterminant–nom (genre et nombre).
//!
//! **Stub** (phase 2) : nécessitera l'analyse morphologique du Lefff pour
//! comparer genre et nombre du déterminant et du nom. Renvoie pour l'instant
//! une liste vide.

use super::Rule;
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord en genre et en nombre entre un déterminant et le nom qu'il
/// introduit (« un belle maison » → « une belle maison »).
pub struct DeterminerNounAgreement;

impl Rule for DeterminerNounAgreement {
    fn check(&self, _tokens: &[Token]) -> Vec<Suggestion> {
        // TODO(phase 2) : implémenter avec la morphologie du Lefff.
        Vec::new()
    }

    fn name(&self) -> &'static str {
        "Accord déterminant–nom"
    }

    fn id(&self) -> &'static str {
        "determiner_noun_agreement"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn stub_returns_empty() {
        let tokens = tokenize("un belle maison");
        assert!(DeterminerNounAgreement.check(&tokens).is_empty());
    }
}
