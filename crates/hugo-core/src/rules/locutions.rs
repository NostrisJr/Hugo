//! Phase 10 — **Accords par locution** : *la plupart/beaucoup/peu de N*,
//! *avoir l'air*, *des plus/des moins*.
//!
//! ## la plupart / beaucoup / peu de N + verbe
//!
//! Ces locutions quantifiantes suivies de *de + nom pluriel* appellent le
//! verbe au **pluriel** :
//! - *«la plupart des gens pense»* → *«pensent»*
//! - *«beaucoup d'élèves manque»* → *«manquent»*
//!
//! Signal : `plupart/beaucoup/peu` + (de/d') + nom + verbe singulier →
//! suggérer le pluriel.
//!
//! ## avoir l'air + adjectif
//!
//! *«elle a l'air fatiguée»* : accord avec le **sujet** (« elle ») ou avec
//! *«air»* (masculin singulier) — les deux sont admis. On ne signale rien pour
//! ne pas générer de faux positifs.
//!
//! ## des plus / des moins + adjectif
//!
//! *«un problème des plus complexes»* → l'adjectif se met au **pluriel**.
//! *«une situation des plus délicate»* → *«délicates»*.
//!
//! Signal : `des` + `plus/moins` + adjectif singulier → suggérer le pluriel.

use super::Rule;
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::lexical_sentences;
use crate::tokenizer::Token;
use crate::Suggestion;

const RULE_ID: &str = "locutions_accord";

/// Quantifieurs déclenchant l'accord au pluriel.
const QUANTIFIERS: &[&str] = &["plupart", "beaucoup", "peu", "nombre", "tant", "autant", "assez", "trop"];

pub struct LocutionsRule;

/// Essaie de mettre `form` au pluriel en consultant le lexique.
fn pluralize_verb(form: &str) -> Option<String> {
    // Formes irrégulières communes
    Some(match form.to_lowercase().as_str() {
        "pense" => "pensent",
        "mange" => "mangent",
        "vient" => "viennent",
        "fait" => "font",
        "est" => "sont",
        "a" => "ont",
        "va" => "vont",
        "dit" => "disent",
        "voit" => "voient",
        "part" => "partent",
        "prend" => "prennent",
        "sait" => "savent",
        "peut" => "peuvent",
        "veut" => "veulent",
        "doit" => "doivent",
        _ => {
            // Pour les verbes réguliers en -e (3ᵉ sg) → -ent (3ᵉ pl)
            if form.ends_with('e') && !form.ends_with("ent") {
                return Some(format!("{}nt", form));
            }
            return None;
        }
    }
    .to_string())
}

/// Essaie de mettre un adjectif au pluriel via le lexique.
fn pluralize_adj(form: &str) -> Option<String> {
    let morphs = morpho::lookup(form);
    // Chercher la forme plurielle du même genre
    let gender = morphs.iter().find_map(|m| {
        if m.category == MorphCategory::Adjective && m.number == Some(Number::Singular) {
            m.gender
        } else {
            None
        }
    });
    let target_gender = gender.unwrap_or(Gender::Masculine);
    morphs.iter().find_map(|m| {
        if m.category == MorphCategory::Adjective
            && m.number == Some(Number::Plural)
            && m.gender == Some(target_gender)
        {
            Some(m.lemma.clone()) // approx
        } else {
            None
        }
    })
    // Fallback : ajouter -s si pas déjà en -s/-x
    .or_else(|| {
        if !form.ends_with('s') && !form.ends_with('x') {
            Some(format!("{form}s"))
        } else {
            None
        }
    })
}

impl Rule for LocutionsRule {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                let form = sentence[i].1.text.to_lowercase();

