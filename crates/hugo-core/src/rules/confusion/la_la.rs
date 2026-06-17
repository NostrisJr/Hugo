//! Règle : confusion **la / là / l'a** — **tranche 3** du moteur de confusions de
//! la phase 6 (cf.
//! [`corpus/confusion-la-la.md`](../../../../../corpus/confusion-la-la.md)).
//!
//! Adossée au CRF ([`crate::pos`]). Mémo (Projet Voltaire) :
//! - **la** est l'article ou le pronom féminin (« **la** table », « il **la**
//!   voit ») ;
//! - **là** est l'adverbe de lieu (« reste **là** », « ce jour-**là** ») ;
//! - **l'a** = pronom élidé « l' » + « a » (*avoir*) : « il **l'a** vu ».
//!
//! ## là → la (« là maison » → « **la** maison »)
//!
//! « là » immédiatement suivi d'un **nom** (étiqueté nom par le CRF) occupe la
//! place de l'article « la ». Comme « là » n'est l'homophone que de « la »
//! (article **féminin**) — un nom masculin réclamerait « le », autre correction
//! — on exclut les noms **explicitement masculins** ; les noms féminins ou non
//! marqués en genre au lexique (« maison », « voiture »…) sont corrigés en
//! « la ». Cela écarte aussi « là-haut »/« là où » (pas de nom à droite).
//!
//! ## la → l'a (« il la mangé » → « il **l'a** mangé »)
//!
//! Un **sujet de 3ᵉ personne** (`il/elle/on`, relatif `qui`, ou un nom/nom
//! propre) suivi de « la » puis d'un **participe passé** (adverbes sautés) : « la »
//! n'est ni article (pas de nom) ni objet d'un verbe conjugué — c'est le pronom
//! élidé suivi de l'auxiliaire, « l'a ».
//!
//! Limites assumées : la→là **adverbial** (« viens la » → « là ») se heurte à
//! l'homographie article/pronom objet ; là→la devant un nom **masculin**
//! (correction « le ») n'est pas traité.

use super::{is_past_participle, match_case, normalize, upos};
use crate::morpho::{self, Gender, MorphCategory};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions « la » / « là » / « l'a ».
pub struct LaConfusion;

const RULE_ID: &str = "confusion_la";

/// Sujets de 3ᵉ personne du singulier (pronoms forts) licenciant « l'a ».
const THIRD_SINGULAR_PRONOUNS: &[&str] = &["il", "elle", "on"];

/// Adverbes/négations sautés entre « la » et le participe passé.
const SKIP_BEFORE_PARTICIPLE: &[&str] = &["en", "y", "déjà", "bien", "presque", "vite", "tout"];

/// Vrai si `form` a une lecture de **nom masculin** au lexique (gage qu'il
/// réclamerait « le », non « la »).
fn has_masculine_noun_reading(form: &str) -> bool {
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == MorphCategory::Noun && m.gender == Some(Gender::Masculine))
}

/// Cherche un participe passé à droite de la position `i` (le « la »), en sautant
/// quelques adverbes (« il la déjà vu »). Le test [`is_past_participle`] (verbe
/// sans personne, porteur d'un genre/nombre) écarte déjà les formes **finies**
/// présentes (« il la voit » : « la » est l'objet d'un verbe conjugué, à laisser).
///
/// Le candidat doit en outre être **étiqueté `VERB`** par le CRF : cela écarte
/// les homographes au participe parasite au lexique (« la **plus** visible » :
/// « plus » a une lecture « participe » fantôme mais le CRF l'étiquette `ADV`,
/// superlatif — ce n'est pas « l'a »).
fn participle_follows(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> bool {
    let mut k = i + 1;
    while let Some((_, tok)) = sentence.get(k) {
        if SKIP_BEFORE_PARTICIPLE.contains(&normalize(&tok.text).as_str()) {
            k += 1;
            continue;
        }
        return upos(sentence, k, tags) == Upos::Verb && is_past_participle(&tok.text);
    }
    false
}

/// Vrai si `form` est un infinitif (sa forme équivaut à son lemme verbal).
fn is_infinitive(form: &str) -> bool {
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == MorphCategory::Verb && m.lemma.eq_ignore_ascii_case(form))
}

