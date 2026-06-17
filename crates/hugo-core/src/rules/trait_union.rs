//! Phase 10 — **Traits d'union obligatoires** : impératif + pronom postposé,
//! *est-ce que*, inversions verbales.
//!
//! ## Impératif + pronom
//!
//! À l'impératif, le pronom postposé est lié par un trait d'union :
//! - *«donne moi»* → *«donne-moi»*
//! - *«vas y»* → *«vas-y»*
//! - *«parle lui»* → *«parle-lui»*
//!
//! Signal : verbe conjugué à l'impératif (2ᵉ personne sans sujet) suivi d'un
//! pronom postposé (`moi`, `toi`, `lui`, `nous`, `vous`, `leur`, `les`, `le`,
//! `la`, `y`, `en`).
//!
//! Heuristique : on identifie un verbe impératif comme un verbe en tête de
//! phrase (ou après une virgule), sans pronom sujet devant lui, suivi d'un
//! pronom postposé.
//!
//! ## est-ce que / est-ce qui
//!
//! *«est ce que»* → *«est-ce que»*, *«est ce qui»* → *«est-ce qui»*.
//!
//! Signal : `est` + `ce` + `que/qui` (trois tokens séparés).
//!
//! ## dit-il / dit-elle (inversion verbale)
//!
//! *«dit il»* → *«dit-il»*. Signal : verbe conjugué + pronom sujet inversé
//! (`il/elle/on/ils/elles/je/tu`).
//!
//! Quand les tags CRF sont disponibles ([`Rule::check_tagged`]), la détection
//! d'inversion n'active le verbe candidat que s'il est **étiqueté `VERB`/`AUX`**,
//! ce qui évite les faux positifs sur les homographes (« poche » étiqueté `NOUN`
//! ne déclenche plus l'inversion devant « qu'il »).

use super::Rule;
use crate::morpho::{self, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::{Token, TokenKind};
use crate::Span;
use crate::Suggestion;

const RULE_ID: &str = "trait_union";

/// Pronoms postposés à l'impératif.
const POSTVERBAL_PRONOUNS: &[&str] = &[
    "moi", "toi", "lui", "nous", "vous", "leur", "les", "le", "la", "y", "en",
    "m", "t", "l", // formes élidées
];

/// Pronoms sujets inversés.
const INVERTED_PRONOUNS: &[&str] = &["il", "elle", "on", "ils", "elles", "je", "tu"];

/// Vrai si le texte est un verbe impératif plausible (forme conjuguée 2ᵉ pers.
/// sans sujet — on approche : verbe dont la morphologie admet une forme
/// impérative). Repli morphologique utilisé par [`Rule::check`].
fn looks_like_imperative(form: &str) -> bool {
    let morphs = morpho::lookup(form);
    morphs.iter().any(|m| {
        m.category == MorphCategory::Verb
            && matches!(m.person, Some(Person::Second))
    })
}

/// Vrai si le verbe admet une forme finie. Repli morphologique utilisé par
/// [`Rule::check`] ; dans [`Rule::check_tagged`] on lui préfère le tag CRF.
fn looks_like_conjugated(form: &str) -> bool {
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == MorphCategory::Verb)
}

/// Personne et nombre attendus pour un pronom inversé.
fn inverted_pronoun_pn(pronoun: &str) -> Option<(Person, Number)> {
    Some(match pronoun.to_lowercase().as_str() {
        "je" => (Person::First, Number::Singular),
        "tu" => (Person::Second, Number::Singular),
        "il" | "elle" | "on" => (Person::Third, Number::Singular),
        "ils" | "elles" => (Person::Third, Number::Plural),
        _ => return None,
    })
}

/// Vrai si le verbe conjugué possède une forme finie compatible avec la
/// personne et le nombre du pronom inversé.
///
/// Guard anti-faux positif : « fut tu » (fut=3sg, tu=2sg) ne constitue pas
/// une inversion — « tu » est ici le participe passé de *taire*.
fn verb_agrees_with_inverted_pronoun(verb: &str, pronoun: &str) -> bool {
    let Some((person, number)) = inverted_pronoun_pn(pronoun) else {
        return true;
    };
    morpho::verb_forms(verb)
        .iter()
        .any(|v| v.person == person && v.number == number)
}

