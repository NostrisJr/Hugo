//! Règle : accord sujet–verbe (personne et nombre).
//!
//! Deux familles de sujets sont traitées :
//!
//! - **pronom personnel** (`je`, `tu`, `il/elle/on`, `nous`, `vous`,
//!   `ils/elles`), qui fournit personne + nombre sans ambiguïté ;
//! - **groupe nominal** introduit par un déterminant de classe fermée
//!   (`le/la/les`, `un/une/des`, `ce/ces`, possessifs…), dont le **nombre** est
//!   porté de façon fiable par le déterminant (« les chats mange » → « mangent »).
//!
//! Principes communs, pensés pour limiter les faux positifs :
//!
//! - on saute les **clitiques préverbaux** (`ne`, `me`, `te`, `se`, `le`…) pour
//!   atteindre le verbe, et — pour les sujets nominaux — les **adjectifs**
//!   antéposés/postposés ;
//! - on ignore l'**impératif** (sans sujet) et les verbes **homographes à
//!   plusieurs lemmes** (`suis` = être/suivre) ;
//! - un groupe nominal n'est retenu comme sujet que s'il n'est pas précédé d'un
//!   **verbe conjugué** (sinon c'est vraisemblablement un complément d'objet)
//!   ni d'une **préposition** (groupe prépositionnel) ;
//! - la correction est engendrée par [`morpho::conjugate`], au même mode/temps
//!   que le verbe fautif (indicatif présent par défaut).

use std::collections::HashSet;

use super::{lexical_sentences, Rule};
use crate::morpho::{self, MorphCategory, MoodTense, Number, Person};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord en personne et en nombre entre un sujet (pronom ou groupe
/// nominal) et son verbe, sans se déclencher sur les inversions
/// (« mange-t-il », conservées comme un seul jeton par le tokenizer).
pub struct SubjectVerbAgreement;

const RULE_ID: &str = "subject_verb_agreement";

/// Nombre maximal de clitiques/adjectifs sautés entre le sujet et le verbe.
const MAX_SKIP: usize = 3;

/// Un sujet identifié et le point de départ de la recherche du verbe.
struct Subject {
    person: Person,
    number: Number,
    /// Texte affiché dans le message (pronom ou nom tête).
    label: String,
    /// Index (dans la phrase lexicale) du premier jeton après le sujet.
    verb_start: usize,
    /// Pour un sujet nominal, on saute aussi les adjectifs postposés.
    skip_adjectives: bool,
}

/// Normalise un jeton pour comparaison : minuscules, apostrophe finale ôtée.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Personne et nombre d'un pronom personnel sujet, le cas échéant.
fn subject_pronoun(text: &str) -> Option<(Person, Number)> {
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

/// Nombre porté par un déterminant de classe fermée pouvant introduire un sujet
/// (articles, démonstratifs, possessifs). Le **nombre** d'un déterminant est
/// fiable, contrairement au genre.
fn determiner_number(text: &str) -> Option<Number> {
    Some(match normalize(text).as_str() {
        "le" | "la" | "l" | "un" | "une" | "ce" | "cet" | "cette" | "mon" | "ton" | "son" | "ma"
        | "ta" | "sa" | "notre" | "votre" | "leur" => Number::Singular,
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => Number::Plural,
        _ => return None,
    })
}

/// Vrai si le jeton est un clitique préverbal (négation ou pronom objet).
fn is_clitic(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "ne" | "n" | "me" | "m" | "te" | "t" | "se" | "s" | "le" | "la" | "les" | "lui" | "leur"
            | "y" | "en"
    )
}

/// Vrai si le jeton est une préposition fréquente (garde anti groupe
/// prépositionnel devant un sujet nominal présumé).
fn is_preposition(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "à" | "a"
            | "de"
            | "d"
            | "du"
            | "dans"
            | "sur"
            | "sous"
            | "par"
            | "pour"
            | "avec"
            | "sans"
            | "vers"
            | "chez"
            | "entre"
            | "contre"
            | "depuis"
            | "pendant"
            | "selon"
            | "parmi"
            | "envers"
            | "derrière"
            | "devant"
    )
}

/// Vrai si le jeton admet une analyse verbale **finie** (forme conjuguée).
fn is_finite_verb(text: &str) -> bool {
    !morpho::verb_forms(text).is_empty()
}

