//! Règle : accord du participe passé à la **voix passive composée**.
//!
//! En voix passive composée (« le gâteau a été mangé »), le participe passé
//! s'accorde en genre et en nombre avec le **sujet** du verbe :
//! « le gâteau » → masculin singulier → « mangé » ;
//! « les fenêtres » → féminin pluriel → « ouvertes ».
//!
//! Construction détectée : `SUJET + avoir + « été » + participe passé`.
//! Le sujet est un pronom à genre connu (`il`, `elle`, `ils`, `elles`) ou un
//! nom commun dont le genre est déterminé par le lexique.
//!
//! Gardes :
//! - on ne traite que le passif **composé** (avoir + été + participe) ;
//!   le passif simple (être + participe) relève de l'accord d'attribut ;
//! - si le participe passé n'est pas identifiable comme tel dans le lexique,
//!   on s'abstient.

use super::{lexical_sentences, Rule};
use crate::dep::DepRel;
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct PassiveParticiple;

const RULE_ID: &str = "passive_participle";

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

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

fn is_avoir(text: &str) -> bool {
    morpho::verb_forms(text)
        .iter()
        .any(|v| v.lemma == "avoir")
}

fn is_skippable_adverb(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "pas" | "jamais" | "plus" | "déjà" | "bien" | "toujours" | "encore" | "vraiment"
            | "souvent" | "par"
    )
}

/// Analyses « participe passé » d'une forme (verbe sans personne, porteur d'un
/// genre et d'un nombre dans le lexique).
///
/// Repli : pour les PP irréguliers dont le masculin singulier est homographe
/// d'une forme conjuguée (« écrit », « pris »…), on synthétise une entrée PP
/// à partir de la lecture adjectivale (genre/nombre) et du lemme verbal.
fn participles(text: &str) -> Vec<morpho::Morph> {
    let all = morpho::lookup(text);

    let direct: Vec<_> = all
        .iter()
        .filter(|m| {
            m.category == MorphCategory::Verb
                && m.person.is_none()
                && m.gender.is_some()
                && m.number.is_some()
        })
        .cloned()
        .collect();
    if !direct.is_empty() {
        return direct;
    }

    // Repli : lemme verbal unique + lectures adjectivales avec genre/nombre.
    let verb_lemmas: std::collections::HashSet<&str> = all
        .iter()
        .filter(|m| m.category == MorphCategory::Verb)
        .map(|m| m.lemma.as_str())
        .collect();
    if verb_lemmas.len() != 1 {
        return vec![];
    }
    let lemma = verb_lemmas.into_iter().next().unwrap();
    all.iter()
        .filter(|m| {
            m.category == MorphCategory::Adjective
                && m.gender.is_some()
                && m.number.is_some()
        })
        .map(|m| morpho::Morph {
            lemma: lemma.to_string(),
            category: MorphCategory::Verb,
            gender: m.gender,
            number: m.number,
            person: None,
        })
        .collect()
}

fn unique_lemma(parts: &[morpho::Morph]) -> Option<String> {
    let mut it = parts.iter().map(|m| m.lemma.as_str());
    let first = it.next()?;
    it.all(|l| l == first).then(|| first.to_string())
}

/// Valeur unique d'un trait, ou `None` si contradictoire.
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

