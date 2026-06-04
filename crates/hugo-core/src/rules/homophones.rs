//! Règle : homophones grammaticaux (a/à, ce/se, mes/mais, son/sont, on/ont).
//!
//! La levée d'ambiguïté fiable réclame le contexte morphosyntaxique. Tant que
//! le désambiguïsateur POS (CRF, phase 4) n'est pas disponible, on ne traite
//! que des configurations **à très haute précision**, déclenchées par le
//! voisinage immédiat (jeton précédent / suivant) au sein d'une même phrase :
//!
//! - **a → à** : « a » précédé d'un verbe conjugué (autre qu'`avoir`) ne peut
//!   être l'auxiliaire (« il va a Paris » → « à ») ;
//! - **à → a / ont** : « à » précédé d'un pronom sujet (« il à faim » → « a »,
//!   « ils à mangé » → « ont ») ;
//! - **on → ont** / **son → sont** : forme précédée de `ils`/`elles`
//!   (« ils on mangé » → « ont », « ils son partis » → « sont ») ;
//! - **ont → on** : « ont » en tête de phrase suivi d'un verbe conjugué
//!   (« Ont va au cinéma » → « On ») ;
//! - **ce → se** : « ce » entre un pronom sujet et un verbe (« il ce lève » →
//!   « se ») ;
//! - **mes → mais** : « mes » suivi d'un pronom sujet (« mes je ne sais pas » →
//!   « mais »).
//!
//! Les confusions plus ambiguës (ou/où, se/ce, sont/son…) attendent la
//! désambiguïsation POS de la phase 4.

use super::{lexical_sentences, Rule};
use crate::morpho;
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions d'homophones grammaticaux fréquents.
pub struct HomophoneRule;

const RULE_ID: &str = "homophone";

/// Minuscules + apostrophe finale ôtée.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Vrai si le jeton est un pronom personnel sujet.
fn is_subject_pronoun(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "je" | "j" | "tu" | "il" | "elle" | "on" | "nous" | "vous" | "ils" | "elles"
    )
}

/// Vrai si le jeton est un pronom personnel sujet de 3ᵉ personne du pluriel.
fn is_third_plural_pronoun(text: &str) -> bool {
    matches!(normalize(text).as_str(), "ils" | "elles")
}

/// Vrai si le jeton admet une analyse verbale finie (forme conjuguée).
fn is_finite_verb(text: &str) -> bool {
    !morpho::verb_forms(text).is_empty()
}

/// Vrai si le jeton est un verbe conjugué dont aucun lemme n'est `avoir`
/// (il ne peut donc pas tenir le rôle de l'auxiliaire « a »).
fn is_finite_verb_not_avoir(text: &str) -> bool {
    let forms = morpho::verb_forms(text);
    !forms.is_empty() && !forms.iter().any(|v| v.lemma == "avoir")
}

/// Calque la casse initiale de `original` sur `replacement`.
fn match_case(original: &str, replacement: &str) -> String {
    if !original.chars().next().is_some_and(|c| c.is_uppercase()) {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => replacement.to_string(),
    }
}