/// Cherche le prochain token `Word` en sautant uniquement l'espace blanc.
/// Stoppe dès qu'un token de ponctuation ou d'élision est rencontré — cela
/// évite de voir une inversion à travers un guillemet ou une élision
/// (`dit : «Je`, `… que il` avec `qu'` élidé).
fn next_word_no_punct(tokens: &[Token], i: usize) -> Option<usize> {
    let mut k = i + 1;
    while k < tokens.len() {
        match tokens[k].kind {
            TokenKind::Whitespace => k += 1,
            TokenKind::Word => return Some(k),
            _ => return None,
        }
    }
    None
}

/// Cherche le prochain token `Word` en sautant ponctuation et espaces.
/// Utilisé pour `est-ce que` et l'impératif.
fn next_word_idx(tokens: &[Token], i: usize) -> Option<usize> {
    (i + 1..tokens.len()).find(|&k| tokens[k].kind == TokenKind::Word)
}

/// Vrai si le verbe se termine par une voyelle graphique — dans ce cas, une
/// inversion devant `il`/`elle`/`on` requiert un *t* euphonique (« sera-t-il »,
/// « va-t-on », « parle-t-elle »).
fn needs_euphonic_t(verb: &str, pronoun: &str) -> bool {
    let ends_vowel = verb
        .chars()
        .last()
        .map_or(false, |c| matches!(c, 'a' | 'e' | 'é' | 'è' | 'ê' | 'ë' | 'i' | 'î' | 'ï' | 'o' | 'ô' | 'u' | 'û' | 'ù' | 'ü'));
    ends_vowel && matches!(pronoun.to_lowercase().as_str(), "il" | "elle" | "on")
}

