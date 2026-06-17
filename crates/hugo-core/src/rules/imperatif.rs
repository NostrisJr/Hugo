//! Règle : impératif présent — suppression du « s » parasite.
//!
//! Deux sous-règles :
//!
//! **1ᵉʳ groupe (-er).** Les verbes du 1ᵉʳ groupe perdent le « s » de la
//! 2ᵉ personne du singulier à l'impératif (« Mange ! », non « Manges ! »),
//! **sauf** devant « en » ou « y » pour l'euphonie.
//!
//! La règle détecte un verbe en tête de phrase se terminant par « -es » en
//! cumulant plusieurs gardes pour limiter les faux positifs :
//!
//! - le mot suivant n'est **pas** « tu », « en », ou « y » ;
//! - le mot suivant n'est pas un verbe conjugué (sinon le mot courant est sujet) ;
//! - une virgule entre le mot et la suite signale un nom propre ou une liste ;
//! - le mot n'est pas lui-même reconnu comme nom dans le Lefff.
//!
//! **« aller » devant un infinitif.** La forme indicative « vas » s'utilise
//! couramment à la place de l'impératif « va » en tête de phrase : « Vas faire
//! les courses » → « Va faire les courses ». La règle détecte la forme « vas »
//! en tête de phrase immédiatement suivie d'un infinitif (-er/-ir/-re/-oir) et
//! suggère la suppression du « s ».
//!
//! Correction proposée : supprimer le « s » final.

