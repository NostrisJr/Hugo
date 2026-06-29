//! Règle : accord du participe passé des **verbes pronominaux** (auxiliaire
//! « être »).
//!
//! Dans « sujet + pronom réfléchi + être + participe », le participe s'accorde
//! le plus souvent avec le sujet : « elle s'est levé » → « levée », « ils se
//! sont assis » est correct, « elles se sont trompé » → « trompées ».
//!
//! Garde essentielle — le **COD postposé** : quand un objet direct suit le
//! participe (« elle s'est lavé **les mains** »), c'est lui le complément, le
//! pronom réfléchi est indirect, et le participe **reste invariable**. La règle
//! s'abstient alors (repérage : un déterminant ou un nom suit le participe).
//!
//! Approche prudente (comme l'accord de l'attribut) : sujet à **genre connu**
//! (pronoms `il/elle/ils/elles` ou nom au genre lexical déterminé) ; les pronoms
//! `je/tu/on/nous/vous`, au genre indéterminé, sont écartés.

use super::{lexical_sentences, Rule};
use crate::dep::DepRel;
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Accord du participe passé des verbes pronominaux.
pub struct PronominalParticiple;

const RULE_ID: &str = "pronominal_participle";

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

/// Valeur unique d'un trait à travers des analyses, ou `None` si absente ou
/// contradictoire.
fn consensus<T: PartialEq + Copy>(values: impl Iterator<Item = Option<T>>) -> Option<T> {
    let mut found: Option<T> = None;
    for v in values.flatten() {
        match found {
            None => found = Some(v),
            Some(prev) if prev == v => {}
            Some(_) => return None,
        }
    }
    found
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « être ».
fn is_etre(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "être")
}

/// Verbes **essentiellement transitifs indirects** (construction « à quelqu'un »)
/// ou intransitifs : leur pronom réfléchi est un **objet indirect**, donc le
/// participe reste **invariable** (« ils se sont parlé », « elles se sont
/// succédé », « ils se sont plu »). Liste curée d'après la Banque de dépannage
/// linguistique / Projet Voltaire — pas un accord avec le sujet.
const INDIRECT_REFLEXIVE_VERBS: &[&str] = &[
    "parler",
    "plaire",
    "complaire",
    "déplaire",
    "succéder",
    "nuire",
    "mentir",
    "ressembler",
    "sourire",
    "rire",
    "suffire",
    "survivre",
    "convenir",
    "téléphoner",
    "écrire",
    "répondre",
    "obéir",
    "désobéir",
    "appartenir",
];

/// Vrai si le participe (par son lemme verbal) appartient à un verbe dont le
/// réfléchi est indirect → participe invariable, accord à ne pas proposer.
fn is_indirect_reflexive(text: &str) -> bool {
    morpho::lookup(text)
        .iter()
        .filter(|m| m.category == MorphCategory::Verb)
        .any(|m| INDIRECT_REFLEXIVE_VERBS.contains(&m.lemma.as_str()))
}

/// Vrai si le jeton est un pronom réfléchi objet préverbal.
fn is_reflexive(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "se" | "s" | "me" | "m" | "te" | "t" | "nous" | "vous"
    )
}

/// Vrai si le jeton est la négation « ne ».
fn is_ne(text: &str) -> bool {
    matches!(normalize(text).as_str(), "ne" | "n")
}

/// Adverbe pouvant s'intercaler entre l'auxiliaire et le participe.
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
            | "vite"
            | "enfin"
    )
}