/// Logique principale. `tags` vaut `None` pour le repli morphologique
/// (chemin [`Rule::check`]) et `Some` pour le chemin [`Rule::check_tagged`]
/// où le verbe candidat doit être étiqueté `VERB`/`AUX` par le CRF.
fn run(tokens: &[Token], tags: Option<&[Tagged]>) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    let n = tokens.len();

    for i in 0..n {
        let tok = &tokens[i];
        if tok.kind != TokenKind::Word {
            continue;
        }
        let lower = tok.text.to_lowercase();

        // --- est ce que/qui → est-ce que/qui ---
        if lower == "est" {
            if let Some(j) = next_word_idx(tokens, i) {
                if tokens[j].text.to_lowercase() == "ce" {
                    if let Some(k) = next_word_idx(tokens, j) {
                        let next2 = tokens[k].text.to_lowercase();
                        if next2 == "que" || next2 == "qui" {
                            let has_space_1 = tokens[i + 1..j]
                                .iter()
                                .any(|t| t.kind == TokenKind::Whitespace);
                            let has_space_2 = tokens[j + 1..k]
                                .iter()
                                .any(|t| t.kind == TokenKind::Whitespace);
                            if has_space_1 || has_space_2 {
                                let span = Span::new(tok.span.start, tokens[j].span.end);
                                suggestions.push(Suggestion {
                                    span,
                                    message: "Trait d'union manquant : «\u{a0}est ce\u{a0}» s'écrit «\u{a0}est-ce\u{a0}».".to_string(),
                                    replacements: vec!["est-ce".to_string()],
                                    rule_id: RULE_ID,
                                });
                            }
                        }
                    }
                }
            }
        }

        // --- verbe + pronom inversé → trait d'union (dit il → dit-il) ---
        // Avec tags : seul un token étiqueté VERB/AUX déclenche la vérification.
        // Sans tags (repli) : on utilise la lecture morphologique.
        // Dans les deux cas, `next_word_no_punct` empêche de traverser une
        // élision ou une ponctuation (frontière de proposition).
        let is_verb = match tags {
            Some(tags) => matches!(tags[i].upos, Upos::Verb | Upos::Aux),
            None => looks_like_conjugated(&tok.text),
        };
        if is_verb {
            if let Some(j) = next_word_no_punct(tokens, i) {
                let next_lower = tokens[j].text.to_lowercase();
                if INVERTED_PRONOUNS.contains(&next_lower.as_str())
                    && verb_agrees_with_inverted_pronoun(&tok.text, &next_lower)
                {
                    let has_space = tokens[i + 1..j]
                        .iter()
                        .any(|t| t.kind == TokenKind::Whitespace);
                    if has_space {
                        let correction = if needs_euphonic_t(&tok.text, &tokens[j].text) {
                            format!("{}-t-{}", tok.text, tokens[j].text)
                        } else {
                            format!("{}-{}", tok.text, tokens[j].text)
                        };
                        let span = Span::new(tok.span.start, tokens[j].span.end);
                        suggestions.push(Suggestion {
                            span,
                            message: format!(
                                "Trait d'union manquant : dans une inversion, «\u{a0}{} {}\u{a0}» s'écrit «\u{a0}{correction}\u{a0}».",
                                tok.text, tokens[j].text
                            ),
                            replacements: vec![correction],
                            rule_id: RULE_ID,
                        });
                    }
                }
            }
        }

        // --- inversion déjà liée mais manquant le t euphonique ---
        // « Sera-il » → « Sera-t-il » : token déjà composé, sans le « -t- ».
        // On reste sur la morphologie pour la partie verbale (un token unique
        // composite n'a pas de tag POS utile pour le préfixe verbal seul).
        if tok.text.contains('-') && !tok.text.contains("-t-") {
            let mut parts = tok.text.splitn(2, '-');
            if let (Some(verb_part), Some(pronoun_part)) = (parts.next(), parts.next()) {
                let pronoun_lower = pronoun_part.to_lowercase();
                if looks_like_conjugated(verb_part)
                    && INVERTED_PRONOUNS.contains(&pronoun_lower.as_str())
                    && needs_euphonic_t(verb_part, pronoun_part)
                {
                    let correction = format!("{verb_part}-t-{pronoun_part}");
                    suggestions.push(Suggestion {
                        span: tok.span,
                        message: format!(
                            "T euphonique manquant : «\u{a0}{}\u{a0}» s'écrit «\u{a0}{correction}\u{a0}» (le verbe se termine par une voyelle).",
                            tok.text
                        ),
                        replacements: vec![correction],
                        rule_id: RULE_ID,
                    });
                }
            }
        }

        // --- impératif + pronom postposé ---
        // On utilise toujours la vérification morphologique (`looks_like_imperative`)
        // même quand les tags sont disponibles : le CRF étiquette VERB aussi bien
        // l'indicatif que l'impératif (il ne distingue pas la personne). La
        // morphologie détecte la 2ᵉ personne plus précisément, ce qui évite de
        // confondre « patientaient » (3ᵉ pl. imparfait) avec un impératif.
        //
        // Garde : si le token précédent est un pronom **sujet** (explicite), le
        // verbe est à l'indicatif, pas à l'impératif (l'impératif n'a jamais de
        // sujet exprimé). Ex. « Elle prends le train » → pas « prends-le ».
        //
        // Garde supplémentaire : si le clitique postverbal est immédiatement
        // suivi d'un infinitif, il dépend de l'infinitif (« va la voir » →
        // « la » est objet de « voir », pas de « va ») → pas de trait d'union.
        let preceded_by_subject = (1..=i).find_map(|k| {
            let prev = &tokens[i - k];
            if prev.kind == TokenKind::Whitespace {
                return None; // continuer à chercher
            }
            if prev.kind == TokenKind::Word {
                const SUBJECT_PRONOUNS: &[&str] = &[
                    "je", "j", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles",
                ];
                if SUBJECT_PRONOUNS.contains(&prev.text.to_lowercase().as_str()) {
                    return Some(true);
                }
            }
            Some(false) // autre token (ponctuation, mot non-sujet) → stopper
        }).unwrap_or(false);
        if looks_like_imperative(&tok.text) && !preceded_by_subject {
            if let Some(j) = next_word_idx(tokens, i) {
                let next_lower = tokens[j].text.to_lowercase();
                if POSTVERBAL_PRONOUNS.contains(&next_lower.as_str()) {
                    let has_space = tokens[i + 1..j]
                        .iter()
                        .any(|t| t.kind == TokenKind::Whitespace);
                    // Le clitique est-il suivi d'un infinitif ?
                    let clitic_governs_inf = next_word_idx(tokens, j)
                        .map_or(false, |k| {
                            crate::morpho::lookup(&tokens[k].text)
                                .iter()
                                .any(|m| {
                                    m.category == crate::morpho::MorphCategory::Verb
                                        && m.lemma == tokens[k].text.to_lowercase()
                                })
                        });
                    if has_space && !clitic_governs_inf {
                        let correction = format!("{}-{}", tok.text, tokens[j].text);
                        let span = Span::new(tok.span.start, tokens[j].span.end);
                        suggestions.push(Suggestion {
                            span,
                            message: format!(
                                "Trait d'union manquant : «\u{a0}{} {}\u{a0}» → «\u{a0}{correction}\u{a0}» (impératif + pronom).",
                                tok.text, tokens[j].text
                            ),
                            replacements: vec![correction],
                            rule_id: RULE_ID,
                        });
                    }
                }
            }
        }
    }

    // Dédoublonner (un verbe peut déclencher à la fois impératif + inversion)
    suggestions.dedup_by_key(|s| s.span);
    suggestions
}

