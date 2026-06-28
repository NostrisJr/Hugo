//! Règle : confusion **et / est** — **tranche 6** du moteur de confusions.
//!
//! Mémo : *«et»* est une conjonction de coordination ; *«est»* est la 3ᵉ
//! personne du singulier du verbe *être*. On peut tester en remplaçant par
//! *«était»* : si ça tient, c'est *«est»*.
//!
//! ## et → est
//!
//! **Sujet singulier de 3ᵉ personne + `et` + attribut/adjectif/PP** :
//! - `il et content` → `est`
//! - `elle et belle` → `est`
//! - `le chat et gris` → `est`
//!
//! On exige que « et » soit précédé d'un sujet singulier et suivi d'un attribut
//! (adjectif, participe passé ou nom sans déterminant indéfini). La coordination
//! de verbes (`mange et dort`) ou de noms (`le chat et le chien`) reste intacte.
//!
//! ## est → et (plus rare)
//!
//! Non traité : la direction « est → et » n'a pas de signal séparable fiable
//! (« elle est grande et belle » — le « est » correct ressemble à une copule).

use super::{is_past_participle, normalize};
use crate::dep::DepRel;
use crate::morpho::{self, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::Rule;
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct EtEstConfusion;

const RULE_ID: &str = "confusion_et_est";

/// Pronoms sujets singuliers de 3ᵉ personne.
const SING_3P_PRONOUNS: &[&str] = &["il", "elle", "on", "ce", "c"];

/// Vrai si la forme est un adjectif ou un participe passé (attribut possible).
fn is_attributive(form: &str) -> bool {
    let morphs = morpho::lookup(form);
    if morphs.iter().any(|m| m.category == MorphCategory::Adjective) {
        return true;
    }
    is_past_participle(form)
}

/// Vrai si le jeton `idx` est un sujet singulier de 3ᵉ personne (pronom du jeu
/// fermé, ou nom singulier). Version pilotée par l'arbre : pas de garde SP
/// positionnelle — la coordination nom+nom est écartée en amont par la structure.
fn is_sing_3rd_subject(tokens: &[Token], tags: &[Tagged], idx: usize) -> bool {
    let form = normalize(tokens[idx].text.as_str());
    if SING_3P_PRONOUNS.contains(&form.as_str()) {
        return true;
    }
    if !matches!(tags[idx].upos, Upos::Noun | Upos::Propn) {
        return false;
    }
    let morphs = morpho::lookup(tokens[idx].text.as_str());
    if morphs.is_empty() {
        return true; // inconnu → supposer singulier
    }
    morphs.iter().any(|m| {
        m.category == MorphCategory::Noun
            && m.number == Some(Number::Singular)
            && m.person.is_none()
    })
}

impl Rule for EtEstConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut tags = crate::pos::tag(tokens);
        crate::dep::parse(tokens, &mut tags);
        self.check_tagged(tokens, &tags)
    }

    /// Détection pilotée par l'arbre : « et » est une conjonction de coordination
    /// (`cc`) rattachée au **second conjoint** C2 ; le premier conjoint C1 est la
    /// tête de C2 (relation `conj`). C'est une confusion avec « est » lorsque la
    /// coordination relie un **sujet** (C1) à un **attribut** (C2) — donc deux
    /// catégories **incompatibles**. Une coordination de même catégorie
    /// (nom+nom « problème et état », verbe+verbe « mange et dort ») est une vraie
    /// conjonction : on n'y touche pas. Plus besoin de garde SP positionnelle.
    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for i in 0..tokens.len() {
            if !tokens[i].is_lexical() || normalize(tokens[i].text.as_str()) != "et" {
                continue;
            }
            if tags[i].dep != DepRel::Cc {
                continue;
            }
            // Second conjoint C2 (tête du `cc`), premier conjoint C1 (tête `conj`).
            let Some(c2) = crate::dep::head_of(tags, i) else {
                continue;
            };
            if tags[c2].dep != DepRel::Conj {
                continue;
            }
            let Some(c1) = crate::dep::head_of(tags, c2) else {
                continue;
            };
            // Ordre attendu : C1 … et … C2.
            if !(c1 < i && i < c2) {
                continue;
            }
            // C1 doit être un sujet singulier de 3ᵉ personne.
            if !is_sing_3rd_subject(tokens, tags, c1) {
                continue;
            }
            // Coordination de deux nominaux → vraie conjonction (« problème et
            // état ») : on s'abstient. C'est la structure qui remplace la garde SP.
            if matches!(tags[c1].upos, Upos::Noun | Upos::Propn)
                && matches!(tags[c2].upos, Upos::Noun | Upos::Propn)
            {
                continue;
            }
            // C2 doit être un attribut : adjectif (POS) ou participe passé / adjectif
            // (lexique). Exclut « mange et dort » (C2 verbe non attributif).
            if !(matches!(tags[c2].upos, Upos::Adj) || is_attributive(&tokens[c2].text)) {
                continue;
            }

            suggestions.push(Suggestion {
                span: tokens[i].span,
                message: format!(
                    "Confusion «\u{a0}et\u{a0}»/«\u{a0}est\u{a0}» : «\u{a0}{}\u{a0}» est peut-être le verbe «\u{a0}être\u{a0}».",
                    tokens[i].text
                ),
                replacements: vec!["est".to_string()],
                rule_id: RULE_ID,
            });
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « et » / « est »"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn full_tags(tokens: &[Token]) -> Vec<Tagged> {
        let mut tags = crate::pos::tag(tokens);
        crate::dep::parse(tokens, &mut tags);
        tags
    }

    fn first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = full_tags(&tokens);
        EtEstConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = full_tags(&tokens);
        EtEstConfusion.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn et_to_est_pronoun_adj() {
        assert_eq!(first("il et content"), Some("est".into()));
        assert_eq!(first("elle et belle"), Some("est".into()));
    }

    #[test]
    fn et_to_est_pronoun_participle() {
        assert_eq!(first("il et parti"), Some("est".into()));
        assert_eq!(first("elle et arrivée"), Some("est".into()));
    }

    #[test]
    fn no_false_positive_conjunction() {
        // Coordination de verbes : ne pas toucher
        assert_eq!(count("il mange et dort"), 0);
        // Coordination de noms
        assert_eq!(count("le chat et le chien"), 0);
        // « est » correct
        assert_eq!(count("il est content"), 0);
    }

    #[test]
    fn no_false_positive_plural() {
        // Sujet pluriel : ne pas toucher
        assert_eq!(count("ils et contents"), 0);
    }

    #[test]
    fn no_false_positive_noun_coordination() {
        // Coordination de deux noms : « et » relie « problème » et « état » (deux
        // nominaux) → vraie conjonction, jamais le verbe. C'est la STRUCTURE
        // (conj nom+nom) qui l'écarte, plus une liste de prépositions.
        assert_eq!(count("Compréhension du problème et état de l'art"), 0);
        assert_eq!(count("la santé et la sécurité"), 0);
    }
}