/// Vrai si le jeton est analysé comme adjectif sans être par ailleurs un verbe
/// conjugué (pour ne pas sauter le verbe par mégarde).
fn is_skippable_adjective(text: &str) -> bool {
    !is_finite_verb(text)
        && morpho::lookup(text)
            .iter()
            .any(|m| m.category == MorphCategory::Adjective)
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

/// Tente d'identifier un sujet débutant à l'index `i` dans une phrase lexicale.
fn detect_subject(sentence: &[(usize, &Token)], i: usize) -> Option<Subject> {
    let text = &sentence[i].1.text;

    // --- Sujet pronominal. ---
    if let Some((person, number)) = subject_pronoun(text) {
        // Si le jeton précédent est déjà un sujet ou un clitique, ce pronom est
        // probablement complément (« je vous parle ») → on l'ignore.
        if i > 0 {
            let prev = &sentence[i - 1].1.text;
            if subject_pronoun(prev).is_some() || is_clitic(prev) {
                return None;
            }
        }
        return Some(Subject {
            person,
            number,
            label: text.clone(),
            verb_start: i + 1,
            skip_adjectives: false,
        });
    }

    // --- Sujet nominal (déterminant + [adjectifs] + nom). ---
    let number = determiner_number(text)?;

    // Gardes contextuelles sur le jeton précédent.
    if i > 0 {
        let prev = &sentence[i - 1].1.text;
        // Groupe prépositionnel (« dans les bois chante… ») : pas un sujet.
        if is_preposition(prev) {
            return None;
        }
        // Complément d'objet après un verbe (« je vois les chats ») : le groupe
        // nominal n'est pas sujet.
        if is_finite_verb(prev) {
            return None;
        }
        // Deux déterminants de suite : configuration douteuse.
        if determiner_number(prev).is_some() {
            return None;
        }
    }

    // Chercher le nom tête en sautant les adjectifs antéposés.
    let mut k = i + 1;
    let mut steps = 0;
    let head = loop {
        if k >= sentence.len() || steps > MAX_SKIP {
            break None;
        }
        let analyses = morpho::lookup(&sentence[k].1.text);
        if analyses.iter().any(|m| m.category == MorphCategory::Noun) {
            break Some(k);
        }
        if analyses
            .iter()
            .any(|m| m.category == MorphCategory::Adjective)
        {
            k += 1;
            steps += 1;
            continue;
        }
        break None;
    };
    let head = head?;

    Some(Subject {
        person: Person::Third,
        number,
        label: sentence[head].1.text.clone(),
        verb_start: head + 1,
        skip_adjectives: true,
    })
}

/// Personne (1/2/3) d'un pronom susceptible de figurer dans un sujet coordonné,
/// formes sujet **et** disjointes (« toi et moi », « lui et elle »).
fn conjunct_pronoun_person(text: &str) -> Option<u8> {
    Some(match normalize(text).as_str() {
        "je" | "j" | "moi" | "nous" => 1,
        "tu" | "toi" | "vous" => 2,
        "il" | "elle" | "on" | "ils" | "elles" | "lui" | "eux" | "soi" => 3,
        _ => return None,
    })
}

/// Vrai si le jeton ressemble à un nom propre (initiale majuscule, lettres et
/// traits d'union uniquement). Heuristique utilisée seulement en coordination.
fn is_probable_proper_noun(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_uppercase())
        && text.chars().all(|c| c.is_alphabetic() || c == '-')
        && text.chars().count() >= 2
}

/// Analyse un membre de coordination débutant à l'index `j`.
///
/// Renvoie `(personne, index_du_nom_tête, index_suivant_le_membre)`. Un membre
/// est un pronom, un groupe nominal (déterminant + adjectifs + nom), un nom
/// commun nu, ou un nom propre présumé.
fn parse_conjunct(sentence: &[(usize, &Token)], j: usize) -> Option<(u8, usize, usize)> {
    let tok = sentence[j].1;
    if let Some(p) = conjunct_pronoun_person(&tok.text) {
        return Some((p, j, j + 1));
    }
    // Groupe nominal : déterminant + adjectifs antéposés + nom tête.
    if determiner_number(&tok.text).is_some() {
        let mut k = j + 1;
        let mut steps = 0;
        loop {
            if k >= sentence.len() || steps > MAX_SKIP {
                return None;
            }
            let analyses = morpho::lookup(&sentence[k].1.text);
            if analyses.iter().any(|m| m.category == MorphCategory::Noun) {
                return Some((3, k, k + 1));
            }
            if analyses
                .iter()
                .any(|m| m.category == MorphCategory::Adjective)
            {
                k += 1;
                steps += 1;
                continue;
            }
            return None;
        }
    }
    // Nom commun nu.
    if morpho::lookup(&tok.text)
        .iter()
        .any(|m| m.category == MorphCategory::Noun)
    {
        return Some((3, j, j + 1));
    }
    // Nom propre présumé (capitalisation).
    if is_probable_proper_noun(&tok.text) {
        return Some((3, j, j + 1));
    }
    None
}

