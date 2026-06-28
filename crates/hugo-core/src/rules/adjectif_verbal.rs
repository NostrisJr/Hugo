//! Phase 10 — **Adjectif verbal vs participe présent** orthographe.
//!
//! Certains adjectifs verbaux ont une orthographe différente du participe
//! présent du même verbe. La distinction se fait par la **fonction** :
//!
//! | Participe présent (VERB) | Adjectif verbal (ADJ) |
//! |---|---|
//! | `fatiguant` (= en fatiguant) | `fatigant` (= qui fatigue) |
//! | `négligeant` | `négligent` |
//! | `différant` (de *différer*) | `différent` |
//! | `provoquant` | `provocant` |
//! | `convainquant` | `convaincant` |
//! | `communiquant` | `communicant` |
//! | `précédant` | `précédent` |
//! | `excellant` | `excellent` |
//! | `équivalant` | `équivalent` |
//!
//! ## Direction : participe → adjectif
//!
//! Si le mot est dans la liste et est en **fonction adjectivale** (attribut
//! après copule, ou épithète d'un nom), on suggère la forme adjectivale.
//!
//! ## Direction : adjectif → participe
//!
//! Si la forme adjectivale est employée comme **participe** (après un sujet
//! suivi directement du mot, sans copule — « en » + forme → participe présent),
//! on suggère la forme participiale.

use super::Rule;
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::lexical_sentences;
use crate::tokenizer::Token;
use crate::Suggestion;

const RULE_ID: &str = "adjectif_verbal";

/// Paires (participe_présent, adjectif_verbal).
const PAIRS: &[(&str, &str)] = &[
    ("fatiguant", "fatigant"),
    ("fatiguants", "fatigants"),
    ("fatiguante", "fatigante"),
    ("fatiguantes", "fatigantes"),
    ("négligeant", "négligent"),
    ("négligeants", "négligents"),
    ("négligeante", "négligente"),
    ("négligeantes", "négligentes"),
    ("provoquant", "provocant"),
    ("provoquants", "provocants"),
    ("provoquante", "provocante"),
    ("provoquantes", "provocantes"),
    ("convainquant", "convaincant"),
    ("convainquants", "convaincants"),
    ("convainquante", "convaincante"),
    ("convainquantes", "convaincantes"),
    ("communiquant", "communicant"),
    ("communiquants", "communicants"),
    ("communiquante", "communicante"),
    ("communiquantes", "communicantes"),
    ("précédant", "précédent"),
    ("précédants", "précédents"),
    ("précédante", "précédente"),
    ("précédantes", "précédentes"),
    ("excellant", "excellent"),
    ("équivalant", "équivalent"),
    ("différant", "différent"),
    ("différants", "différents"),
    ("différante", "différente"),
    ("différantes", "différentes"),
    ("naviguant", "navigant"),
    ("naviguants", "navigants"),
];

/// Vrai si `form` est l'adjectif verbal d'une paire (clé = adjectif).
fn participe_for_adj(form: &str) -> Option<&'static str> {
    let lower = form.to_lowercase();
    PAIRS
        .iter()
        .find(|(_, adj)| *adj == lower.as_str())
        .map(|(part, _)| *part)
}

/// Vrai si `form` est le participe présent d'une paire (clé = participe).
fn adj_for_participe(form: &str) -> Option<&'static str> {
    let lower = form.to_lowercase();
    PAIRS
        .iter()
        .find(|(part, _)| *part == lower.as_str())
        .map(|(_, adj)| *adj)
}

/// Copules pouvant précéder un adjectif attribut.
const COPULAS: &[&str] = &[
    "suis", "es", "est", "sommes", "êtes", "sont",
    "étais", "était", "étions", "étiez", "étaient",
    "serai", "seras", "sera", "serons", "serez", "seront",
    "sembles", "semble", "semblent",
    "parais", "paraît", "paraissent",
    "restes", "reste", "restent",
    "deviens", "devient", "devenons", "devenez", "deviennent",
];

pub struct AdjectifVerbalRule;

/// Vrai si la forme adjectivale `adj_form` (position `i`) est suivi d'un nom
/// étiqueté NOUN/PROPN par le CRF et que ce nom s'accorde avec elle. Ce motif
/// « en ADJ NOM » (« en différents scénarios ») indique un SP, pas un gérondif.
///
/// Double garde : CRF pour exclure les homographes (« son » = Det ou Noun) +
/// accord morphologique pour confirmer la cohérence adjectivale.
fn adj_agrees_with_following_noun(
    sentence: &[(usize, &Token)],
    i: usize,
    adj_form: &str,
    tags: &[Tagged],
) -> bool {
    let Some(&(next_orig, next_tok)) = sentence.get(i + 1) else {
        return false;
    };
    // Le mot suivant doit être un vrai NOM selon le CRF (pas DET, ADV…).
    if !matches!(tags[next_orig].upos, Upos::Noun | Upos::Propn) {
        return false;
    }
    let noun_readings: Vec<_> = morpho::lookup(&next_tok.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if noun_readings.is_empty() {
        return false;
    }
    let adj_readings: Vec<_> = morpho::lookup(adj_form)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Adjective)
        .collect();
    if adj_readings.is_empty() {
        return false;
    }
    adj_readings.iter().any(|adj| {
        noun_readings.iter().any(|noun| {
            let g_ok = match (adj.gender, noun.gender) {
                (Some(ag), Some(ng)) => ag == ng || ag == Gender::Epicene || ng == Gender::Epicene,
                _ => true,
            };
            let n_ok = match (adj.number, noun.number) {
                (Some(an), Some(nn)) => an == nn || an == Number::Invariable || nn == Number::Invariable,
                _ => true,
            };
            g_ok && n_ok
        })
    })
}

