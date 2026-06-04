//! Règle : subjonctif après certaines conjonctions de subordination.
//!
//! Quelques locutions conjonctives imposent le subjonctif dans la subordonnée :
//! « bien qu'il est ici » → « bien qu'il soit ici », « pour que tu viens » →
//! « pour que tu viennes ».
//!
//! Approche prudente, pour éviter les faux positifs :
//!
//! - la conjonction appartient à un **ensemble fermé sans ambiguïté** (sont
//!   écartées « de sorte que », « moins que »… qui admettent l'indicatif) ;
//! - le sujet est un **pronom personnel** (« qu'il ») **ou nominal** (« que les
//!   enfants », 3ᵉ personne, nombre lu sur le nom ou son déterminant), repéré via
//!   les étiquettes POS ;
//! - on ne corrige que si le verbe est à l'**indicatif présent** sans forme
//!   subjonctive *identique* : « mange » vaut pour l'indicatif **et** le
//!   subjonctif, donc « bien qu'il mange » est déjà correct et n'est pas touché ;
//! - la forme subjonctive est engendrée par [`morpho::conjugate`] ; introuvable,
//!   on n'émet rien.
//!
//! L'**indicatif imparfait** (« bien qu'il était ») est volontairement écarté :
//! le corriger en subjonctif présent (« soit ») changerait le temps, et le
//! subjonctif imparfait (« fût ») est littéraire — la concordance des temps est
//! laissée de côté. La catégorie verbale du candidat est confirmée par son
//! étiquette POS ([`Rule::check_tagged`]).

use super::{lexical_sentences, Rule};
use crate::morpho::{self, MoodTense, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte un indicatif là où une conjonction impose le subjonctif.
pub struct SubjunctiveAfterConjunction;

const RULE_ID: &str = "subjunctive";

/// Mots qui, immédiatement suivis de « que », forment une conjonction exigeant
/// le subjonctif. Restreint aux locutions **non ambiguës** (« de sorte que »,
/// « moins que », « après que »… sont volontairement exclues).
const TRIGGER_WORDS: &[&str] = &[
    "bien",
    "afin",
    "pour",
    "avant",
    "sans",
    "pourvu",
    "condition",
    "peur",
    "crainte",
    "quoi",
];

/// Minuscules + apostrophe finale ôtée (« qu' » → « qu », « j' » → « j »).
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

/// Personne et nombre d'un pronom personnel sujet.
fn pronoun_person_number(text: &str) -> Option<(Person, Number)> {
    Some(match normalize(text).as_str() {
        "je" | "j" => (Person::First, Number::Singular),
        "tu" => (Person::Second, Number::Singular),
        "il" | "elle" | "on" => (Person::Third, Number::Singular),
        "nous" => (Person::First, Number::Plural),
        "vous" => (Person::Second, Number::Plural),
        "ils" | "elles" => (Person::Third, Number::Plural),
        _ => return None,
    })
}

/// Vrai si le jeton est un clitique préverbal pouvant séparer le sujet du verbe.
fn is_clitic(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "ne" | "n"
            | "me"
            | "m"
            | "te"
            | "t"
            | "se"
            | "s"
            | "le"
            | "la"
            | "les"
            | "lui"
            | "leur"
            | "y"
            | "en"
    )
}

/// Vrai si la position `t` de la phrase ouvre une subordonnée au subjonctif.
fn is_trigger(lex: &[(usize, &Token)], t: usize) -> bool {
    let cur = normalize(lex[t].1.text.as_str());
    if cur == "quoique" || cur == "quoiqu" {
        return true;
    }
    (cur == "que" || cur == "qu")
        && t > 0
        && TRIGGER_WORDS.contains(&normalize(lex[t - 1].1.text.as_str()).as_str())
}

/// Si `text` est à l'indicatif présent pour `(person, number)` **sans** forme
/// subjonctive présente identique, renvoie la forme subjonctive correspondante.
fn subjunctive_form(text: &str, person: Person, number: Number) -> Option<String> {
    let forms = morpho::verb_forms(text);
    let matches = |mt: MoodTense, v: &morpho::VerbForm| {
        v.mood_tense == mt && v.person == person && v.number == number
    };
    let already_subjunctive = forms
        .iter()
        .any(|v| matches(MoodTense::SubjunctivePresent, v));
    if already_subjunctive {
        return None;
    }
    // Lemme unique parmi les analyses « indicatif présent » à cette personne.
    let mut lemmas = forms
        .iter()
        .filter(|v| matches(MoodTense::IndicativePresent, v))
        .map(|v| v.lemma.as_str());
    let lemma = lemmas.next()?;
    if !lemmas.all(|l| l == lemma) {
        return None;
    }
    let sub = morpho::conjugate(lemma, MoodTense::SubjunctivePresent, person, number)?;
    (!sub.eq_ignore_ascii_case(text)).then_some(sub)
}

/// Nombre imposé par un déterminant (pour un sujet nominal).
fn determiner_number(text: &str) -> Option<Number> {
    Some(match normalize(text).as_str() {
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => Number::Plural,
        "le" | "la" | "l" | "un" | "une" | "ce" | "cet" | "cette" | "mon" | "ton" | "son"
        | "ma" | "ta" | "sa" | "notre" | "votre" | "leur" | "du" => Number::Singular,
        _ => return None,
    })
}