/// Correction d'un « là » en « la » : devant un nom non explicitement masculin,
/// ou devant un infinitif précédé d'un verbe conjugué (« elle va là voir » →
/// « elle va la voir » — « là » est ici un pronom COD, non un adverbe de lieu).
fn correction_la_grave(
    sentence: &[(usize, &Token)],
    i: usize,
    tags: &[Tagged],
) -> Option<&'static str> {
    let next = sentence.get(i + 1)?;
    // Cas 1 : là + NOM (article confondu avec adverbe de lieu).
    if upos(sentence, i + 1, tags) == Upos::Noun && !has_masculine_noun_reading(&next.1.text) {
        return Some("la");
    }
    // Cas 2 : VERBE_FINI + là + INFINITIF → « la » pronom COD.
    // « Elle va là voir » → « Elle va la voir ».
    if i > 0
        && upos(sentence, i + 1, tags) == Upos::Verb
        && is_infinitive(&next.1.text)
        && matches!(upos(sentence, i - 1, tags), Upos::Verb | Upos::Aux)
    {
        return Some("la");
    }
    None
}

/// Correction d'un « la » en « l'a » : sujet 3ᵉ pers. + la + participe passé.
fn correction_la(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<&'static str> {
    if i == 0 || !participle_follows(sentence, i, tags) {
        return None;
    }
    let subj = normalize(sentence[i - 1].1.text.as_str());
    let is_third = THIRD_SINGULAR_PRONOUNS.contains(&subj.as_str())
        || subj == "qui"
        || matches!(upos(sentence, i - 1, tags), Upos::Noun | Upos::Propn);
    is_third.then_some("l'a")
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « la »/« là »/« l'a » : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for LaConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let corrected = match normalize(sentence[i].1.text.as_str()).as_str() {
                    "là" => correction_la_grave(&sentence, i, tags),
                    "la" => correction_la(&sentence, i, tags),
                    _ => None,
                };
                if let Some(c) = corrected {
                    suggestions.push(suggestion(sentence[i].1, c));
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « la » / « là » / « l'a »"
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
        LaConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        LaConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- là → la ---

    #[test]
    fn la_grave_to_la_before_feminine_noun() {
        assert_eq!(first("là maison est belle").as_deref(), Some("la"));
        assert_eq!(first("là montagne est haute").as_deref(), Some("la"));
    }

    // --- la → l'a ---

    #[test]
    fn la_to_l_a_with_participle() {
        assert_eq!(first("il la mangé").as_deref(), Some("l'a"));
        assert_eq!(first("elle la vu hier").as_deref(), Some("l'a"));
        assert_eq!(first("on la cassé en tombant").as_deref(), Some("l'a"));
        assert_eq!(first("le chat la attrapé").as_deref(), Some("l'a"));
    }

    #[test]
    fn la_to_l_a_skips_adverb() {
        assert_eq!(first("il la déjà vu").as_deref(), Some("l'a"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "la maison est belle",    // article correct
            "il la voit chaque jour", // pronom objet + verbe conjugué
            "il l'a vu hier",         // « l'a » déjà correct
            "reste là un moment",     // adverbe de lieu correct
            "elle la regarde",        // pronom objet + verbe conjugué
            "je la mange",            // pronom objet (1ʳᵉ pers., pas de « l'a »)
            "viens là",               // adverbe de lieu (gap la→là assumé)
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn superlative_la_plus_is_not_l_a() {
        // « la plus visible » : superlatif (article + « plus » ADV + adjectif).
        // « plus » a un participe fantôme au lexique, mais le CRF l'étiquette
        // ADV → pas de fausse correction « l'a ».
        assert_eq!(count("la partie la plus visible de toutes"), 0);
        assert_eq!(count("c'est la plus belle de la classe"), 0);
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Là maison est belle").as_deref(), Some("La"));
    }
}
