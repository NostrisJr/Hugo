//! Règle : homophones grammaticaux (a/à, ce/se, mes/mais, son/sont, on/ont).
//!
//! La levée d'ambiguïté fiable réclame le contexte morphosyntaxique. Un premier
//! jeu de corrections **à très haute précision** s'appuie sur le seul voisinage
//! immédiat (jeton précédent / suivant) au sein d'une même phrase :
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
//! Depuis la phase 4, la règle exploite aussi les **étiquettes POS** du CRF
//! ([`crate::pos`]) via [`Rule::check_tagged`], ce qui débloque des confusions
//! jusque-là hors de portée des seules heuristiques de voisinage :
//!
//! - **se → ce** : « se » suivi d'un mot étiqueté nom/adjectif (« se petit chat
//!   dort » → « ce ») — un « se » réflexif est toujours préverbal, donc « se »
//!   devant un nom/adjectif est forcément « ce » ; le « se livre » verbal
//!   légitime reste étiqueté verbe et ne déclenche pas.
//!
//! Lorsque l'étiqueteur « rattrape » l'erreur en donnant à la phrase fautive une
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
use crate::pos::{self, Tagged, Upos};
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
    let next = sentence.get(i + 1).map(|(_, t)| normalize(t.text.as_str()));
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

/// Corrections d'homophones **fondées sur les étiquettes POS** du CRF, hors de
/// portée des seules heuristiques de voisinage. `tags` est aligné sur la tranche
/// complète des jetons ; on y accède via l'index d'origine `sentence[k].0`.
fn correction_pos(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<&'static str> {
    let cur = normalize(sentence[i].1.text.as_str());
    let next_pos = sentence.get(i + 1).map(|(idx, _)| tags[*idx].upos);

    match cur.as_str() {
        // se → ce : « se » devant un nom (propre) ou un adjectif. Un « se »
        // réflexif est toujours préverbal ; devant un nom/adjectif, c'est « ce ».
        // Le « se livre » verbal légitime reste étiqueté verbe et ne déclenche pas.
        "se" if matches!(next_pos, Some(Upos::Noun | Upos::Propn | Upos::Adj)) => Some("ce"),

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

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        // Score POS du texte inchangé, calculé au plus une fois (seulement si un
        // candidat à l'étiquetage contrefactuel se présente).
        let mut base: Option<f32> = None;
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let (idx, token) = sentence[i];
                // 1. Heuristiques de voisinage, puis 2. corrections POS directes.
                if let Some(c) =
                    correction(&sentence, i).or_else(|| correction_pos(&sentence, i, tags))
                {
                    suggestions.push(suggestion(token, c));
                    continue;
                }
                // 3. Étiquetage contrefactuel (comparaison de scores des graphies).
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
            "il a faim",          // « a » auxiliaire, prev pronom
            "il va à Paris",      // « à » déjà correct
            "il pense à lui",     // « à » après un verbe : correct
            "on a mangé",         // « on » sujet
            "ils ont mangé",      // « ont » correct
            "son chat dort",      // « son » possessif
            "ce livre est lourd", // « ce » démonstratif en tête
            "mes amis sont là",   // « mes » possessif + nom
            "il se lève",         // « se » déjà correct
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn no_cross_sentence_leak() {
        // « va » (phrase 1) ne doit pas déclencher « a » de la phrase 2.
        assert_eq!(count("il va. a b c"), 0);
    }

    // --- Corrections fondées sur les étiquettes POS (CRF). ---

    #[test]
    fn se_to_ce_before_noun_or_adjective() {
        // « se » devant un nom (propre) ou un adjectif : c'est « ce ».
        assert_eq!(first_tagged("il aime se chien").as_deref(), Some("ce"));
        assert_eq!(first_tagged("se petit chat dort").as_deref(), Some("ce"));
    }

    #[test]
    fn se_to_ce_preserves_case() {
        assert_eq!(first_tagged("Se petit chat dort").as_deref(), Some("Ce"));
    }

    #[test]
    fn se_before_verb_is_not_corrected() {
        // « se » réflexif préverbal : aucune correction (pas de faux positif).
        assert_eq!(count_tagged("il se livre à la lecture"), 0);
        assert_eq!(count_tagged("il se lève"), 0);
    }

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
        let tokens = tokenize("il va a Paris");
        let tags = crate::pos::tag(&tokens);
        let sugg = HomophoneRule.check_tagged(&tokens, &tags);
        assert_eq!(
            sugg.first()
                .and_then(|s| s.replacements.first())
                .map(String::as_str),
            Some("à")
        );
    }
}