use super::Rule;
use crate::morpho::{self, MoodTense, MorphCategory, Number, Person};
use crate::pos::Tagged;
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Vrai si la forme est un infinitif (pas de personne dans le lexique ou
/// terminaison d'infinitif reconnue), utilisé pour détecter « vas + infinitif ».
fn looks_like_infinitive(text: &str) -> bool {
    let low = text.to_lowercase();
    // Terminaisons d'infinitif courantes.
    if low.ends_with("er") || low.ends_with("ir") || low.ends_with("re") || low.ends_with("oir") {
        // Vérifier que ce n'est pas une forme conjuguée connue.
        return morpho::verb_forms(&low).is_empty();
    }
    false
}

/// Vérifie si la forme « vas » en tête de phrase est suivie d'un infinitif et,
/// si oui, renvoie une suggestion de correction (→ « va »).
fn check_vas_infinitif(tokens: &[Token], first_lex: usize) -> Option<Suggestion> {
    let token = &tokens[first_lex];
    if token.text.to_lowercase() != "vas" {
        return None;
    }

    // Trouver le prochain token lexical dans la même phrase.
    let mut j = first_lex + 1;
    while j < tokens.len() && !tokens[j].is_lexical() {
        if tokens[j].kind == TokenKind::Punctuation
            && matches!(tokens[j].text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
        {
            return None;
        }
        j += 1;
    }
    let next = tokens.get(j)?;
    if !next.is_lexical() {
        return None;
    }
    // Gardes : « y » (vas-y) et « en » conservent le « s ».
    if matches!(next.text.to_lowercase().as_str(), "y" | "en") {
        return None;
    }
    if !looks_like_infinitive(&next.text) {
        return None;
    }

    let corrected = token.text[..token.text.len() - 1].to_string();
    Some(Suggestion {
        span: token.span,
        message: format!(
            "Impératif : « {} » → devant un infinitif, l'impératif de « aller » est « va » (sans « s »).",
            token.text
        ),
        replacements: vec![corrected],
        rule_id: "imperatif_groupe1",
    })
}

pub struct ImperatifGroupe1;

const RULE_ID: &str = "imperatif_groupe1";

/// Mots qui, immédiatement après le verbe potentiel, indiquent qu'on est à
/// l'indicatif (« tu ») ou qu'un « s » d'euphonie est requis (« en », « y »).
fn keeps_s_or_is_subject(w: &str) -> bool {
    matches!(w.to_lowercase().as_str(), "tu" | "en" | "y")
}

/// Vrai si la forme (en minuscules) est un verbe conjugué fini.
fn is_finite_verb_lc(text: &str) -> bool {
    !morpho::verb_forms(&text.to_lowercase()).is_empty()
}

/// Vrai si le token est une ponctuation de fin de phrase.
fn is_sentence_terminator(t: &Token) -> bool {
    t.kind == TokenKind::Punctuation
        && matches!(t.text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
}

/// Analyse le token d'index `first_lex` (premier mot de la phrase courante) :
/// renvoie une suggestion si ce mot ressemble à un impératif 1er groupe erroné.
fn check_imperatif(tokens: &[Token], first_lex: usize) -> Option<Suggestion> {
    let token = &tokens[first_lex];
    let low = token.text.to_lowercase();

    // Doit se terminer par « -es » avec au moins 4 caractères
    // (p. ex. « aes » ou « bes » ne peuvent pas être des verbes -er courants).
    if !low.ends_with("es") || low.chars().count() < 4 {
        return None;
    }

    // Parcours des tokens non-lexicaux jusqu'au prochain mot, en restant dans
    // la même phrase (arrêt sur ponctuation forte) et en détectant la virgule.
    let mut j = first_lex + 1;
    let mut saw_comma = false;
    while j < tokens.len() && !tokens[j].is_lexical() {
        if tokens[j].kind == TokenKind::Punctuation {
            match tokens[j].text.as_str() {
                "," => saw_comma = true,
                // Fin de phrase : le mot suivant sera dans une autre phrase.
                "." | "!" | "?" | "…" | ";" | ":" => {
                    j = tokens.len(); // signal « pas de mot suivant dans cette phrase »
                    break;
                }
                _ => {}
            }
        }
        j += 1;
    }

    // Une virgule entre le verbe et la suite signale un nom propre ou une liste
    // (ex. « Castres, belle ville du Tarn ») → on s'abstient.
    if saw_comma {
        return None;
    }

    // Prochain token lexical dans la même phrase (None si fin de phrase).
    let next_lex: Option<&Token> = if j < tokens.len() && tokens[j].is_lexical() {
        Some(&tokens[j])
    } else {
        None
    };

    // Garde : « tu », « en », « y » → indicatif normal ou euphonie.
    if let Some(next) = next_lex {
        if keeps_s_or_is_subject(&next.text) {
            return None;
        }
        // Garde : verbe conjugué suivant → le mot courant est un sujet, pas un verbe.
        if is_finite_verb_lc(&next.text) {
            return None;
        }
    }

    // Garde : le mot courant est reconnu comme nom dans le Lefff
    // (filtre « tables », « roses », « phases »… qui ont une lecture nominale).
    let analyses = morpho::lookup(&low);
    if analyses.iter().any(|m| m.category == MorphCategory::Noun) {
        return None;
    }

    // Chemin 1 : le Lefff connaît la forme comme 2sg présent d'un verbe -er.
    let is_v1_2sg = morpho::verb_forms(&low).iter().any(|v| {
        v.mood_tense == MoodTense::IndicativePresent
            && v.person == Person::Second
            && v.number == Number::Singular
            && v.lemma.ends_with("er")
    });

    if !is_v1_2sg {
        // Chemin 2 : repli suffixe pour les formes absentes du Lefff.
        // « implémentes » → stem « implément » → infinitif « implémenter » → VERB connu.
        let stem = low.strip_suffix("es")?;
        let infinitive = format!("{}er", stem);
        let is_known_verb = morpho::lookup(&infinitive)
            .iter()
            .any(|m| m.category == MorphCategory::Verb);
        if !is_known_verb {
            return None;
        }
    }

    // Correction : supprimer le « s » final (ASCII, 1 octet).
    let corrected = token.text[..token.text.len() - 1].to_string();
    Some(Suggestion {
        span: token.span,
        message: format!(
            "Impératif : « {} » → les verbes du 1ᵉʳ groupe (-er) perdent le « s » à l'impératif 2ᵉ personne du singulier.",
            token.text
        ),
        replacements: vec![corrected],
        rule_id: RULE_ID,
    })
}

impl Rule for ImperatifGroupe1 {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let mut at_sentence_start = true;

        for i in 0..tokens.len() {
            let t = &tokens[i];
            if is_sentence_terminator(t) {
                at_sentence_start = true;
            } else if t.is_lexical() {
                if at_sentence_start {
                    if let Some(s) = check_imperatif(tokens, i) {
                        suggestions.push(s);
                    } else if let Some(s) = check_vas_infinitif(tokens, i) {
                        suggestions.push(s);
                    }
                }
                at_sentence_start = false;
            }
            // Espaces / ponctuation non-terminale : on maintient `at_sentence_start`.
        }
        suggestions
    }

    // La règle n'utilise pas les étiquettes POS (le CRF étiquette les verbes
    // en tête de phrase avec capitale comme PROPN, ce qui serait contre-productif
    // ici). Le chemin `check` suffit.

    fn name(&self) -> &'static str {
        "Impératif groupe 1 (–er)"
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
        ImperatifGroupe1
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        ImperatifGroupe1.check(&tokenize(text)).len()
    }

    // --- Cas positifs. ---

    #[test]
    fn imperatif_simple_connu_lefff() {
        // « manges » est en Lefff comme 2sg de manger.
        assert_eq!(first("Manges ta soupe !").as_deref(), Some("Mange"));
    }

    #[test]
    fn imperatif_er_absent_lefff() {
        // « implémentes » absent du Lefff mais infinitif « implémenter » connu.
        assert_eq!(
            first("Implémentes la phase 12.").as_deref(),
            Some("Implémente")
        );
    }

    #[test]
    fn imperatif_sans_point_exclamation() {
        assert_eq!(first("Apportes le document.").as_deref(), Some("Apporte"));
    }

    #[test]
    fn imperatif_milieu_texte_apres_point() {
        assert_eq!(
            first("Il dort. Implémentes la phase 12.").as_deref(),
            Some("Implémente")
        );
    }

    // --- Gardes contre les faux positifs. ---

    #[test]
    fn indicatif_tu_est_correct() {
        // « Tu manges » : indicatif avec sujet explicite → rien.
        assert_eq!(count("Tu manges ta soupe."), 0);
    }

    #[test]
    fn inversion_manges_tu_pas_de_suggestion() {
        // « Manges-tu ? » : inversion conservée comme jeton unique → rien.
        assert_eq!(count("Manges-tu ta soupe ?"), 0);
    }

    #[test]
    fn euphonie_en_pas_de_suggestion() {
        // « Manges-en » : « en » conserve le « s » → rien.
        assert_eq!(count("Manges-en encore !"), 0);
    }

    #[test]
    fn noms_communs_du_lefff_pas_de_faux_positif() {
        for ok in [
            "Tables et chaises disponibles.",
            "Roses de notre jardin.",
            "Phases critiques du projet.",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn sujet_verbe_suivant_pas_de_faux_positif() {
        // Le mot suivant est un verbe conjugué → le premier mot est un sujet.
        for ok in [
            "Castres est une belle ville.",
            "Tables sont dressées.",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn virgule_apres_premier_mot_pas_de_faux_positif() {
        // Une virgule après le mot → probablement un nom propre ou une liste.
        assert_eq!(count("Castres, belle cité du Tarn, accueille des touristes."), 0);
    }

    #[test]
    fn imperatif_correct_est_silencieux() {
        for ok in [
            "Mange ta soupe !",
            "Apporte le document.",
            "Implémente la phase 12.",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }
}
