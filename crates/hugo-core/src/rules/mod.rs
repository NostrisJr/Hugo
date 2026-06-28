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

pub mod adjectif_verbal;
pub mod agreement;
pub mod attribute;
pub mod capitalization;
pub mod confusion;
pub mod conjugation;
pub mod demi;
pub mod detached_appositive;
pub mod duplicates;
pub mod elision;
pub mod epithet;
pub mod homophones;
pub mod imperatif;
pub mod locutions;
pub mod numeraux;
pub mod passive_participle;
pub mod past_participle;
pub mod pronominal_participle;
pub mod quantifier;
pub mod special_agreement;
pub mod subjunctive;
pub mod trait_union;
pub mod typography;

use crate::morpho::Morph;
use crate::pos::Tagged;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Vrai si un nom est **invariable en nombre** : sa forme du singulier (le
/// lemme) se termine déjà par -s/-x/-z, si bien que le pluriel est identique
/// (« cours », « fois », « prix », « nez »). De tels noms ne doivent pas
/// déclencher d'accord en **nombre** — ils sont compatibles avec le singulier
/// comme le pluriel. Partagé par les accords déterminant–nom et épithète.
pub(crate) fn is_number_invariable(noun_analyses: &[Morph]) -> bool {
    noun_analyses
        .iter()
        .any(|m| matches!(m.lemma.chars().next_back(), Some('s' | 'x' | 'z')))
}

/// Une règle de correction grammaticale.
///
/// Les implémentations doivent être `Send + Sync` afin que le
/// [`Checker`](crate::Checker) puisse être partagé entre threads.
pub trait Rule: Send + Sync {
    /// Analyse les tokens et renvoie les suggestions détectées.
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion>;

    /// Variante recevant en plus les étiquettes POS désambiguïsées par le CRF
    /// ([`crate::pos::tag`]), **alignées** sur `tokens` (`tags[i]` étiquette
    /// `tokens[i]`).
    ///
    /// Par défaut, ignore les tags et délègue à [`Rule::check`] : les règles
    /// historiques (heuristiques sur la morphologie brute) continuent de
    /// fonctionner inchangées. Les règles qui ont besoin d'une catégorie unique
    /// (ou/où, se/ce, accord nominal au-delà du premier homographe…) surchargent
    /// cette méthode.
    fn check_tagged(&self, tokens: &[Token], _tags: &[Tagged]) -> Vec<Suggestion> {
        self.check(tokens)
    }

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
        Box::new(pronominal_participle::PronominalParticiple),
        Box::new(epithet::EpithetAdjectiveAgreement),
        Box::new(quantifier::ToutAgreement),
        Box::new(special_agreement::SpecialAgreement),
        Box::new(past_participle::PastParticipleAvoir),
        Box::new(passive_participle::PassiveParticiple),
        Box::new(subjunctive::SubjunctiveAfterConjunction),
        Box::new(confusion::AAConfusion),
        Box::new(confusion::CeSeConfusion),
        Box::new(confusion::OuConfusion),
        Box::new(confusion::LaConfusion),
        Box::new(confusion::LeurConfusion),
        Box::new(confusion::PeuConfusion),
        Box::new(confusion::QuelConfusion),
        Box::new(confusion::QuandConfusion),
        Box::new(confusion::SansConfusion),
        Box::new(confusion::TerminaisonsConfusion),
        Box::new(homophones::HomophoneRule),
        Box::new(imperatif::ImperatifGroupe1),
        // Phase 8 — élisions et contractions obligatoires.
        Box::new(elision::ElisionRule),
        // Phase 9 — confusions et homophones restants.
        Box::new(confusion::EtEstConfusion),
        Box::new(confusion::DontDoncConfusion),
        Box::new(confusion::SaCaConfusion),
        Box::new(confusion::NiNyConfusion),
        Box::new(confusion::PresPreConfusion),
        Box::new(confusion::DansDenConfusion),
        Box::new(confusion::PlutotConfusion),
        Box::new(confusion::AccentsConfusion),
        // Phase 10 — accords avancés, traits d'union, numéraux.
        Box::new(numeraux::NumerauxRule),
        Box::new(demi::DemiRule),
        Box::new(locutions::LocutionsRule),
        Box::new(trait_union::TraitUnionRule),
        Box::new(adjectif_verbal::AdjectifVerbalRule),
        // Phase 12 — apposition détachée avec sujet postposé.
        Box::new(detached_appositive::DetachedAppositive),
        // Phase 7 — typographie, ponctuation, espaces, nombres (déterministe).
        Box::new(typography::PunctDoublingRule),
        Box::new(typography::SpacingRule),
        Box::new(typography::LigatureRule),
        Box::new(typography::OrdinalRule),
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
/// point-virgule, deux-points, ou backslash utilisé comme saut de ligne doux).
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
