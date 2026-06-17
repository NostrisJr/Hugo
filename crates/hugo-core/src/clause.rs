//! Segmentation légère en propositions (clauses).
//!
//! Ce module identifie, dans la séquence de tokens d'une phrase, les frontières
//! de propositions, la position du verbe fini, et celle du sujet. Il est utilisé
//! par les règles nécessitant une structure syntaxique légère — notamment
//! l'accord de l'adjectif apposé avec un sujet postposé (phase 12).
//!
//! L'implémentation est **déterministe** et O(n) : pas de nouveau modèle CRF.
//! La structure est construite en un seul passage sur les tags déjà produits par
//! le CRF, sans coût supplémentaire significatif.

use crate::morpho::{self, MorphCategory};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::{Token, TokenKind};

/// Une proposition dans un texte : ses bornes (indices de tokens), la position
/// du verbe fini, celle du sujet, et un indicateur d'inversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// Index du premier token de la proposition dans la tranche complète.
    pub start: usize,
    /// Index exclusif du premier token **hors** de la proposition.
    pub end: usize,
    /// Index du verbe fini dans la tranche complète, s'il est identifié.
    pub verb_pos: Option<usize>,
    /// Index de la tête nominale ou pronominale du sujet, si identifiée.
    pub subject_pos: Option<usize>,
    /// Vrai si le sujet nominal vient **après** le verbe (inversion stylistique).
    pub inverted: bool,
}

/// Vrai si le token marque une frontière de proposition (virgule ou ponctuation
/// terminale). Virgule → frontière de sous-proposition ; `.!?…;:` → fin de phrase.
fn is_clause_boundary(tok: &Token) -> bool {
    tok.kind == TokenKind::Punctuation
        && matches!(tok.text.as_str(), "," | "." | "!" | "?" | "…" | ";" | ":")
}

/// Vrai si le token est un verbe fini (a au moins une analyse avec une personne).
fn is_finite_verb(tok: &Token) -> bool {
    // morpho::verb_forms renvoie les formes finies (indicatif, subjonctif…) ;
    // on complète par les lectures morpho brutes pour les formes manquantes.
    !morpho::verb_forms(&tok.text).is_empty()
        || morpho::lookup(&tok.text)
            .iter()
            .any(|m| m.category == MorphCategory::Verb && m.person.is_some())
}

/// Vrai si le token est un pronom personnel sujet.
fn is_subject_pronoun(text: &str) -> bool {
    let lower = text.to_lowercase();
    let s = lower.trim_end_matches(['\'', '\u{2019}']);
    matches!(
        s,
        "je" | "j" | "tu" | "il" | "elle" | "on" | "nous" | "vous" | "ils" | "elles"
    )
}

/// Segmente la séquence de tokens en propositions.
///
/// Coupe sur les virgules et les terminateurs de phrase (`.,!?…;:`). Pour chaque
/// segment, identifie le verbe fini et (si possible) le sujet — avant le verbe
/// (ordre normal) ou après (inversion).
///
/// Les segments vides (deux frontières consécutives) sont omis.
pub fn segment_clauses(tokens: &[Token], tags: &[Tagged]) -> Vec<Clause> {
    let mut result = Vec::new();
    let mut seg_start = 0usize;

    for i in 0..tokens.len() {
        if is_clause_boundary(&tokens[i]) {
            if seg_start < i {
                result.push(build_clause(tokens, tags, seg_start, i));
            }
            seg_start = i + 1;
        }
    }
    if seg_start < tokens.len() {
        result.push(build_clause(tokens, tags, seg_start, tokens.len()));
    }
    result
}

fn build_clause(tokens: &[Token], tags: &[Tagged], start: usize, end: usize) -> Clause {
    let verb_pos = (start..end).find(|&i| {
        tokens[i].is_lexical()
            && matches!(tags[i].upos, Upos::Verb | Upos::Aux)
            && is_finite_verb(&tokens[i])
    });

    let (subject_pos, inverted) = if let Some(vp) = verb_pos {
        let pre = find_subject_before(tokens, tags, start, vp);
        if pre.is_some() {
            (pre, false)
        } else {
            let post = find_subject_after(tokens, tags, vp + 1, end);
            let inv = post.is_some();
            (post, inv)
        }
    } else {
        (None, false)
    };

    Clause {
        start,
        end,
        verb_pos,
        subject_pos,
        inverted,
    }
}

/// Cherche le sujet **avant** le verbe (ordre direct).
///
/// Remonte depuis la position du verbe vers le début de la proposition en
/// sautant les adjectifs et les adverbes. S'arrête sur un nom ou un pronom
/// sujet. Abandonne sur une préposition ou un autre verbe.
fn find_subject_before(tokens: &[Token], tags: &[Tagged], start: usize, verb: usize) -> Option<usize> {
    for i in (start..verb).rev() {
        if !tokens[i].is_lexical() {
            continue;
        }
        match tags[i].upos {
            Upos::Pron if is_subject_pronoun(&tokens[i].text) => return Some(i),
            Upos::Noun | Upos::Propn => return Some(i),
            Upos::Adp | Upos::Verb | Upos::Aux => return None,
            _ => {}
        }
    }
    None
}

/// Cherche le sujet **après** le verbe (inversion stylistique).
///
/// Avance depuis la position suivant le verbe. Saute déterminants, adjectifs
/// et adverbes. S'arrête sur un nom ou un pronom sujet.
fn find_subject_after(tokens: &[Token], tags: &[Tagged], from: usize, end: usize) -> Option<usize> {
    for i in from..end {
        if !tokens[i].is_lexical() {
            continue;
        }
        match tags[i].upos {
            Upos::Det | Upos::Adj | Upos::Adv => {}
            Upos::Noun | Upos::Propn => return Some(i),
            Upos::Pron if is_subject_pronoun(&tokens[i].text) => return Some(i),
            Upos::Verb | Upos::Aux => return None,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pos;
    use crate::tokenizer::tokenize;

    fn clauses(text: &str) -> Vec<Clause> {
        let tokens = tokenize(text);
        let tags = pos::tag(&tokens);
        segment_clauses(&tokens, &tags)
    }

    #[test]
    fn simple_direct_order() {
        // Ordre normal : sujet avant verbe.
        let cs = clauses("Les soldats partent.");
        let main = cs.iter().find(|c| c.verb_pos.is_some());
        assert!(main.is_some());
        let c = main.unwrap();
        assert!(c.verb_pos.is_some());
        assert!(c.subject_pos.is_some());
        assert!(!c.inverted);
    }

    #[test]
    fn inverted_subject() {
        // Inversion : verbe avant sujet.
        let cs = clauses("Partirent les soldats.");
        let main = cs.iter().find(|c| c.verb_pos.is_some());
        assert!(main.is_some());
        let c = main.unwrap();
        assert!(c.inverted, "le sujet devrait être détecté comme postposé");
        assert!(c.subject_pos.is_some());
    }

    #[test]
    fn comma_creates_subclauses() {
        // Une virgule divise la phrase en deux propositions.
        let cs = clauses("Fatigués, ils dorment.");
        assert!(cs.len() >= 2, "devrait produire au moins 2 clauses");
    }

    #[test]
    fn no_verb_no_subject() {
        let cs = clauses("Silence.");
        assert!(cs.iter().all(|c| c.verb_pos.is_none()));
    }
}
