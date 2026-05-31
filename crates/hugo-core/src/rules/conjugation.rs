//! Règle : accord sujet–verbe (personne et nombre).
//!
//! **Stub** (phase 2) : nécessitera l'analyse morphologique du Lefff pour
//! comparer la personne et le nombre du sujet et du verbe. Renvoie pour
//! l'instant une liste vide.

use super::Rule;
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord en personne et en nombre entre un sujet et son verbe
/// (« ils mange » → « ils mangent »), sans se déclencher sur les inversions
/// (« mange-t-il »).
pub struct SubjectVerbAgreement;

impl Rule for SubjectVerbAgreement {
    fn check(&self, _tokens: &[Token]) -> Vec<Suggestion> {
        // TODO(phase 2) : implémenter avec la morphologie du Lefff.
        Vec::new()
    }

    fn name(&self) -> &'static str {
        "Accord sujet–verbe"
    }

    fn id(&self) -> &'static str {
        "subject_verb_agreement"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn stub_returns_empty() {
        let tokens = tokenize("ils mange");
        assert!(SubjectVerbAgreement.check(&tokens).is_empty());
    }
}