/// Genre et nombre d'un sujet à genre connu (pronom ou nom au genre lexical sûr).
fn subject_features(token: &Token) -> Option<(Gender, Number)> {
    match normalize(&token.text).as_str() {
        "il" => return Some((Gender::Masculine, Number::Singular)),
        "elle" => return Some((Gender::Feminine, Number::Singular)),
        "ils" => return Some((Gender::Masculine, Number::Plural)),
        "elles" => return Some((Gender::Feminine, Number::Plural)),
        // Pronoms au genre indéterminé : écartés.
        "je" | "j" | "tu" | "on" | "nous" | "vous" => return None,
        _ => {}
    }
    let nouns: Vec<_> = morpho::lookup(&token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if nouns.is_empty() {
        return None;
    }
    let gender = consensus(nouns.iter().map(|m| m.gender))?;
    let number = consensus(nouns.iter().map(|m| m.number))?;
    (gender != Gender::Epicene).then_some((gender, number))
}

/// Analyses « participe passé » d'une forme.
fn participles(text: &str) -> Vec<morpho::Morph> {
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

/// Vrai si le jeton (POS) introduit un objet direct postposé (déterminant ou
/// nom) — auquel cas le participe pronominal reste invariable.
fn opens_object(tag: Upos) -> bool {
    matches!(tag, Upos::Det | Upos::Noun | Upos::Propn)
}

impl PronominalParticiple {
    /// Émet la correction d'accord du participe `part_token` avec un sujet de
    /// genre/nombre donnés, ou `None` si déjà accordé / lemme ambigu / forme
    /// introuvable. Partagé par les chemins positionnel et arbre.
    fn suggest(part_token: &Token, gender: Gender, number: Number) -> Option<Suggestion> {
        // Verbe à réfléchi indirect (« se parler » = parler à) : invariable.
        if is_indirect_reflexive(&part_token.text) {
            return None;
        }
        let parts = participles(&part_token.text);
        if parts.is_empty() {
            return None;
        }
        if parts
            .iter()
            .any(|m| m.gender == Some(gender) && m.number == Some(number))
        {
            return None; // déjà accordé
        }
        let mut lemmas = parts.iter().map(|m| m.lemma.as_str());
        let lemma = lemmas.next().unwrap();
        if !lemmas.all(|l| l == lemma) {
            return None; // lemme ambigu
        }
        let corrected = morpho::participle(lemma, gender, number)?;
        if corrected.eq_ignore_ascii_case(&part_token.text) {
            return None;
        }
        Some(Suggestion {
            span: part_token.span,
            message: format!(
                "Accord du participe passé pronominal : « {} » doit s'accorder avec le sujet.",
                part_token.text
            ),
            replacements: vec![match_case(&part_token.text, &corrected)],
            rule_id: RULE_ID,
        })
    }

    /// Chemin positionnel (sans arbre) : `sujet [ne] réfléchi être [adv] PP`,
    /// garde COD postposé par le POS du token suivant. Sert au repli `check()`.
    fn positional(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for c in 0..lex.len() {
                if !is_etre(&lex[c].1.text) {
                    continue;
                }
                if c == 0 || !is_reflexive(&lex[c - 1].1.text) {
                    continue;
                }
                let mut s = c - 1;
                while s > 0 && is_ne(&lex[s - 1].1.text) {
                    s -= 1;
                }
                if s == 0 {
                    continue;
                }
                let Some((gender, number)) = subject_features(lex[s - 1].1) else {
                    continue;
                };
                let mut p = c + 1;
                while p < lex.len() && is_skippable_adverb(&lex[p].1.text) {
                    p += 1;
                }
                let Some(&(_, part_token)) = lex.get(p) else {
                    continue;
                };
                // Garde COD postposé : déterminant ou nom après le participe.
                if let Some(&(next_idx, _)) = lex.get(p + 1) {
                    if opens_object(tags[next_idx].upos) {
                        continue;
                    }
                }
                if let Some(sugg) = Self::suggest(part_token, gender, number) {
                    suggestions.push(sugg);
                }
            }
        }
        suggestions
    }

    /// Chemin **piloté par l'arbre** (production). Le participe pronominal porte
    /// un sujet (`nsubj`/`nsubj:pass`), un réfléchi préverbal (enfant clitique
    /// `se`/`s'`/`me`…) et son auxiliaire `être` (arc `aux`). Le **COD postposé**
    /// — qui rend le participe invariable (« elle s'est lavé **les mains** ») —
    /// est l'enfant `obj` situé **après** le participe ; on s'abstient alors.
    /// Plus robuste que le positionnel : sujet et COD repérés par-delà négation,
    /// adverbes et compléments intercalés.
    fn run_tree(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for p in 0..tokens.len() {
            if !tokens[p].is_lexical() || participles(&tokens[p].text).is_empty() {
                continue;
            }
            // Auxiliaire « être » (arc `aux`).
            let Some(aux) = crate::dep::child_with(tags, p, &[DepRel::Aux]) else {
                continue;
            };
            if !is_etre(&tokens[aux].text) {
                continue;
            }
            // Réfléchi préverbal : un enfant clitique réflexif avant le participe
            // (confirme le verbe pronominal, quelle que soit son étiquette
            // expl/iobj/obj selon le parser).
            let has_reflexive = crate::dep::children(tags, p)
                .into_iter()
                .any(|c| c < p && is_reflexive(&tokens[c].text));
            if !has_reflexive {
                continue;
            }
            // Sujet à genre connu (nsubj/nsubj:pass/csubj).
            let Some(subj) = crate::dep::subject_of(tags, p) else {
                continue;
            };
            // Sujet coordonné (« l'homme et la femme se sont rencontrés ») : la
            // tête ne porte qu'un conjoint singulier ; le vrai sujet est pluriel.
            // On s'abstient (le genre/nombre composé n'est pas calculé ici).
            if crate::dep::child_with(tags, subj, &[DepRel::Conj]).is_some() {
                continue;
            }
            let Some((gender, number)) = subject_features(&tokens[subj]) else {
                continue;
            };
            // Garde COD postposé : un objet direct après le participe → invariable.
            if let Some(obj) = crate::dep::child_with(tags, p, &[DepRel::Obj]) {
                if obj > p {
                    continue;
                }
            }
            if let Some(sugg) = Self::suggest(&tokens[p], gender, number) {
                suggestions.push(sugg);
            }
        }
        suggestions
    }
}

impl Rule for PronominalParticiple {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.positional(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        self.run_tree(tokens, tags)
    }

    fn name(&self) -> &'static str {
        "Accord du participe passé pronominal"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    /// Construit les tags comme le Checker (POS + arbre) et passe par le chemin
    /// de production `check_tagged`.
    fn tagged(text: &str) -> Vec<Suggestion> {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        PronominalParticiple.check_tagged(&tokens, &tags)
    }

    fn first(text: &str) -> Option<String> {
        tagged(text)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        tagged(text).len()
    }

    #[test]
    fn feminine_subject() {
        assert_eq!(first("elle s'est levé").as_deref(), Some("levée"));
    }

    #[test]
    fn plural_subjects() {
        assert_eq!(first("ils se sont levé").as_deref(), Some("levés"));
        assert_eq!(first("elles se sont trompé").as_deref(), Some("trompées"));
    }

    #[test]
    fn negation_is_handled() {
        assert_eq!(first("elle ne s'est pas levé").as_deref(), Some("levée"));
    }

    #[test]
    fn postposed_object_blocks_agreement() {
        // « elle s'est lavé les mains » : COD postposé → participe invariable.
        assert_eq!(count("elle s'est lavé les mains"), 0);
    }

    #[test]
    fn correct_agreement_is_silent() {
        for ok in [
            "elle s'est levée",
            "ils se sont levés",
            "il s'est levé",
            "elles se sont trompées",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn unknown_gender_subject_is_ignored() {
        // « je me suis levé » : genre du locuteur inconnu → aucune correction.
        assert_eq!(count("je me suis levé"), 0);
    }

    #[test]
    fn postposed_object_with_adverb_blocks_agreement() {
        // ROBUSTESSE : le COD postposé reste repéré par l'arbre malgré un adverbe
        // intercalé (le chemin positionnel ne regardait que le token suivant).
        assert_eq!(count("elle s'est soigneusement lavé les mains"), 0);
    }
}
