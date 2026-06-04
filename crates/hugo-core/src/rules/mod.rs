//! Moteur de règles grammaticales.
//!
//! Chaque règle implémente le trait [`Rule`] : elle reçoit la liste des
//! [`Token`]s d'un texte et renvoie les [`Suggestion`]s détectées. Le
//! [`Checker`](crate::Checker) agrège les résultats de [`all_rules`].
//!
//! Les règles d'accord ([`agreement`], [`conjugation`], [`attribute`],
//! [`epithet`], [`quantifier`]) et d'homophonie ([`homophones`]) reposent sur l'analyse morphologique
//! ([`crate::morpho`]). Les règles purement positionnelles ([`duplicates`],
//! [`capitalization`]) n'en dépendent pas.
//!
//! Les règles qui inspectent le voisinage d'un mot raisonnent **par phrase**
//! via [`lexical_sentences`], afin que le dernier mot d'une phrase ne soit pas
//! pris pour le voisin du premier mot de la suivante.

pub mod agreement;
pub mod attribute;
pub mod capitalization;
pub mod conjugation;
pub mod duplicates;
pub mod epithet;
pub mod homophones;
pub mod quantifier;

use crate::tokenizer::{Token, TokenKind};
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
        Box::new(attribute::AttributeAdjectiveAgreement),
        Box::new(epithet::EpithetAdjectiveAgreement),
        Box::new(quantifier::ToutAgreement),
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

/// Vrai si le jeton est une ponctuation marquant une frontière de phrase
/// (point, point d'exclamation/d'interrogation, points de suspension,
/// point-virgule, deux-points).
fn is_sentence_terminator(token: &Token) -> bool {
    token.kind == TokenKind::Punctuation
        && matches!(token.text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
}

/// Découpe les tokens en phrases, et renvoie pour chacune ses jetons lexicaux
/// (avec leur index d'origine, comme [`lexical_tokens`]).
///
/// Les règles qui inspectent le **jeton précédent** (sujet, préposition…)
/// doivent raisonner par phrase : sans cela, le dernier mot d'une phrase
/// « fuit » sur la première de la suivante (« Il dort. Les chats… » verrait
/// « dort » comme voisin de « Les »). Les segments vides sont omis.
pub(crate) fn lexical_sentences(tokens: &[Token]) -> Vec<Vec<(usize, &Token)>> {
    let mut sentences = Vec::new();
    let mut current = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if is_sentence_terminator(token) {
            if !current.is_empty() {
                sentences.push(std::mem::take(&mut current));
            }
        } else if token.is_lexical() {
            current.push((i, token));
        }
    }
    if !current.is_empty() {
        sentences.push(current);
    }
    sentences
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

    #[test]
    fn lexical_sentences_splits_on_terminators() {
        let tokens = crate::tokenizer::tokenize("Il dort. Les chats mangent !");
        let sentences = lexical_sentences(&tokens);
        let texts: Vec<Vec<&str>> = sentences
            .iter()
            .map(|s| s.iter().map(|(_, t)| t.text.as_str()).collect())
            .collect();
        assert_eq!(
            texts,
            vec![vec!["Il", "dort"], vec!["Les", "chats", "mangent"]]
        );
    }

    #[test]
    fn lexical_sentences_preserve_origin_index() {
        let tokens = crate::tokenizer::tokenize("Le chat dort.");
        let sentences = lexical_sentences(&tokens);
        // Les index renvoyés pointent dans la tranche d'origine.
        for sentence in &sentences {
            for &(i, t) in sentence {
                assert_eq!(&tokens[i], t);
            }
        }
    }
}
