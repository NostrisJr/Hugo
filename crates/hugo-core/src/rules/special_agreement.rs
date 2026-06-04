//! Règle : accords spéciaux — adjectifs de couleur invariables, *même*,
//! *quelque*.
//!
//! Trois accords particuliers, traités avec prudence (et confirmation POS du
//! voisinage nominal via [`Rule::check_tagged`]) :
//!
//! - **couleurs invariables** : un nom employé comme adjectif de couleur reste
//!   invariable — « des gants marrons » → « marron », « des yeux noisettes » →
//!   « noisette ». On ne touche qu'aux couleurs **issues de noms** (liste fermée
//!   [`INVARIABLE_COLORS`]) ; les couleurs devenues de vrais adjectifs (rose,
//!   mauve, pourpre, écarlate, fauve) s'accordent et sont exclues. La couleur ne
//!   se déclenche que **postposée à un nom** (le mot coloré), ce qui écarte
//!   l'emploi nominal (« des oranges », fruits) ;
//! - **même** : adjectif, il s'accorde en nombre (« les même livres » →
//!   « mêmes ») ; adverbe (« même les enfants », *even*), il est invariable et
//!   n'est pas touché (repérage : *même* suivi d'un déterminant) ;
//! - **quelque** : devant un nom **pluriel**, c'est « quelques » (« quelque
//!   livres » → « quelques ») ; les emplois invariables (« quelque chose »,
//!   « quelque cent personnes ») restent intacts (le nom suivant n'y est pas
//!   pluriel).

use super::{lexical_sentences, Rule};
use crate::morpho::{self, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Accords spéciaux (couleurs invariables, *même*, *quelque*).
pub struct SpecialAgreement;

const RULE_ID: &str = "special_agreement";

/// Couleurs **issues de noms**, invariables en emploi adjectival. Exclut les
/// couleurs devenues de vrais adjectifs (rose, mauve, pourpre, écarlate, fauve),
/// qui s'accordent.
const INVARIABLE_COLORS: &[&str] = &[
    "marron",
    "orange",
    "turquoise",
    "émeraude",
    "marine",
    "crème",
    "cerise",
    "chocolat",
    "noisette",
    "olive",
    "saumon",
    "corail",
    "ivoire",
    "kaki",
    "ocre",
    "grenat",
    "lavande",
    "moutarde",
    "caramel",
    "anthracite",
    "azur",
    "brique",
];

/// Minuscules + apostrophe finale ôtée.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
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

/// Si `cur` (minuscule) est la forme **pluralisée** d'une couleur invariable,
/// renvoie sa forme de base (invariable). « marrons » → « marron ».
fn pluralized_color(cur: &str) -> Option<&'static str> {
    let base = cur.strip_suffix('s')?;
    INVARIABLE_COLORS.iter().copied().find(|&c| c == base)
}

fn is_plural_determiner(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs"
    )
}

fn is_singular_determiner(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "le" | "la"
            | "l"
            | "un"
            | "une"
            | "ce"
            | "cet"
            | "cette"
            | "mon"
            | "ton"
            | "son"
            | "ma"
            | "ta"
            | "sa"
            | "notre"
            | "votre"
            | "leur"
            | "du"
    )
}

/// Vrai si toutes les analyses nominales du jeton sont au pluriel (et il en
/// existe au moins une).
fn noun_is_plural(text: &str) -> bool {
    let nouns: Vec<_> = morpho::lookup(text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    !nouns.is_empty() && nouns.iter().all(|m| m.number == Some(Number::Plural))
}

/// Cherche la correction d'accord spécial pour le jeton lexical `i`.
fn correction(lex: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<String> {
    let cur = normalize(lex[i].1.text.as_str());
    let tag_of = |k: usize| tags[lex[k].0].upos;
    let prev_is_noun = i > 0 && matches!(tag_of(i - 1), Upos::Noun | Upos::Propn);

    // --- Couleur invariable postposée à un nom. ---
    if let Some(base) = pluralized_color(&cur) {
        if prev_is_noun {
            return Some(base.to_string());
        }
    }

    match cur.as_str() {
        // même → mêmes : adjectif après un pluriel ; pas l'adverbe « même les ».
        "même" => {
            let next_is_det = lex.get(i + 1).is_some_and(|(_, t)| {
                is_plural_determiner(&t.text) || is_singular_determiner(&t.text)
            });
            if next_is_det {
                return None; // « même les enfants » : adverbe « even ».
            }
            let prev_plural = i > 0
                && (is_plural_determiner(&lex[i - 1].1.text)
                    || (matches!(tag_of(i - 1), Upos::Noun) && noun_is_plural(&lex[i - 1].1.text)));
            prev_plural.then(|| "mêmes".to_string())
        }
        // mêmes → même : après un déterminant singulier (« le mêmes »).
        "mêmes" => {
            (i > 0 && is_singular_determiner(&lex[i - 1].1.text)).then(|| "même".to_string())
        }
        // quelque → quelques : devant un nom pluriel.
        "quelque" => {
            let next_plural_noun = lex.get(i + 1).is_some_and(|&(idx, t)| {
                matches!(tags[idx].upos, Upos::Noun) && noun_is_plural(&t.text)
            });
            next_plural_noun.then(|| "quelques".to_string())
        }
        _ => None,
    }
}

impl Rule for SpecialAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for i in 0..lex.len() {
                let Some(corrected) = correction(&lex, i, tags) else {
                    continue;
                };
                let token = lex[i].1;
                suggestions.push(Suggestion {
                    span: token.span,
                    message: format!(
                        "Accord spécial : « {} » devrait être « {} ».",
                        token.text, corrected
                    ),
                    replacements: vec![match_case(&token.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accords spéciaux (couleurs, même, quelque)"
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
        SpecialAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        SpecialAgreement.check(&tokenize(text)).len()
    }

    #[test]
    fn invariable_color_postnominal() {
        assert_eq!(first("des gants marrons").as_deref(), Some("marron"));
        assert_eq!(first("les yeux noisettes").as_deref(), Some("noisette"));
    }

    #[test]
    fn color_as_noun_is_not_flagged() {
        // « des oranges » (fruits) / « des marrons » (châtaignes) : noms têtes.
        assert_eq!(count("des oranges"), 0);
        assert_eq!(count("des marrons"), 0);
    }

    #[test]
    fn correct_invariable_color_is_silent() {
        assert_eq!(count("des gants marron"), 0);
        assert_eq!(count("des yeux noisette"), 0);
    }

    #[test]
    fn meme_agrees_in_number() {
        assert_eq!(first("les même livres").as_deref(), Some("mêmes"));
        assert_eq!(first("les livres même").as_deref(), Some("mêmes"));
    }

    #[test]
    fn adverbial_meme_is_not_flagged() {
        // « même les enfants » (even) : adverbe invariable.
        assert_eq!(count("même les enfants sont venus"), 0);
        assert_eq!(count("le même livre"), 0);
    }

    #[test]
    fn quelque_before_plural_noun() {
        assert_eq!(first("quelque livres").as_deref(), Some("quelques"));
    }

    #[test]
    fn invariable_quelque_is_silent() {
        for ok in ["quelque chose", "quelque part", "quelques livres"] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }
}