pub struct TraitUnionRule;

impl Rule for TraitUnionRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        run(tokens, None)
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        run(tokens, Some(tags))
    }

    fn name(&self) -> &'static str {
        "Traits d'union obligatoires (impératif, est-ce, inversion)"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    /// Chemin de production : tags CRF fournis.
    fn first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        TraitUnionRule
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        TraitUnionRule.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn est_ce_que() {
        assert_eq!(first("est ce que tu viens"), Some("est-ce".into()));
        assert_eq!(first("est ce qui se passe"), Some("est-ce".into()));
    }

    #[test]
    fn est_ce_already_correct() {
        assert_eq!(count("est-ce que tu viens"), 0);
    }

    #[test]
    fn dit_il_inversion() {
        assert_eq!(first("dit il"), Some("dit-il".into()));
        assert_eq!(first("mange t il"), Some("mange-t".into()));
    }

    #[test]
    fn dit_il_already_correct() {
        assert_eq!(count("dit-il"), 0);
    }

    #[test]
    fn euphonic_t_on_vowel_ending_verb() {
        // Verbe en voyelle + il/elle/on séparés par un espace → t euphonique.
        assert_eq!(first("sera il"), Some("sera-t-il".into()));
        assert_eq!(first("va il"), Some("va-t-il".into()));
        assert_eq!(first("parle elle"), Some("parle-t-elle".into()));
        assert_eq!(first("ira on"), Some("ira-t-on".into()));
    }

    #[test]
    fn no_euphonic_t_on_consonant_ending_verb() {
        assert_eq!(first("dit il"), Some("dit-il".into()));
        assert_eq!(first("vient il"), Some("vient-il".into()));
    }

    #[test]
    fn euphonic_t_on_already_hyphenated() {
        // Token déjà lié (tokenizer), t euphonique manquant.
        assert_eq!(first("Sera-il des nôtres"), Some("Sera-t-il".into()));
        assert_eq!(first("va-il"), Some("va-t-il".into()));
    }

    #[test]
    fn no_false_euphonic_t_on_correct_forms() {
        assert_eq!(count("dit-il"), 0);
        assert_eq!(count("vient-il"), 0);
        assert_eq!(count("mange-t-il"), 0);
        assert_eq!(count("sera-t-il"), 0);
    }

    #[test]
    fn no_false_imperative_with_infinitive_clitic() {
        // « va la voir » : « la » dépend de « voir » (infinitif) → pas de trait d'union.
        assert_eq!(count("Elle va la voir et fait ce qu'elle peut"), 0);
        assert_eq!(count("va le chercher"), 0);
    }

    #[test]
    fn no_false_imperative_on_non_second_person_verb() {
        // « patientaient » : imparfait 3ᵉ pl., pas de lecture 2ᵉ pers. → pas d'impératif.
        assert_eq!(count("Là bas patientaient les enfants"), 0);
    }

    #[test]
    fn no_false_inversion_on_noun_homograph() {
        // « poche » est étiqueté NOUN par le CRF → pas de fausse inversion.
        assert_eq!(count("Il a de l'argent de poche qu'il ne peut pas utiliser"), 0);
    }

    #[test]
    fn no_false_inversion_through_punctuation() {
        // Ponctuation entre verbe et pronom → frontière de proposition.
        assert_eq!(count("Il lui dit : Je ne pense pas"), 0);
    }

    #[test]
    fn donne_moi_imperatif() {
        assert_eq!(first("donne moi"), Some("donne-moi".into()));
        assert_eq!(first("vas y"), Some("vas-y".into()));
    }

    #[test]
    fn donne_moi_correct() {
        assert_eq!(count("donne-moi"), 0);
    }
}
