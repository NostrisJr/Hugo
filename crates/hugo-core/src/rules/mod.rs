//! Moteur de règles grammaticales.
//!
//! Chaque règle implémente le trait [`Rule`] : elle reçoit la liste des
//! [`Token`]s d'un texte et renvoie les [`Suggestion`]s détectées. Le
//! [`Checker`](crate::Checker) agrège les résultats de [`all_rules`].
//!
//! Les règles d'accord ([`agreement`], [`conjugation`]) et d'homophonie
//! ([`homophones`]) reposeront sur l'analyse morphologique ; elles sont pour
//! l'instant à l'état de stubs. Les règles purement positionnelles
//! ([`duplicates`], [`capitalization`]) sont déjà fonctionnelles.

pub mod agreement;
pub mod capitalization;
pub mod conjugation;
pub mod duplicates;
pub mod homophones;

use crate::tokenizer::Token;
use crate::Suggestion;

/// Une règle de correction grammaticale.
///
/// Les implémentations doivent être `Send + Sync` afin que le
/// [`Checker`](crate::Checker) puisse être partagé entre threads.
pub trait Rule: Send + Sync {
    /// Analyse les tokens et renvoie les suggestions détectées.
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion>;

    /// Nom lisible de la règle.
    fn name(&self) -> &'static str;

    /// Identifiant stable de la règle (réutilisé dans `Suggestion::rule_id`).
    fn id(&self) -> &'static str;
}

/// Retourne l'ensemble des règles actives.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(duplicates::DuplicateWord),
        Box::new(capitalization::CapitalizationAfterPeriod),
        Box::new(agreement::DeterminerNounAgreement),
        Box::new(conjugation::SubjectVerbAgreement),
        Box::new(homophones::HomophoneRule),
    ]
}

/// Itère sur les tokens lexicaux (mots et élisions) en conservant leur index
/// d'origine dans la tranche complète. Utilitaire partagé par les règles qui
/// raisonnent sur la suite des mots en ignorant espaces et ponctuation.
pub(crate) fn lexical_tokens(tokens: &[Token]) -> Vec<(usize, &Token)> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.is_lexical())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rules_have_unique_ids() {
        let rules = all_rules();
        let mut ids: Vec<&str> = rules.iter().map(|r| r.id()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "identifiants de règles dupliqués");
    }

    #[test]
    fn lexical_tokens_skips_non_lexical() {
        let tokens = crate::tokenizer::tokenize("Le chat, lui.");
        let lex = lexical_tokens(&tokens);
        let texts: Vec<&str> = lex.iter().map(|(_, t)| t.text.as_str()).collect();
        assert_eq!(texts, vec!["Le", "chat", "lui"]);
    }
}
