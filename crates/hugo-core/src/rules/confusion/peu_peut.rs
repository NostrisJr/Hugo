//! Règle : confusion **peu / peut / peux** — **tranche 3** du moteur de
//! confusions de la phase 6 (cf.
//! [`corpus/confusion-peu-peut.md`](../../../../../corpus/confusion-peu-peut.md)).
//!
//! Adossée au CRF ([`crate::pos`]). Mémo (Projet Voltaire) :
//! - **peu** est l'adverbe de quantité (« un **peu** », « **peu** de gens ») ;
//!   on peut le remplacer par « beaucoup ».
//! - **peut** / **peux** sont le verbe *pouvoir* : « il **peut** », « je/tu
//!   **peux** » ; remplaçables par « pouvait » / « pouvais ».
//!
//! ## peu → peut/peux (« il peu marcher » → « il **peut** marcher »)
//!
//! Un **pronom sujet** (`je/tu/il/elle/on`) directement suivi de « peu » puis
//! d'un **infinitif** (négation/adverbes sautés) appelle le verbe *pouvoir* :
//! l'adverbe « peu » n'introduit pas d'infinitif. La forme est choisie sur la
//! personne du sujet (`je/tu` → « peux », `il/elle/on` → « peut »).
//!
//! ## peut/peux → peu (« un peut de sel » → « un **peu** de sel »)
//!
//! Précédé d'un **quantifieur/déterminant** qui ne peut régir un verbe (`un`,
//! `très`, `trop`, `si`, `assez`…), ou d'un *avoir* immédiatement suivi de
//! « de » (« il a peut de temps »), « peut »/« peux » est l'adverbe « peu ».
//!
//! Limite assumée : la simple **confusion de personne** peux↔peut (« je peut »,
//! « il peux ») est un défaut d'accord déjà capté par l'accord sujet–verbe
//! ([`crate::rules::conjugation`]) ; on ne la double pas ici.

use super::{is_infinitive, match_case, normalize, upos};
use crate::morpho;
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions « peu » / « peut » / « peux ».
pub struct PeuConfusion;

const RULE_ID: &str = "confusion_peu";

/// Pronoms sujets singuliers dont *pouvoir* fait « peux » (1ʳᵉ/2ᵉ pers.).
const SUBJECTS_PEUX: &[&str] = &["je", "j", "tu"];

/// Pronoms sujets singuliers dont *pouvoir* fait « peut » (3ᵉ pers.).
const SUBJECTS_PEUT: &[&str] = &["il", "elle", "on"];

/// Quantifieurs / déterminants qui ne peuvent régir un verbe : un « peut »/
/// « peux » qui les suit est l'adverbe « peu ».
/// On exclut volontairement `plus`/`moins`/`combien`/`tout` : « tout peut
/// arriver » est correct, et `combien/plus peut-il` relèvent de l'inversion
/// (jeton à trait d'union, non concerné).
const QUANTIFIERS_BEFORE: &[&str] = &[
    "un",
    "une",
    "très",
    "trop",
    "si",
    "aussi",
    "tellement",
    "bien",
    "assez",
    "fort",
    "quelque",
];

/// Négations et adverbes courts sautés entre « peu » et l'infinitif.
const SKIP_BEFORE_INFINITIVE: &[&str] = &[
    "ne", "n", "pas", "plus", "jamais", "rien", "y", "en", "le", "la", "les", "lui", "leur", "me",
    "m", "te", "t", "se", "s", "nous", "vous",
];

/// Vrai si une analyse de `form` est le verbe *avoir* (forme finie).
fn is_avoir(form: &str) -> bool {
    morpho::verb_forms(form).iter().any(|v| v.lemma == "avoir")
}

/// Cherche un infinitif à droite de la position `i` (le « peu »), en sautant la
/// négation et les clitiques (« il ne peu pas venir »).
fn infinitive_follows(sentence: &[(usize, &Token)], i: usize) -> bool {
    let mut k = i + 1;
    while let Some((_, tok)) = sentence.get(k) {
        if SKIP_BEFORE_INFINITIVE.contains(&normalize(&tok.text).as_str()) {
            k += 1;
            continue;
        }
        return is_infinitive(&tok.text);
    }
    false
}