/// Genre et nombre du sujet : pronom à genre déterminé ou nom commun lexical.
fn subject_features(text: &str) -> Option<(Gender, Number)> {
    match normalize(text).as_str() {
        "il" => return Some((Gender::Masculine, Number::Singular)),
        "elle" => return Some((Gender::Feminine, Number::Singular)),
        "ils" => return Some((Gender::Masculine, Number::Plural)),
        "elles" => return Some((Gender::Feminine, Number::Plural)),
        _ => {}
    }
    let nouns: Vec<_> = morpho::lookup(text)
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

/// À partir d'un participe passé passif et du genre/nombre de son **sujet**,
/// renvoie la correction d'accord, ou `None` si déjà accordé / hors de portée.
/// Partagé par le chemin positionnel (`check`) et le chemin par l'arbre.
fn participle_suggestion(
    part_token: &Token,
    gender: Gender,
    number: Number,
    subj_text: &str,
) -> Option<Suggestion> {
    let parts = participles(&part_token.text);
    let (lemma, already_agreed) = if !parts.is_empty() {
        let agreed = parts
            .iter()
            .any(|m| m.gender == Some(gender) && m.number == Some(number));
        let lemma = unique_lemma(&parts)?;
        (lemma, agreed)
    } else {
        // Repli : forme conjuguée sans lecture PP ni adjectivale.
        let all = morpho::lookup(&part_token.text);
        let vlemmas: std::collections::HashSet<&str> = all
            .iter()
            .filter(|m| m.category == MorphCategory::Verb)
            .map(|m| m.lemma.as_str())
            .collect();
        if vlemmas.len() != 1 {
            return None;
        }
        (vlemmas.into_iter().next().unwrap().to_string(), false)
    };

    if already_agreed {
        return None;
    }
    let corrected = morpho::participle(&lemma, gender, number)?;
    if corrected.eq_ignore_ascii_case(&part_token.text) {
        return None;
    }
    Some(Suggestion {
        span: part_token.span,
        message: format!(
            "Accord du participe passé passif : « {} » doit s'accorder avec le sujet « {} ».",
            part_token.text, subj_text
        ),
        replacements: vec![match_case(&part_token.text, &corrected)],
        rule_id: RULE_ID,
    })
}

impl Rule for PassiveParticiple {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for e in 0..lex.len() {
                // Cherche « été ».
                if normalize(&lex[e].1.text) != "été" {
                    continue;
                }

                // Auxiliaire avoir avant « été » (en sautant les adverbes).
                let mut a = e;
                let aux_pos = loop {
                    if a == 0 {
                        break None;
                    }
                    a -= 1;
                    if is_skippable_adverb(&lex[a].1.text) {
                        continue;
                    }
                    break Some(a);
                };
                let Some(a) = aux_pos else { continue };
                if !is_avoir(&lex[a].1.text) || a == 0 {
                    continue;
                }

                // Participe passé après « été » (en sautant les adverbes).
                let mut q = e + 1;
                while q < lex.len() && is_skippable_adverb(&lex[q].1.text) {
                    q += 1;
                }
                let Some(&(_, part_token)) = lex.get(q) else {
                    continue;
                };

                // Sujet : pour un GN (« le gâteau »), le nom tête est en a-1.
                let subj_cand = lex[a - 1].1;
                let Some((gender, number)) = subject_features(&subj_cand.text) else {
                    continue;
                };

                if let Some(s) =
                    participle_suggestion(part_token, gender, number, &subj_cand.text)
                {
                    suggestions.push(s);
                }
            }
        }
        suggestions
    }

    /// Chemin de production : le sujet du participe passif est lu directement
    /// dans l'arbre via `nsubj:pass` (ou `nsubj`), au lieu d'être deviné à la
    /// position `aux − 1`. Indispensable quand le sujet est **éloigné** du verbe
    /// (« Deux solutions visant à automatiser la migration ont été citées » : le
    /// sujet est « solutions », pas « migration »).
    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for p in 0..tokens.len() {
            if tags[p].upos != Upos::Verb {
                continue;
            }
            let kids = crate::dep::children(tags, p);
            // Passif composé : auxiliaire passif « été » (aux:pass) + auxiliaire
            // « avoir » (aux). On reste sur le passif composé pour ne pas
            // empiéter sur la règle d'attribut (passif simple « est mangé »).
            let has_passive_aux = kids.iter().any(|&c| tags[c].dep == DepRel::AuxPass);
            if !has_passive_aux {
                continue;
            }
            let has_avoir = kids
                .iter()
                .any(|&c| tags[c].dep == DepRel::Aux && is_avoir(&tokens[c].text));
            if !has_avoir {
                continue;
            }
            let Some(s_idx) = crate::dep::subject_of(tags, p) else {
                continue;
            };
            let Some((gender, number)) = subject_features(&tokens[s_idx].text) else {
                continue;
            };
            if let Some(s) =
                participle_suggestion(&tokens[p], gender, number, &tokens[s_idx].text)
            {
                suggestions.push(s);
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord du participe passé (voix passive)"
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
        PassiveParticiple
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        PassiveParticiple.check(&tokenize(text)).len()
    }

    /// Chemin de production (POS + dépendances).
    fn first_tagged(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        PassiveParticiple
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count_tagged(text: &str) -> usize {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        PassiveParticiple.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn distant_subject_via_tree() {
        // Sujet ÉLOIGNÉ du verbe (séparé par une proposition participiale) : le
        // sujet est « solutions » (nsubj:pass), pas « migration » (le mot juste
        // avant l'auxiliaire). Phrase correcte → silence (pas de faux positif).
        assert_eq!(
            count_tagged(
                "Deux solutions visant à automatiser la migration ont été citées."
            ),
            0
        );
        // Vrai désaccord avec le sujet éloigné → corrigé vers le pluriel féminin.
        assert_eq!(
            first_tagged("Deux solutions visant à automatiser la migration ont été cité.")
                .as_deref(),
            Some("citées")
        );
    }

    #[test]
    fn masc_sing_subject_plural_participle() {
        assert_eq!(first("Le gâteau a été mangés par les enfants.").as_deref(), Some("mangé"));
    }

    #[test]
    fn fem_plural_subject_masc_sing_participle() {
        assert_eq!(first("Les fenêtres ont été ouvert par le vent.").as_deref(), Some("ouvertes"));
    }

    #[test]
    fn fem_sing_subject_masc_participle() {
        assert_eq!(first("La voiture a été réparé par le mécanicien.").as_deref(), Some("réparée"));
    }

    #[test]
    fn fem_plural_signale() {
        assert_eq!(first("Les erreurs ont été signalé par l'auditeur.").as_deref(), Some("signalées"));
    }

    #[test]
    fn pronoun_subject_elle() {
        assert_eq!(first("Elle a été blessé.").as_deref(), Some("blessée"));
    }

    #[test]
    fn correct_passive_is_silent() {
        for ok in [
            "Le gâteau a été mangé par les enfants.",
            "Les fenêtres ont été ouvertes par le vent.",
            "La voiture a été réparée par le mécanicien.",
            "Il a été invité à la fête.",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }
}

#[cfg(test)]
mod integration {
    use crate::rules::past_participle::PastParticipleAvoir;
    use crate::rules::imperatif::ImperatifGroupe1;
    use crate::rules::Rule;
    use crate::pos;
    use crate::tokenizer::tokenize;
    use super::*;

    fn check_all(text: &str) -> Vec<String> {
        let tokens = tokenize(text);
        // Comme le Checker : POS puis arbre de dépendances (lu par
        // PastParticipleAvoir::check_tagged).
        let mut tags = pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        let mut out = vec![];
        for s in ImperatifGroupe1.check(&tokens) { out.extend(s.replacements); }
        for s in PastParticipleAvoir.check_tagged(&tokens, &tags) { out.extend(s.replacements); }
        for s in PassiveParticiple.check(&tokens) { out.extend(s.replacements); }
        out
    }

    #[test]
    fn integration_cases() {
        let cases = [
            ("Vas faire les courses.", vec!["Va"]),
            // COD relatif — genre connu dans Lexique383
            ("Les fleurs que tu as cueilli sont jolies.", vec!["cueillies"]),
            // Voix passive
            ("Le gâteau a été mangés par les enfants.", vec!["mangé"]),
            ("Les fenêtres ont été ouvert par le vent.", vec!["ouvertes"]),
            ("Les erreurs ont été signalé par l'auditeur.", vec!["signalées"]),
            ("La voiture a été réparé par le mécanicien.", vec!["réparée"]),
        ];
        for (text, expected) in cases {
            let got = check_all(text);
            for exp in &expected {
                assert!(got.iter().any(|g| g == exp),
                    "MISSED {text:?}: expected {exp:?} in {got:?}");
            }
        }
    }

    #[test]
    fn no_false_positive_passive_correct() {
        let correct = [
            "Le gâteau a été mangé.",
            "Les fenêtres ont été ouvertes.",
            "La voiture a été réparée.",
            "Va faire les courses.",
        ];
        for text in correct {
            let got = check_all(text);
            assert!(got.is_empty(), "FALSE POSITIVE {text:?}: got {got:?}");
        }
    }
}