/// Nombre d'un nom si toutes ses analyses nominales s'accordent.
fn noun_number(text: &str) -> Option<Number> {
    let mut found: Option<Number> = None;
    for m in morpho::lookup(text) {
        if m.category != MorphCategory::Noun {
            continue;
        }
        match (m.number, found) {
            (Some(n), None) => found = Some(n),
            (Some(n), Some(p)) if n != p => return None,
            _ => {}
        }
    }
    found
}

/// Index lexical du premier jeton non clitique à partir de `v`.
fn skip_clitics(lex: &[(usize, &Token)], mut v: usize) -> usize {
    while v < lex.len() && is_clitic(&lex[v].1.text) {
        v += 1;
    }
    v
}

/// Personne, nombre et index lexical du verbe d'une subordonnée ouverte par la
/// conjonction en position `t`. Gère le sujet **pronominal** (« qu'il ») et le
/// sujet **nominal** (« que les enfants », 3ᵉ personne, nombre du nom ou de son
/// déterminant), repéré via les étiquettes POS.
fn subject_and_verb(
    lex: &[(usize, &Token)],
    t: usize,
    tags: &[Tagged],
) -> Option<(Person, Number, usize)> {
    let s0 = t + 1;
    if let Some((p, n)) = lex
        .get(s0)
        .and_then(|(_, tok)| pronoun_person_number(&tok.text))
    {
        return Some((p, n, skip_clitics(lex, s0 + 1)));
    }
    // Sujet nominal : (déterminant ?) (adjectifs) nom.
    let mut k = s0;
    let mut steps = 0;
    let mut number: Option<Number> = None;
    while k < lex.len() && steps <= 3 {
        match tags[lex[k].0].upos {
            Upos::Det => {
                number = number.or_else(|| determiner_number(&lex[k].1.text));
                k += 1;
                steps += 1;
            }
            Upos::Adj => {
                k += 1;
                steps += 1;
            }
            Upos::Noun | Upos::Propn => {
                let num = number.or_else(|| noun_number(&lex[k].1.text))?;
                return Some((Person::Third, num, skip_clitics(lex, k + 1)));
            }
            _ => break,
        }
    }
    None
}

impl Rule for SubjunctiveAfterConjunction {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for t in 0..lex.len() {
                if !is_trigger(&lex, t) {
                    continue;
                }
                let Some((person, number, v)) = subject_and_verb(&lex, t, tags) else {
                    continue;
                };
                let Some(&(idx, vtok)) = lex.get(v) else {
                    continue;
                };
                if !matches!(tags[idx].upos, Upos::Verb | Upos::Aux) {
                    continue;
                }
                let Some(corrected) = subjunctive_form(&vtok.text, person, number) else {
                    continue;
                };
                suggestions.push(Suggestion {
                    span: vtok.span,
                    message: format!(
                        "Subjonctif attendu après la conjonction : « {} » devrait être « {} ».",
                        vtok.text, corrected
                    ),
                    replacements: vec![match_case(&vtok.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Subjonctif après conjonction"
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
        SubjunctiveAfterConjunction
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SubjunctiveAfterConjunction
            .check_tagged(&tokens, &tags)
            .len()
    }

    #[test]
    fn bien_que_with_etre() {
        assert_eq!(first("bien qu'il est malade").as_deref(), Some("soit"));
        // « suis » est ambigu (être/suivre) : on prend un verbe non ambigu.
        assert_eq!(first("bien que je peux partir").as_deref(), Some("puisse"));
    }

    #[test]
    fn pour_que_and_avant_que() {
        assert_eq!(first("pour que tu viens").as_deref(), Some("viennes"));
        assert_eq!(first("avant qu'il part").as_deref(), Some("parte"));
    }

    #[test]
    fn quoique_single_word() {
        assert_eq!(first("quoiqu'il peut").as_deref(), Some("puisse"));
    }

    #[test]
    fn nominal_subjects() {
        // Sujet nominal pluriel : « sont » → « soient ».
        assert_eq!(
            first("bien que les enfants sont malades").as_deref(),
            Some("soient")
        );
        // Sujet nominal singulier (déterminant possessif) : « est » → « soit ».
        assert_eq!(
            first("bien que mon ami est malade").as_deref(),
            Some("soit")
        );
    }

    #[test]
    fn already_subjunctive_is_silent() {
        for ok in [
            "bien qu'il soit malade",
            "pour que tu viennes",
            "bien qu'il mange", // « mange » : indicatif = subjonctif, déjà correct
            "avant qu'il parte",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn non_triggering_conjunction_is_silent() {
        // « parce que », « pendant que » : indicatif attendu, pas de correction.
        assert_eq!(count("parce qu'il est malade"), 0);
        assert_eq!(count("pendant qu'il mange"), 0);
        assert_eq!(count("je pense qu'il est là"), 0);
    }

    #[test]
    fn capitalization_preserved() {
        assert_eq!(first("Bien qu'il est tard").as_deref(), Some("soit"));
    }
}