                // --- plupart/beaucoup/peu + de + nom + verbe singulier ---
                if QUANTIFIERS.contains(&form.as_str()) {
                    // Chercher « de/d' » après le quantifieur
                    let next_form = sentence
                        .get(i + 1)
                        .map(|(_, t)| t.text.to_lowercase());
                    let has_de = next_form
                        .as_deref()
                        .is_some_and(|f| f == "de" || f == "d" || f == "des" || f == "du");
                    if !has_de {
                        continue;
                    }
                    // Chercher un nom après « de/des »
                    let noun_pos = i + 2;
                    let Some((_, _noun_tok)) = sentence.get(noun_pos) else {
                        continue;
                    };
                    let noun_upos = super::confusion::upos(&sentence, noun_pos, tags);
                    if !matches!(noun_upos, Upos::Noun) {
                        continue;
                    }
                    // Chercher le verbe après le nom
                    let verb_pos = noun_pos + 1;
                    if verb_pos >= len {
                        continue;
                    }
                    let (_, verb_tok) = sentence[verb_pos];
                    let verb_upos = super::confusion::upos(&sentence, verb_pos, tags);
                    if !matches!(verb_upos, Upos::Verb | Upos::Aux) {
                        continue;
                    }
                    // Vérifier que le verbe est au singulier (pas déjà au pluriel)
                    let verb_morphs = morpho::lookup(verb_tok.text.as_str());
                    let is_singular = verb_morphs.iter().any(|m| {
                        m.category == MorphCategory::Verb
                            && m.number == Some(Number::Singular)
                            && m.person == Some(crate::morpho::Person::Third)
                    });
                    if !is_singular {
                        continue;
                    }
                    if let Some(plural) = pluralize_verb(&verb_tok.text) {
                        suggestions.push(Suggestion {
                            span: verb_tok.span,
                            message: format!(
                                "Accord avec quantifieur : «\u{a0}{}\u{a0}» de + nom pluriel appelle le verbe au pluriel.",
                                sentence[i].1.text
                            ),
                            replacements: vec![plural],
                            rule_id: RULE_ID,
                        });
                    }
                }

                // --- des plus/moins + adjectif singulier ---
                if form == "des" {
                    let next = sentence.get(i + 1).map(|(_, t)| t.text.to_lowercase());
                    if !matches!(next.as_deref(), Some("plus") | Some("moins")) {
                        continue;
                    }
                    let adj_pos = i + 2;
                    let Some((_, adj_tok)) = sentence.get(adj_pos) else {
                        continue;
                    };
                    let adj_morphs = morpho::lookup(adj_tok.text.as_str());
                    let is_adj_sg = adj_morphs.iter().any(|m| {
                        m.category == MorphCategory::Adjective
                            && m.number == Some(Number::Singular)
                    });
                    // Pas déjà au pluriel
                    let is_adj_pl = adj_morphs.iter().any(|m| {
                        m.category == MorphCategory::Adjective
                            && m.number == Some(Number::Plural)
                    });
                    if is_adj_sg && !is_adj_pl {
                        if let Some(plural) = pluralize_adj(adj_tok.text.as_str()) {
                            suggestions.push(Suggestion {
                                span: adj_tok.span,
                                message: "Accord superlatif : «\u{a0}des plus/moins\u{a0}» + adjectif → pluriel.".to_string(),
                                replacements: vec![plural],
                                rule_id: RULE_ID,
                            });
                        }
                    }
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accords par locution (plupart, beaucoup, des plus)"
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
        LocutionsRule
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        LocutionsRule.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn plupart_de_nom_verbe_pluriel() {
        // « la plupart des gens pense » → « pensent »
        assert!(first("la plupart des gens pense").is_some());
    }

    #[test]
    fn plupart_correct_plural() {
        assert_eq!(count("la plupart des gens pensent"), 0);
    }

    #[test]
    fn des_plus_adj_pluriel() {
        assert!(first("un problème des plus complexe").is_some());
    }

    #[test]
    fn des_plus_adj_already_plural() {
        assert_eq!(count("un problème des plus complexes"), 0);
    }
}