/// Correction d'un « peu » en « peut »/« peux » selon le sujet pronominal.
fn correction_peu(sentence: &[(usize, &Token)], i: usize) -> Option<&'static str> {
    if i == 0 || !infinitive_follows(sentence, i) {
        return None;
    }
    // Remonte au sujet en sautant la négation « ne »/« n' » (« il ne peu pas… »).
    let mut s = i - 1;
    while s > 0 && matches!(normalize(sentence[s].1.text.as_str()).as_str(), "ne" | "n") {
        s -= 1;
    }
    let subj = normalize(sentence[s].1.text.as_str());
    if SUBJECTS_PEUX.contains(&subj.as_str()) {
        Some("peux")
    } else if SUBJECTS_PEUT.contains(&subj.as_str()) {
        Some("peut")
    } else {
        None
    }
}

/// Correction d'un « peut »/« peux » en « peu » selon le voisinage.
fn correction_peut(
    sentence: &[(usize, &Token)],
    i: usize,
    tags: &[Tagged],
) -> Option<&'static str> {
    let prev = (i > 0).then(|| normalize(sentence[i - 1].1.text.as_str()));
    let next = sentence.get(i + 1).map(|(_, t)| normalize(&t.text));

    // Quantifieur / déterminant + peut/peux → peu (« un peut », « très peut »).
    if prev
        .as_deref()
        .is_some_and(|p| QUANTIFIERS_BEFORE.contains(&p))
    {
        return Some("peu");
    }

    // avoir + peut/peux + « de » → peu (« il a peut de temps »). Le veto sur le
    // tag du précédent (réellement *avoir*) évite « il peut de nouveau marcher ».
    if i > 0
        && upos(sentence, i - 1, tags) == Upos::Aux
        && prev.as_deref().is_some_and(is_avoir)
        && next.as_deref() == Some("de")
    {
        return Some("peu");
    }

    None
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « peu »/« peut »/« peux » : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for PeuConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let token = sentence[i].1;
                let corrected = match normalize(&token.text).as_str() {
                    "peu" => correction_peu(&sentence, i),
                    "peut" | "peux" => correction_peut(&sentence, i, tags),
                    _ => None,
                };
                if let Some(c) = corrected {
                    suggestions.push(suggestion(token, c));
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « peu » / « peut » / « peux »"
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
        PeuConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        PeuConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- peu → peut/peux ---

    #[test]
    fn peu_to_peut_with_third_person() {
        assert_eq!(first("il peu marcher").as_deref(), Some("peut"));
        assert_eq!(first("elle peu venir demain").as_deref(), Some("peut"));
        assert_eq!(first("on peu partir maintenant").as_deref(), Some("peut"));
    }

    #[test]
    fn peu_to_peux_with_first_second_person() {
        assert_eq!(first("je peu venir").as_deref(), Some("peux"));
        assert_eq!(first("tu peu partir").as_deref(), Some("peux"));
    }

    #[test]
    fn peu_to_peut_skips_negation() {
        assert_eq!(first("il ne peu pas venir").as_deref(), Some("peut"));
    }

    // --- peut/peux → peu ---

    #[test]
    fn peut_to_peu_after_quantifier() {
        assert_eq!(first("un peut de sel suffit").as_deref(), Some("peu"));
        assert_eq!(first("il y a très peut de monde").as_deref(), Some("peu"));
        assert_eq!(first("trop peut de gens le savent").as_deref(), Some("peu"));
    }

    #[test]
    fn peut_to_peu_after_avoir_before_de() {
        assert_eq!(first("il a peut de temps").as_deref(), Some("peu"));
        assert_eq!(first("elle a peut de patience").as_deref(), Some("peu"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "il peut marcher",          // « peut » verbe correct
            "je peux venir",            // « peux » verbe correct
            "il y a peu de monde",      // « peu » adverbe correct
            "un peu de sel",            // « peu » adverbe correct
            "il peut de nouveau jouer", // « peut » + « de nouveau » (avoir absent)
            "il a peu de temps",        // « peu » adverbe correct
            "elle peut partir",         // verbe correct
            "il mange peu",             // adverbe en fin de proposition
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn person_confusion_is_left_to_conjugation() {
        // « je peut » / « il peux » : défaut d'accord en personne, hors de cette
        // règle (capté par l'accord sujet–verbe). On ne signale rien ici.
        assert_eq!(count("je peut venir"), 0);
        assert_eq!(count("il peux partir"), 0);
    }

    #[test]
    fn lowercase_is_preserved() {
        // « peu » fautif minuscule en milieu de phrase → « peut » minuscule.
        assert_eq!(first("Il peu marcher").as_deref(), Some("peut"));
    }
}