/// Cherche la correction d'homophone pour le jeton `i` de la phrase, d'après son
/// voisinage. Renvoie la forme corrigée (avant calque de casse), le cas échéant.
fn correction(sentence: &[(usize, &Token)], i: usize) -> Option<&'static str> {
    let cur = normalize(sentence[i].1.text.as_str());
    let prev = (i > 0).then(|| normalize(sentence[i - 1].1.text.as_str()));
    let next = sentence
        .get(i + 1)
        .map(|(_, t)| normalize(t.text.as_str()));
    let prev_text = (i > 0).then(|| sentence[i - 1].1.text.as_str());
    let next_text = sentence.get(i + 1).map(|(_, t)| t.text.as_str());

    match cur.as_str() {
        // a → à : « a » précédé d'un verbe conjugué non-`avoir`.
        "a" if prev_text.is_some_and(is_finite_verb_not_avoir) => Some("à"),

        // à → a / ont : « à » précédé d'un pronom sujet.
        "à" => match prev.as_deref() {
            Some("il" | "elle" | "on") => Some("a"),
            Some("ils" | "elles") => Some("ont"),
            _ => None,
        },

        // on → ont : « on » précédé de ils/elles.
        "on" if prev_text.is_some_and(is_third_plural_pronoun) => Some("ont"),

        // ont → on : « ont » en tête de phrase suivi d'un verbe conjugué.
        "ont" if i == 0 && next_text.is_some_and(is_finite_verb) => Some("on"),

        // son → sont : « son » précédé de ils/elles.
        "son" if prev_text.is_some_and(is_third_plural_pronoun) => Some("sont"),

        // ce → se : « ce » entre un pronom sujet et un verbe conjugué.
        "ce" if prev_text.is_some_and(is_subject_pronoun)
            && next_text.is_some_and(is_finite_verb) =>
        {
            Some("se")
        }

        // mes → mais : « mes » suivi d'un pronom sujet.
        "mes" if next.as_deref().is_some_and(is_subject_pronoun) => Some("mais"),

        _ => None,
    }
}

impl Rule for HomophoneRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let Some(corrected) = correction(&sentence, i) else {
                    continue;
                };
                let token = sentence[i].1;
                suggestions.push(Suggestion {
                    span: token.span,
                    message: format!(
                        "Confusion d'homophones : « {} » devrait être « {} ».",
                        token.text, corrected
                    ),
                    replacements: vec![match_case(&token.text, corrected)],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Homophones grammaticaux"
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
        HomophoneRule
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        HomophoneRule.check(&tokenize(text)).len()
    }

    #[test]
    fn a_to_a_grave_after_verb() {
        assert_eq!(first("il va a Paris").as_deref(), Some("à"));
        assert_eq!(first("elle pense a lui").as_deref(), Some("à"));
        assert_eq!(first("il commence a manger").as_deref(), Some("à"));
    }

    #[test]
    fn a_grave_to_a_after_pronoun() {
        assert_eq!(first("il à faim").as_deref(), Some("a"));
        assert_eq!(first("elle à un chat").as_deref(), Some("a"));
        assert_eq!(first("ils à mangé").as_deref(), Some("ont"));
    }

    #[test]
    fn on_to_ont_after_plural() {
        assert_eq!(first("ils on mangé").as_deref(), Some("ont"));
        assert_eq!(first("elles on compris").as_deref(), Some("ont"));
    }

    #[test]
    fn ont_to_on_sentence_initial() {
        assert_eq!(first("Ont va au cinéma").as_deref(), Some("On"));
    }

    #[test]
    fn son_to_sont_after_plural() {
        assert_eq!(first("ils son partis").as_deref(), Some("sont"));
    }

    #[test]
    fn ce_to_se_between_subject_and_verb() {
        assert_eq!(first("il ce lève").as_deref(), Some("se"));
    }

    #[test]
    fn mes_to_mais_before_pronoun() {
        assert_eq!(first("mes je ne sais pas").as_deref(), Some("mais"));
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Il va a Paris").as_deref(), Some("à"));
    }

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "il a faim",            // « a » auxiliaire, prev pronom
            "il va à Paris",        // « à » déjà correct
            "il pense à lui",       // « à » après un verbe : correct
            "on a mangé",           // « on » sujet
            "ils ont mangé",        // « ont » correct
            "son chat dort",        // « son » possessif
            "ce livre est lourd",   // « ce » démonstratif en tête
            "mes amis sont là",     // « mes » possessif + nom
            "il se lève",           // « se » déjà correct
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn no_cross_sentence_leak() {
        // « va » (phrase 1) ne doit pas déclencher « a » de la phrase 2.
        assert_eq!(count("il va. a b c"), 0);
    }
}
