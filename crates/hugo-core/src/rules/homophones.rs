//! Règle : homophones grammaticaux (mes/mais, son/sont, on/ont).
//!
//! Les confusions **a/à** (tranche 1) et **ce/se, c'est/s'est** (tranche 2) ont
//! leurs propres règles, adossées au CRF ([`crate::rules::confusion`], phase 6).
//!
//! La levée d'ambiguïté fiable réclame le contexte morphosyntaxique. Un premier
//! jeu de corrections **à très haute précision** s'appuie sur le seul voisinage
//! immédiat (jeton précédent / suivant) au sein d'une même phrase :
//!
//! - **on → ont** / **son → sont** : forme précédée de `ils`/`elles`
//!   (« ils on mangé » → « ont », « ils son partis » → « sont ») ;
//! - **ont → on** : « ont » en tête de phrase suivi d'un verbe conjugué
//!   (« Ont va au cinéma » → « On ») ;
//! - **mes → mais** : « mes » suivi d'un pronom sujet (« mes je ne sais pas » →
//!   « mais »).
//!
//! Depuis la phase 4, la règle exploite aussi les **étiquettes POS** du CRF
//! ([`crate::pos`]) via [`Rule::check_tagged`] pour l'**étiquetage
//! contrefactuel** : lorsque l'étiqueteur « rattrape » l'erreur en donnant à la
//! phrase fautive une
//! lecture plausible (le POS du voisin ne trahit alors plus la faute), on recourt
//! à un **étiquetage contrefactuel** ([`COUNTERFACTUALS`]) : on compare le score
//! POS du texte tel quel à celui obtenu en substituant la graphie alternative ;
//! un écart net désigne la faute.
//!
//! - **son ↔ sont** : « mes parents son venus » → « sont » ; « il caresse sont
//!   chat » → « son ».
//!
//! La confusion **ou/où** n'a pas de signal séparable (l'étiqueteur accepte
//! « où » dans trop de coordinations correctes) et reste hors de portée.

use super::{lexical_sentences, Rule};
use crate::morpho;
use crate::pos::{self, Tagged};
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
    let next = sentence.get(i + 1).map(|(_, t)| normalize(t.text.as_str()));
    let prev_text = (i > 0).then(|| sentence[i - 1].1.text.as_str());
    let next_text = sentence.get(i + 1).map(|(_, t)| t.text.as_str());

    match cur.as_str() {
        // on → ont : « on » précédé de ils/elles.
        "on" if prev_text.is_some_and(is_third_plural_pronoun) => Some("ont"),

        // ont → on : « ont » en tête de phrase suivi d'un verbe conjugué.
        "ont" if i == 0 && next_text.is_some_and(is_finite_verb) => Some("on"),

        // son → sont : « son » précédé de ils/elles.
        "son" if prev_text.is_some_and(is_third_plural_pronoun) => Some("sont"),

        // mes → mais : « mes » suivi d'un pronom sujet.
        "mes" if next.as_deref().is_some_and(is_subject_pronoun) => Some("mais"),

        _ => None,
    }
}

/// Une règle d'**étiquetage contrefactuel** : si remplacer la graphie `from`
/// par `to` fait gagner au texte au moins `min_delta` de score POS (Viterbi),
/// c'est que `to` est la lecture nettement plus plausible — donc une faute.
struct Counterfactual {
    from: &'static str,
    to: &'static str,
    min_delta: f32,
}

/// Confusions tranchées par comparaison de score POS des deux graphies.
///
/// Les seuils sont calibrés empiriquement, ponctuation comprise (marge de
/// sécurité ≥ 1,5 vis-à-vis des emplois corrects observés). Ils diffèrent par
/// direction car les deux types de faute ont des signatures de score distinctes :
/// « sont » écrit pour « son » (déterminant devant nom) se détache très nettement
/// (≥ +7 vs ≤ +3,4 pour les emplois corrects), tandis que « son » écrit pour
/// « sont » (auxiliaire) donne un écart plus ténu (≥ +2 pour la plupart des
/// fautes vs ≤ −3 pour les emplois corrects). Conséquence assumée : une faute au
/// signal faible — « les enfants son partis », où « son partis » se lit comme un
/// groupe nominal plausible — n'est pas rattrapée.
const COUNTERFACTUALS: &[Counterfactual] = &[
    Counterfactual {
        from: "son",
        to: "sont",
        min_delta: 1.5,
    },
    Counterfactual {
        from: "sont",
        to: "son",
        min_delta: 5.5,
    },
];