impl Rule for AdjectifVerbalRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                let form = sentence[i].1.text.as_str();
                let lower = form.to_lowercase();

                // --- participe présent → adjectif verbal (fonction attribut) ---
                if let Some(adj_form) = adj_for_participe(&lower) {
                    let prev_is_copula = i > 0 && {
                        let prev = sentence[i - 1].1.text.to_lowercase();
                        COPULAS.contains(&prev.as_str())
                    };
                    if prev_is_copula {
                        let tok = sentence[i].1;
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: format!(
                                "Orthographe : en fonction adjectivale (attribut), «\u{a0}{form}\u{a0}» s'écrit «\u{a0}{adj_form}\u{a0}»."
                            ),
                            replacements: vec![recase(form, adj_form)],
                            rule_id: RULE_ID,
                        });
                    }
                }

                // --- participe présent → adjectif verbal (épithète d'un nom) ---
                // La règle POS : si le CRF étiquette le token ADJ, il doit utiliser
                // la forme adjectivale.
                if let Some(adj_form) = adj_for_participe(&lower) {
                    let pos = super::confusion::upos(&sentence, i, tags);
                    if matches!(pos, Upos::Adj) {
                        let tok = sentence[i].1;
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: format!(
                                "Orthographe : en fonction adjectivale, «\u{a0}{form}\u{a0}» s'écrit «\u{a0}{adj_form}\u{a0}»."
                            ),
                            replacements: vec![recase(form, adj_form)],
                            rule_id: RULE_ID,
                        });
                    }
                }

                // --- adjectif verbal → participe présent (après « en » gérondif) ---
                // « en » est soit le gérondif (suivi d'un participe présent) soit
                // la préposition (suivi d'un adjectif qui modifie un nom suivant).
                // Garde : si la forme adjectivale s'accorde en genre et nombre avec
                // le nom qui lui est adjacent (ex. « différents scénarios » → Masc
                // Pl = Masc Pl), c'est un SP, pas un gérondif.
                if let Some(part_form) = participe_for_adj(&lower) {
                    let prev_is_en = i > 0 && {
                        let prev = sentence[i - 1].1.text.to_lowercase();
                        prev == "en"
                    };
                    if prev_is_en && !adj_agrees_with_following_noun(&sentence, i, form, tags) {
                        let tok = sentence[i].1;
                        suggestions.push(Suggestion {
                            span: tok.span,
                            message: format!(
                                "Orthographe : après «\u{a0}en\u{a0}» (gérondif), «\u{a0}{form}\u{a0}» s'écrit «\u{a0}{part_form}\u{a0}» (participe présent)."
                            ),
                            replacements: vec![recase(form, part_form)],
                            rule_id: RULE_ID,
                        });
                    }
                }
            }
        }
        // Dédoublonner (un token peut déclencher plusieurs fois)
        suggestions.dedup_by_key(|s| s.span);
        suggestions
    }

    fn name(&self) -> &'static str {
        "Adjectif verbal vs participe présent"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

fn recase(original: &str, lower: &str) -> String {
    if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars = lower.chars();
        match chars.next() {
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            None => lower.to_string(),
        }
    } else {
        lower.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AdjectifVerbalRule
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AdjectifVerbalRule.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn participe_to_adj_after_copula() {
        // « il est fatiguant » (adjectif attribut) → « fatigant »
        assert_eq!(first("il est fatiguant"), Some("fatigant".into()));
        assert_eq!(first("elle est négligeant"), Some("négligent".into()));
    }

    #[test]
    fn adj_to_participe_after_en() {
        // « en fatigant le lecteur » → « fatiguant »
        assert_eq!(first("en fatigant le lecteur"), Some("fatiguant".into()));
        assert_eq!(first("en négligent son travail"), Some("négligeant".into()));
    }

    #[test]
    fn correct_attributive_form() {
        assert_eq!(count("il est fatigant"), 0);
        assert_eq!(count("une situation fatigante"), 0);
    }

    #[test]
    fn correct_participial_form() {
        assert_eq!(count("en fatiguant le lecteur"), 0);
    }
}