/// Détecte un sujet coordonné (« A et B [et C…] ») débutant à l'index `i`.
///
/// Renvoie `(personne, index_de_début_du_verbe, libellé)`. La personne est la
/// plus prioritaire des membres (1 > 2 > 3), le nombre étant toujours pluriel.
fn detect_coordinated(sentence: &[(usize, &Token)], i: usize) -> Option<(Person, usize, String)> {
    let (mut person, mut head, mut end) = parse_conjunct(sentence, i)?;
    let mut labels = vec![sentence[head].1.text.clone()];
    while end < sentence.len() && normalize(&sentence[end].1.text) == "et" {
        let Some((p, h, e)) = parse_conjunct(sentence, end + 1) else {
            break;
        };
        person = person.min(p);
        head = h;
        end = e;
        labels.push(sentence[head].1.text.clone());
        if labels.len() >= 6 {
            break;
        }
    }
    if labels.len() < 2 {
        return None;
    }
    let person = match person {
        1 => Person::First,
        2 => Person::Second,
        _ => Person::Third,
    };
    Some((person, end, labels.join(" et ")))
}

/// Index du jeton candidat verbe à partir de `verb_start`, en sautant les
/// clitiques (et les adjectifs si `skip_adjectives`).
fn find_verb(sentence: &[(usize, &Token)], verb_start: usize, skip_adjectives: bool) -> Option<usize> {
    let mut k = verb_start;
    let mut steps = 0;
    while k < sentence.len() && steps < MAX_SKIP {
        let t = &sentence[k].1.text;
        if is_clitic(t) || (skip_adjectives && is_skippable_adjective(t)) {
            k += 1;
            steps += 1;
        } else {
            break;
        }
    }
    (k < sentence.len()).then_some(k)
}

/// Si le verbe au jeton `k` n'accorde pas avec `(person, number)`, renvoie la
/// suggestion de correction.
fn agree_at(
    sentence: &[(usize, &Token)],
    k: usize,
    person: Person,
    number: Number,
    label: &str,
) -> Option<Suggestion> {
    let verb_token = sentence.get(k)?.1;

    // Formes verbales finies, hors impératif (qui n'a pas de sujet).
    let finite: Vec<_> = morpho::verb_forms(&verb_token.text)
        .into_iter()
        .filter(|v| v.mood_tense != MoodTense::Imperative)
        .collect();
    if finite.is_empty() {
        return None;
    }
    // Déjà accordé ?
    if finite
        .iter()
        .any(|v| v.person == person && v.number == number)
    {
        return None;
    }
    // Lemme unique seulement (sinon homographe ambigu : suis, vis…).
    let lemmas: HashSet<&str> = finite.iter().map(|v| v.lemma.as_str()).collect();
    if lemmas.len() != 1 {
        return None;
    }
    let lemma = finite[0].lemma.clone();

    // Cibler le même mode/temps que le verbe fautif (indicatif présent par
    // défaut s'il fait partie des analyses).
    let target_mt = if finite
        .iter()
        .any(|v| v.mood_tense == MoodTense::IndicativePresent)
    {
        MoodTense::IndicativePresent
    } else {
        finite[0].mood_tense
    };

    let corrected = morpho::conjugate(&lemma, target_mt, person, number)?;
    if corrected.eq_ignore_ascii_case(&verb_token.text) {
        return None;
    }

    Some(Suggestion {
        span: verb_token.span,
        message: format!(
            "Accord sujet–verbe : « {} » ne s'accorde pas avec le sujet « {} ».",
            verb_token.text, label
        ),
        replacements: vec![match_case(&verb_token.text, &corrected)],
        rule_id: RULE_ID,
    })
}

impl SubjectVerbAgreement {
    fn check_sentence(sentence: &[(usize, &Token)], out: &mut Vec<Suggestion>) {
        let mut local = Vec::new();
        // Verbes déjà pris en charge par un sujet coordonné : la détection
        // simple doit les ignorer, sans quoi un membre singulier (« le chien »
        // dans « le chat et le chien dorment ») tenterait de ramener au
        // singulier un verbe correctement pluriel.
        let mut claimed: HashSet<usize> = HashSet::new();

        // Sujets coordonnés d'abord : ils fixent la personne (nous/vous) que la
        // détection simple ne saurait deviner.
        for i in 0..sentence.len() {
            if let Some((person, verb_start, label)) = detect_coordinated(sentence, i) {
                if let Some(k) = find_verb(sentence, verb_start, true) {
                    claimed.insert(k);
                    if let Some(s) = agree_at(sentence, k, person, Number::Plural, &label) {
                        local.push(s);
                    }
                }
            }
        }

        // Sujets simples (pronom ou groupe nominal).
        for i in 0..sentence.len() {
            if let Some(subject) = detect_subject(sentence, i) {
                if let Some(k) = find_verb(sentence, subject.verb_start, subject.skip_adjectives) {
                    if claimed.contains(&k) {
                        continue;
                    }
                    if let Some(s) =
                        agree_at(sentence, k, subject.person, subject.number, &subject.label)
                    {
                        local.push(s);
                    }
                }
            }
        }

        // Dédoublonnage par verbe (un verbe n'a qu'un accord).
        let mut seen = HashSet::new();
        for s in local {
            if seen.insert(s.span.start) {
                out.push(s);
            }
        }
    }
}