/// Cherche une correction contrefactuelle pour le jeton d'index d'origine `idx`.
/// `base` est le score POS du texte inchangé (calculé une seule fois).
fn counterfactual(tokens: &[Token], idx: usize, base: f32) -> Option<&'static str> {
    let cur = normalize(tokens[idx].text.as_str());
    for rule in COUNTERFACTUALS {
        if cur != rule.from {
            continue;
        }
        let mut alt = tokens.to_vec();
        alt[idx].text = match_case(&tokens[idx].text, rule.to);
        if pos::best_score(&alt) - base > rule.min_delta {
            return Some(rule.to);
        }
    }
    None
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion d'homophones : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for HomophoneRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                if let Some(corrected) = correction(&sentence, i) {
                    suggestions.push(suggestion(sentence[i].1, corrected));
                }
            }
        }
        suggestions
    }

    fn check_tagged(&self, tokens: &[Token], _tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        // Score POS du texte inchangé, calculé au plus une fois (seulement si un
        // candidat à l'étiquetage contrefactuel se présente).
        let mut base: Option<f32> = None;
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let (idx, token) = sentence[i];
                // 1. Heuristiques de voisinage.
                if let Some(c) = correction(&sentence, i) {
                    suggestions.push(suggestion(token, c));
                    continue;
                }
                // 2. Étiquetage contrefactuel (comparaison de scores des graphies).
                let b = *base.get_or_insert_with(|| pos::best_score(tokens));
                if let Some(c) = counterfactual(tokens, idx, b) {
                    suggestions.push(suggestion(token, c));
                }
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

    /// Première suggestion via le chemin POS (`check_tagged`).
    fn first_tagged(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        HomophoneRule
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count_tagged(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        HomophoneRule.check_tagged(&tokens, &tags).len()
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
    fn mes_to_mais_before_pronoun() {
        assert_eq!(first("mes je ne sais pas").as_deref(), Some("mais"));
    }

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "on a mangé",       // « on » sujet
            "ils ont mangé",    // « ont » correct
            "son chat dort",    // « son » possessif
            "mes amis sont là", // « mes » possessif + nom
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    // --- Étiquetage contrefactuel (CRF). ---

    #[test]
    fn counterfactual_sont_to_son() {
        // « sont » écrit pour le possessif « son » (déterminant devant nom).
        assert_eq!(first_tagged("il caresse sont chat").as_deref(), Some("son"));
        assert_eq!(first_tagged("je prends sont stylo").as_deref(), Some("son"));
    }

    #[test]
    fn counterfactual_son_to_sont() {
        // « son » écrit pour l'auxiliaire « sont », au-delà de ils/elles.
        assert_eq!(
            first_tagged("Mes parents son venus.").as_deref(),
            Some("sont")
        );
        assert_eq!(
            first_tagged("Les portes son ouvertes.").as_deref(),
            Some("sont")
        );
    }

    #[test]
    fn counterfactual_no_false_positive_on_correct_usage() {
        for ok in [
            "son chat dort",
            "il prend son livre",
            "ils sont partis",
            "les chats sont noirs",
            "les enfants sont contents",
        ] {
            assert_eq!(count_tagged(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn check_tagged_keeps_heuristic_corrections() {
        // Le chemin POS conserve aussi les corrections de voisinage.
        let tokens = tokenize("ils on mangé");
        let tags = crate::pos::tag(&tokens);
        let sugg = HomophoneRule.check_tagged(&tokens, &tags);
        assert_eq!(
            sugg.first()
                .and_then(|s| s.replacements.first())
                .map(String::as_str),
            Some("ont")
        );
    }
}
