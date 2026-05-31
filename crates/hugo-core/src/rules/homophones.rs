//! Règle : homophones grammaticaux (a/à, ou/où, ce/se, son/sont…).
//!
//! **Stub** (phase 2) : la levée d'ambiguïté fiable réclame le contexte
//! morphosyntaxique (catégorie du mot voisin). La structure est posée ;
//! `check` renvoie pour l'instant une liste vide.

use super::Rule;
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions d'homophones grammaticaux fréquents
/// (« il va a Paris » → « il va à Paris »).
pub struct HomophoneRule;

impl Rule for HomophoneRule {
    fn check(&self, _tokens: &[Token]) -> Vec<Suggestion> {
        // TODO(phase 2) : implémenter avec le contexte morphosyntaxique.
        Vec::new()
    }

    fn name(&self) -> &'static str {
        "Homophones grammaticaux"
    }

    fn id(&self) -> &'static str {
        "homophone"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn stub_returns_empty() {
        let tokens = tokenize("il va a Paris");
        assert!(HomophoneRule.check(&tokens).is_empty());
    }
}
