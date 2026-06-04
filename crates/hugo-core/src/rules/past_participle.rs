//! Règle : accord du participe passé employé avec l'auxiliaire « avoir » et un
//! **complément d'objet direct antéposé**.
//!
//! Avec « avoir », le participe passé s'accorde en genre et en nombre avec le
//! COD **lorsque celui-ci précède** le verbe : « je les ai vu » → « vus », « il
//! la avait prise » est correct, « il les a mis » → « mis » déjà accordé.
//!
//! Le COD antéposé est ici un **pronom clitique objet non ambigu** :
//!
//! - **les** → pluriel (genre indéterminé) : on propose les deux graphies
//!   (« je les ai vu » → « vus » / « vues ») quand le participe est au singulier ;
//! - **la** → féminin singulier (« il la a vu » → « vue »).
//!
//! Sont volontairement écartés, faute de signal fiable :
//!
//! - `me`/`te`/`nous`/`vous`, qui peuvent être **sujets** (« nous avons vu » —
//!   pas d'accord — contre « il nous a vus ») ;
//! - `l'`, dont le genre est indéterminé et le nombre déjà singulier.
//!
//! La construction est repérée par le motif lexical « clitique, auxiliaire
//! avoir, participe », confirmé par l'étiquette POS `PRON` du clitique
//! ([`Rule::check_tagged`]).

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, Morph, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord du participe passé avec « avoir » et un COD antéposé.
pub struct PastParticipleAvoir;

const RULE_ID: &str = "past_participle_avoir";

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

/// Genre (éventuel) et nombre imposés par un pronom clitique objet **non
/// ambigu**. `None` pour les pronoms ambigus (sujet/objet) ou indéterminés.
fn cod_features(text: &str) -> Option<(Option<Gender>, Number)> {
    match normalize(text).as_str() {
        "les" => Some((None, Number::Plural)),
        "la" => Some((Some(Gender::Feminine), Number::Singular)),
        _ => None,
    }
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « avoir ».
fn is_avoir(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "avoir")
}

/// Vrai si le jeton est un adverbe pouvant s'intercaler entre l'auxiliaire et le
/// participe (« je les ai déjà vus », « je ne les ai pas vus »).
fn is_skippable_adverb(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "pas"
            | "jamais"
            | "plus"
            | "déjà"
            | "bien"
            | "toujours"
            | "encore"
            | "vraiment"
            | "souvent"
            | "tous"
            | "tout"
            | "même"
            | "trop"
    )
}

/// Analyses « participe passé » d'une forme (verbe sans personne, porteur d'un
/// genre et d'un nombre).
fn participles(text: &str) -> Vec<Morph> {
    morpho::lookup(text)
        .into_iter()
        .filter(|m| {
            m.category == MorphCategory::Verb
                && m.person.is_none()
                && m.gender.is_some()
                && m.number.is_some()
        })
        .collect()
}

/// Lemme commun à toutes ces analyses, ou `None` s'il y en a plusieurs.
fn unique_lemma(analyses: &[Morph]) -> Option<&str> {
    let mut it = analyses.iter().map(|m| m.lemma.as_str());
    let first = it.next()?;
    it.all(|l| l == first).then_some(first)
}

impl PastParticipleAvoir {
    /// Cœur de la règle. `pron_ok(idx)` confirme que le jeton d'index d'origine
    /// `idx` est bien un pronom (filtre POS optionnel).
    fn run(&self, tokens: &[Token], pron_ok: impl Fn(usize) -> bool) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for p in 0..lex.len() {
                let parts = participles(&lex[p].1.text);
                if parts.is_empty() {
                    continue;
                }

                // Auxiliaire : jeton lexical précédent, en sautant les adverbes.
                let mut a = p;
                let aux = loop {
                    if a == 0 {
                        break None;
                    }
                    a -= 1;
                    if is_skippable_adverb(&lex[a].1.text) {
                        continue;
                    }
                    break Some(a);
                };
                let Some(a) = aux else { continue };
                if !is_avoir(&lex[a].1.text) || a == 0 {
                    continue;
                }

                // COD antéposé : jeton immédiatement avant l'auxiliaire.
                let cod = lex[a - 1];
                let Some((gender, number)) = cod_features(&cod.1.text) else {
                    continue;
                };
                if !pron_ok(cod.0) {
                    continue;
                }

                // Déjà accordé ? (une analyse de participe compatible suffit)
                let agrees = parts.iter().any(|m| {
                    m.number == Some(number) && gender.map_or(true, |g| m.gender == Some(g))
                });
                if agrees {
                    continue;
                }

                let Some(lemma) = unique_lemma(&parts) else {
                    continue;
                };

                // Génération : si le genre est connu, une forme ; sinon (les) les
                // deux graphies plurielles, masculin d'abord (défaut conventionnel).
                let genders: &[Gender] = match gender {
                    Some(Gender::Feminine) => &[Gender::Feminine],
                    Some(Gender::Masculine) => &[Gender::Masculine],
                    _ => &[Gender::Masculine, Gender::Feminine],
                };
                let mut replacements: Vec<String> = Vec::new();
                for &g in genders {
                    if let Some(form) = morpho::participle(lemma, g, number) {
                        let cased = match_case(&lex[p].1.text, &form);
                        if !cased.eq_ignore_ascii_case(&lex[p].1.text)
                            && !replacements.contains(&cased)
                        {
                            replacements.push(cased);
                        }
                    }
                }
                if replacements.is_empty() {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: lex[p].1.span,
                    message: format!(
                        "Accord du participe passé : « {} » doit s'accorder avec le complément \
                         d'objet direct antéposé.",
                        lex[p].1.text
                    ),
                    replacements,
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }
}

impl Rule for PastParticipleAvoir {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.run(tokens, |_| true)
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        self.run(tokens, |idx| tags[idx].upos == Upos::Pron)
    }

    fn name(&self) -> &'static str {
        "Accord du participe passé (avoir + COD antéposé)"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn run(text: &str) -> Vec<Suggestion> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        PastParticipleAvoir.check_tagged(&tokens, &tags)
    }

    fn first(text: &str) -> Option<Vec<String>> {
        run(text).into_iter().next().map(|s| s.replacements)
    }

    fn count(text: &str) -> usize {
        run(text).len()
    }

    #[test]
    fn les_offers_both_plural_genders() {
        assert_eq!(
            first("je les ai vu").as_deref(),
            Some(["vus".to_string(), "vues".to_string()].as_slice())
        );
    }

    #[test]
    fn les_with_intervening_adverb() {
        assert_eq!(
            first("je ne les ai pas vu").as_deref(),
            Some(["vus".to_string(), "vues".to_string()].as_slice())
        );
    }

    #[test]
    fn la_gives_feminine_singular() {
        assert_eq!(
            first("il la a vu").as_deref(),
            Some(["vue".to_string()].as_slice())
        );
    }

    #[test]
    fn already_agreed_is_silent() {
        for ok in ["je les ai vus", "je les ai vues", "il la a vue"] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn subject_pronoun_is_not_cod() {
        // « nous »/« vous » sujets : pas de COD antéposé, aucun accord.
        assert_eq!(count("nous avons mangé"), 0);
        assert_eq!(count("vous avez compris"), 0);
    }

    #[test]
    fn etre_auxiliary_is_out_of_scope() {
        // L'accord avec être relève d'une autre règle ; rien ici.
        assert_eq!(count("elles sont parties"), 0);
    }
}