impl Rule for SubjectVerbAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            Self::check_sentence(&sentence, &mut suggestions);
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord sujet–verbe"
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
        SubjectVerbAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        SubjectVerbAgreement.check(&tokenize(text)).len()
    }

    // --- Sujets pronominaux. ---

    #[test]
    fn plural_pronoun_singular_verb() {
        assert_eq!(first("ils mange").as_deref(), Some("mangent"));
    }

    #[test]
    fn second_singular() {
        // « tu mange » → « manges » (et non l'impératif « mange »).
        assert_eq!(first("tu mange").as_deref(), Some("manges"));
    }

    #[test]
    fn correct_agreement_yields_nothing() {
        for ok in [
            "ils mangent",
            "il mange",
            "je mange",
            "nous mangeons",
            "vous mangez",
            "tu manges",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn skips_clitics() {
        assert_eq!(first("ils ne mange pas").as_deref(), Some("mangent"));
        assert_eq!(count("je le mange"), 0);
    }

    #[test]
    fn object_pronoun_not_treated_as_subject() {
        assert_eq!(count("je vous parle"), 0);
    }

    #[test]
    fn inversion_is_ignored() {
        assert_eq!(count("mange-t-il"), 0);
    }

    #[test]
    fn correction_matches_verb_case() {
        assert_eq!(first("Ils mange").as_deref(), Some("mangent"));
    }

    // --- Sujets nominaux. ---

    #[test]
    fn nominal_plural_subject() {
        // « les chats mange » → « mangent ».
        assert_eq!(first("les chats mange").as_deref(), Some("mangent"));
    }

    #[test]
    fn nominal_plural_with_postnominal_adjective() {
        // « les chats noirs mange » → « mangent » (adjectif postposé sauté).
        assert_eq!(first("les chats noirs mange").as_deref(), Some("mangent"));
    }

    #[test]
    fn nominal_possessive_subject() {
        assert_eq!(first("mes amis arrive").as_deref(), Some("arrivent"));
    }

    #[test]
    fn nominal_correct_agreement_is_silent() {
        for ok in ["les chats mangent", "le chat mange", "des chats dorment"] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn object_noun_phrase_after_verb_is_ignored() {
        // « je vois les chats » : « les chats » est complément d'objet.
        assert_eq!(count("je vois les chats"), 0);
    }

    #[test]
    fn prepositional_phrase_is_not_a_subject() {
        // « dans les bois » est un groupe prépositionnel, pas un sujet.
        assert_eq!(count("dans les bois chante un oiseau"), 0);
    }

    #[test]
    fn no_cross_sentence_leak() {
        // « dort » (phrase 1) ne doit pas être vu comme voisin de « Les ».
        assert_eq!(
            first("Il dort. Les chats mange.").as_deref(),
            Some("mangent")
        );
    }

    // --- Sujets coordonnés. ---

    #[test]
    fn coordinated_proper_nouns() {
        // « Pierre et Marie mange » → « mangent » (3ᵉ pluriel).
        assert_eq!(first("Pierre et Marie mange").as_deref(), Some("mangent"));
    }

    #[test]
    fn coordinated_singular_nouns_become_plural() {
        // Deux singuliers coordonnés → verbe pluriel.
        assert_eq!(
            first("le chat et le chien mange").as_deref(),
            Some("mangent")
        );
    }

    #[test]
    fn coordinated_person_priority_first() {
        // « toi et moi » → 1re du pluriel → « sommes ».
        assert_eq!(first("toi et moi est là").as_deref(), Some("sommes"));
    }

    #[test]
    fn coordinated_person_priority_second() {
        // « toi et Pierre » → 2e du pluriel → « mangez ».
        assert_eq!(first("toi et Pierre mange").as_deref(), Some("mangez"));
    }

    #[test]
    fn coordinated_correct_agreement_is_silent() {
        for ok in [
            "Pierre et Marie mangent",
            "le chat et le chien dorment",
            "toi et moi sommes là",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn clause_coordination_is_not_a_subject() {
        // « Jean dort et Marie mange » : « et » coordonne deux propositions,
        // pas deux sujets → aucune correction (chaque verbe est au singulier).
        assert_eq!(count("Jean dort et Marie mange"), 0);
    }

    #[test]
    fn no_duplicate_for_plural_conjuncts() {
        // Les deux membres pluriels ne doivent pas produire deux suggestions.
        assert_eq!(count("les chats et les chiens mange"), 1);
    }
}
