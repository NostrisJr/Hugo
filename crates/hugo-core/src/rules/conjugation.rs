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
//!
//! Quand les étiquettes POS du CRF sont disponibles ([`Rule::check_tagged`]),
//! des gardes plus fines remplacent les heuristiques morphologiques (le chemin
//! [`Rule::check`] reste le repli) : le **verbe candidat** doit être étiqueté
//! `VERB`/`AUX` (un nom homographe — « des points/barres » — n'est plus pris
//! pour un verbe) ; un groupe nominal précédé d'un token `VERB`/`AUX` est un
//! complément (y compris derrière un **infinitif**, que `is_finite_verb` ne
//! voit pas) ; « des »/« du » après un nom est un complément en *de + article*.
//!
//! **Repli par suffixe 1ᵉʳ groupe.** Quand le Lefff ne connaît pas la forme
//! conjuguée d'un verbe (ex. « implémentes », « conjugueriez » — absents de la
//! table des formes mais présents en tant qu'infinitif), le CRF étiquette
//! correctement `VERB`. Dans ce cas, `agree_at` tente de retrouver l'infinitif
//! et le mode/temps par analyse des **suffixes réguliers** des verbes du 1ᵉʳ
//! groupe (-er). Le guard `morpho::lookup(infinitif) → VERB` évite les faux
//! infinitifs produits par dépréfixation sur un verbe du 2ᵉ ou 3ᵉ groupe.
//! pas un sujet ; un pronom après le relatif « qui » est un objet.

use std::collections::HashSet;

use super::{lexical_sentences, Rule};
use crate::dep::{self, DepRel};
use crate::morpho::{self, MoodTense, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
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

/// Personne et nombre d'un pronom personnel ou indéfini sujet.
fn subject_pronoun(text: &str) -> Option<(Person, Number)> {
    Some(match normalize(text).as_str() {
        "je" | "j" => (Person::First, Number::Singular),
        "tu" => (Person::Second, Number::Singular),
        "il" | "elle" | "on" => (Person::Third, Number::Singular),
        "nous" => (Person::First, Number::Plural),
        "vous" => (Person::Second, Number::Plural),
        "ils" | "elles" => (Person::Third, Number::Plural),
        // Pronoms indéfinis toujours singuliers (accord 3e pers. sg.).
        "chacun" | "chacune" | "nul" | "nulle" | "personne" | "quiconque" => {
            (Person::Third, Number::Singular)
        }
        _ => return None,
    })
}

/// Nombre porté par un déterminant de classe fermée pouvant introduire un sujet
/// (articles, démonstratifs, possessifs). Le **nombre** d'un déterminant est
/// fiable, contrairement au genre.
fn determiner_number(text: &str) -> Option<Number> {
    Some(match normalize(text).as_str() {
        "le" | "la" | "l" | "un" | "une" | "ce" | "cet" | "cette" | "mon" | "ton" | "son"
        | "ma" | "ta" | "sa" | "notre" | "votre" | "leur" => Number::Singular,
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => Number::Plural,
        _ => return None,
    })
}

/// Vrai si le jeton est un clitique préverbal (négation ou pronom objet).
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

/// Vrai si le jeton à l'index lexical `idx` **occupe la position sujet** : un
/// nom/nom propre, ou un pronom démonstratif/indéfini sujet (`cela`, `ça`,
/// `ceci`, `ce`, `on`). Sert à reconnaître qu'un « vous »/« nous » suivant est en
/// réalité un **clitique objet** (« cela vous suffit », « l'agence vous
/// assure ») et non le sujet.
fn fills_subject_slot(text: &str, idx: usize, tags: Option<&[Tagged]>) -> bool {
    if matches!(
        normalize(text).as_str(),
        "cela" | "ça" | "ca" | "ceci" | "ce" | "on"
    ) {
        return true;
    }
    match tags {
        Some(tags) => matches!(tags[idx].upos, Upos::Noun | Upos::Propn),
        None => morpho::lookup(text)
            .iter()
            .any(|m| m.category == MorphCategory::Noun),
    }
}

/// Vrai si le jeton d'index `k+1` admet une analyse nominale. Sert à décider
/// qu'un mot nom/adjectif ambigu (« petit », « principales »…) est en réalité un
/// adjectif antéposé lorsqu'un nom le suit.
fn next_is_noun(sentence: &[(usize, &Token)], k: usize) -> bool {
    sentence.get(k + 1).is_some_and(|(_, t)| {
        morpho::lookup(&t.text)
            .iter()
            .any(|m| m.category == MorphCategory::Noun)
    })
}

/// Cherche le nom tête à partir de l'index `start`, en sautant les adjectifs
/// antéposés (y compris les homographes nom/adjectif suivis d'un autre nom)
/// **et les numéraux** (« les deux vigies », « les trois enfants »).
/// Les numéraux sont identifiés par l'étiquette POS `NUM` du CRF (quand
/// disponible) ou par une liste de cardinaux courants.
/// Renvoie l'index du nom, ou `None`.
fn noun_head_from(
    sentence: &[(usize, &Token)],
    start: usize,
    tags: Option<&[Tagged]>,
) -> Option<usize> {
    let mut k = start;
    let mut steps = 0;
    // Dernière position d'un numéral vu : si aucun nom ne suit, le numéral
    // lui-même est la tête (« les deux regardaient » → tête = « deux »,
    // « les 2 regardaient » → tête = « 2 »).
    let mut last_num: Option<usize> = None;
    loop {
        if k >= sentence.len() || steps > MAX_SKIP {
            return last_num;
        }
        let (tok_idx, tok) = sentence[k];
        let analyses = morpho::lookup(&tok.text);
        let has_noun = analyses.iter().any(|m| m.category == MorphCategory::Noun);
        let has_adj = analyses
            .iter()
            .any(|m| m.category == MorphCategory::Adjective);
        let is_num = tags.map_or(false, |tags| tags[tok_idx].upos == Upos::Num)
            || is_cardinal_numeral(&tok.text)
            || tok.kind == crate::tokenizer::TokenKind::Number;
        if has_noun {
            if has_adj && next_is_noun(sentence, k) {
                k += 1;
                steps += 1;
                continue;
            }
            return Some(k);
        }
        if has_adj || is_num {
            if is_num {
                last_num = Some(k);
            }
            k += 1;
            steps += 1;
            continue;
        }
        // Token non-sautables (verbe, adverbe…) : si un numéral a été vu
        // avant, il est la tête du GN (ex. « les deux regardaient »).
        return last_num;
    }
}

/// Cardinaux simples dont le Lefff ne retourne pas toujours une analyse
/// adjectivale — on les saute comme les adjectifs antéposés.
fn is_cardinal_numeral(text: &str) -> bool {
    matches!(
        text.to_lowercase().as_str(),
        "zéro"
            | "un"
            | "une"
            | "deux"
            | "trois"
            | "quatre"
            | "cinq"
            | "six"
            | "sept"
            | "huit"
            | "neuf"
            | "dix"
            | "onze"
            | "douze"
            | "treize"
            | "quatorze"
            | "quinze"
            | "seize"
            | "vingt"
            | "trente"
            | "quarante"
            | "cinquante"
            | "soixante"
            | "cent"
            | "mille"
    )
}

/// Vrai si le jeton est analysé comme adjectif sans être par ailleurs un verbe
/// conjugué (pour ne pas sauter le verbe par mégarde).
/// Guard : un pronom sujet comme « tu » n'est jamais sauté, même s'il a une
/// lecture participiale-adjectivale (ex. « tu » = participe passé de *taire*).
fn is_skippable_adjective(text: &str) -> bool {
    subject_pronoun(text).is_none()
        && !is_finite_verb(text)
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
///
/// `tags` (le cas échéant) porte les étiquettes POS du CRF, **alignées sur les
/// tokens d'origine** (indexées par `sentence[..].0`). Quand elles sont
/// fournies, des gardes plus fines remplacent les heuristiques morphologiques :
/// objet de relatif, COD d'infinitif, complément en de+les.
fn detect_subject(
    sentence: &[(usize, &Token)],
    i: usize,
    tags: Option<&[Tagged]>,
) -> Option<Subject> {
    let text = &sentence[i].1.text;

    // --- Sujet pronominal. ---
    if let Some((person, number)) = subject_pronoun(text) {
        // Si le jeton précédent est déjà un sujet ou un clitique, ce pronom est
        // probablement complément (« je vous parle ») → on l'ignore.
        if i > 0 {
            let (prev_idx, prev_tok) = sentence[i - 1];
            let prev = &prev_tok.text;
            if subject_pronoun(prev).is_some() || is_clitic(prev) {
                return None;
            }
            // Relatif sujet « qui » : le pronom qui suit est un objet, pas le
            // sujet (« le problème qui nous a été posé » → « nous » n'est pas le
            // sujet ; le verbe s'accorde avec l'antécédent). On s'abstient.
            if tags.is_some() && normalize(prev) == "qui" {
                return None;
            }
            // « vous »/« nous » sont aussi des **clitiques objets** : précédés
            // d'un remplisseur de la position sujet (nom, nom propre, « cela »…),
            // ils ne sont pas le sujet (« cela vous suffit », « l'agence vous
            // assure »). Les autres pronoms (`il`, `je`…) ne sont jamais objets.
            if matches!(normalize(text).as_str(), "vous" | "nous")
                && fills_subject_slot(prev, prev_idx, tags)
            {
                return None;
            }
        }
        // Pour les indéfinis de type « chacun de NP » : sauter le complément
        // partitif (de/des + [adj*] nom) pour atteindre le verbe. On utilise
        // les tags CRF quand disponibles ; sans tags, on reste au token suivant.
        const INDEF_WITH_PARTITIVE: &[&str] =
            &["chacun", "chacune", "nul", "nulle", "personne", "quiconque"];
        let verb_start = if INDEF_WITH_PARTITIVE.contains(&normalize(text).as_str()) {
            if let Some(tags) = tags {
                // Chercher le premier VERB/AUX après la position courante.
                (i + 1..sentence.len())
                    .find(|&k| {
                        matches!(tags[sentence[k].0].upos, Upos::Verb | Upos::Aux)
                    })
                    .unwrap_or(i + 1)
            } else {
                i + 1
            }
        } else {
            i + 1
        };
        return Some(Subject {
            person,
            number,
            label: text.clone(),
            verb_start,
            skip_adjectives: false,
        });
    }

    // --- Sujet nom propre nu (sans déterminant, étiquette Propn par le CRF). ---
    // « Marc puisses » → Marc est 3e pers. sg. Garde : étiquette obligatoire
    // (évite de confondre un nom commun en début de phrase avec un nom propre).
    if let Some(tags) = tags {
        if tags[sentence[i].0].upos == crate::pos::Upos::Propn {
            // Ne pas prendre un nom propre objet (précédé d'un clitique ou d'une prépo).
            let blocked = i > 0 && {
                let prev = &sentence[i - 1].1.text;
                is_clitic(prev) || is_preposition(prev)
            };
            if !blocked {
                return Some(Subject {
                    person: Person::Third,
                    number: Number::Singular,
                    label: text.clone(),
                    verb_start: i + 1,
                    skip_adjectives: false,
                });
            }
        }
    }

    // --- Sujet nominal (déterminant + [adjectifs] + nom). ---
    let number = determiner_number(text)?;

    // Gardes contextuelles sur le jeton précédent.
    if i > 0 {
        let (prev_idx, prev_tok) = sentence[i - 1];
        let prev = &prev_tok.text;
        // Groupe prépositionnel (« dans les bois chante… ») : pas un sujet.
        if is_preposition(prev) {
            return None;
        }
        match tags {
            Some(tags) => {
                // Complément d'objet après un verbe, **y compris un infinitif**
                // (« redéployer des postes ») que `is_finite_verb` ne détecte
                // pas : c'est l'étiquette POS qui tranche.
                if matches!(tags[prev_idx].upos, Upos::Verb | Upos::Aux) {
                    return None;
                }
                // « des »/« du » = « de » + article : précédés d'un nom, ils
                // introduisent un complément (« la démultiplication des usages »),
                // pas un nouveau sujet.
                if matches!(normalize(text).as_str(), "des" | "du")
                    && tags[prev_idx].upos == Upos::Noun
                {
                    return None;
                }
            }
            None => {
                // Complément d'objet après un verbe conjugué (« je vois les
                // chats ») : le groupe nominal n'est pas sujet.
                if is_finite_verb(prev) {
                    return None;
                }
            }
        }
        // Deux déterminants de suite : configuration douteuse.
        if determiner_number(prev).is_some() {
            return None;
        }
    }

    // Chercher le nom tête en sautant les adjectifs et numéraux antéposés.
    let head = noun_head_from(sentence, i + 1, tags)?;

    Some(Subject {
        person: Person::Third,
        number,
        label: sentence[head].1.text.clone(),
        verb_start: head + 1,
        skip_adjectives: true,
    })
}

/// Vrai si le verbe candidat à l'index `k` est compatible avec les tags POS :
/// sans tags, on accepte (chemin morphologique) ; avec tags, on exige que le
/// jeton soit étiqueté `VERB`/`AUX`, ce qui écarte les **noms homographes**
/// (« des points/barres » : `barres` est étiqueté `NOUN`).
fn verb_candidate_ok(sentence: &[(usize, &Token)], k: usize, tags: Option<&[Tagged]>) -> bool {
    match tags {
        None => true,
        Some(tags) => matches!(tags[sentence[k].0].upos, Upos::Verb | Upos::Aux),
    }
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
    if j >= sentence.len() {
        return None;
    }
    let tok = sentence[j].1;
    if let Some(p) = conjunct_pronoun_person(&tok.text) {
        return Some((p, j, j + 1));
    }
    // Groupe nominal : déterminant + adjectifs antéposés + nom tête.
    if determiner_number(&tok.text).is_some() {
        return noun_head_from(sentence, j + 1, None).map(|h| (3, h, h + 1));
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
    // Une coordination introduite par une **préposition** est un complément, pas
    // un sujet (« …producteurs de Saverne et environs organise » : « Saverne et
    // environs » complète « producteurs », ce n'est pas le sujet de « organise »).
    if i > 0 && is_preposition(&sentence[i - 1].1.text) {
        return None;
    }
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
fn find_verb(
    sentence: &[(usize, &Token)],
    verb_start: usize,
    skip_adjectives: bool,
) -> Option<usize> {
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

/// Retrouve l'infinitif et le mode/temps probable d'un verbe du 1ᵉʳ groupe
/// (-er) à partir de sa forme conjuguée, par analyse des suffixes réguliers.
///
/// Retourne `(infinitif, mode_temps)` ou `None` si la forme ne correspond à
/// aucun patron -er reconnu.
///
/// Ambiguïté conditionnel/imparfait : après suppression d'un suffixe commun
/// (-aient, -iez, -ions, -ait, -ais), si le reste se termine en « er » c'est
/// le conditionnel (infinitif déjà reconstitué) ; sinon c'est l'imparfait
/// (ajouter « er » pour former l'infinitif).
fn guess_er_infinitive(form: &str) -> Option<(String, MoodTense)> {
    let low = form.to_lowercase();

    // Suffixes dont le reste indique conditionnel (se termine en "er") ou
    // imparfait (ajouter "er") selon le résidu. Vérifiés avant les suffixes
    // purement imparfaits/présent pour éviter les ambiguïtés.
    for suffix in ["aient", "iez", "ions", "ait", "ais"] {
        if let Some(rem) = low.strip_suffix(suffix) {
            let (inf, mt) = if rem.ends_with("er") {
                (rem.to_string(), MoodTense::ConditionalPresent)
            } else {
                (format!("{}er", rem), MoodTense::IndicativeImperfect)
            };
            return Some((inf, mt));
        }
    }
    // Futur simple (infinitif + terminaison ; toujours conditionnel).
    for suffix in ["eront", "erez", "erons", "eras", "erai", "era"] {
        if let Some(rem) = low.strip_suffix(suffix) {
            return Some((format!("{}er", rem), MoodTense::IndicativeFuture));
        }
    }
    // Présent de l'indicatif (radical + terminaison).
    for suffix in ["ent", "ons", "ez"] {
        if let Some(rem) = low.strip_suffix(suffix) {
            return Some((format!("{}er", rem), MoodTense::IndicativePresent));
        }
    }
    // "-es" (2sg) : strip "es" → radical → ajouter "er" = infinitif.
    if let Some(rem) = low.strip_suffix("es") {
        return Some((format!("{}er", rem), MoodTense::IndicativePresent));
    }
    None
}

/// Génère une forme régulière du 1ᵉʳ groupe (-er) pour un mode/temps, une
/// personne et un nombre donnés.
///
/// Repli quand `morpho::conjugate` échoue sur un verbe absent du Lefff. Ne
/// gère pas les variations orthographiques (-ger → -geons, -cer → -çons) :
/// les résultats peuvent être approximatifs pour ces sous-groupes.
fn generate_er_form(infinitive: &str, mt: MoodTense, person: Person, number: Number) -> Option<String> {
    // Le radical est l'infinitif sans « er ».
    let stem = infinitive.strip_suffix("er")?;
    Some(match (mt, person, number) {
        // Présent de l'indicatif
        (MoodTense::IndicativePresent, Person::First | Person::Third, Number::Singular) => {
            format!("{}e", stem)
        }
        (MoodTense::IndicativePresent, Person::Second, Number::Singular) => {
            format!("{}es", stem)
        }
        (MoodTense::IndicativePresent, Person::First, Number::Plural) => {
            format!("{}ons", stem)
        }
        (MoodTense::IndicativePresent, Person::Second, Number::Plural) => {
            format!("{}ez", stem)
        }
        (MoodTense::IndicativePresent, Person::Third, Number::Plural) => {
            format!("{}ent", stem)
        }
        // Imparfait de l'indicatif
        (MoodTense::IndicativeImperfect, Person::First | Person::Second, Number::Singular) => {
            format!("{}ais", stem)
        }
        (MoodTense::IndicativeImperfect, Person::Third, Number::Singular) => {
            format!("{}ait", stem)
        }
        (MoodTense::IndicativeImperfect, Person::First, Number::Plural) => {
            format!("{}ions", stem)
        }
        (MoodTense::IndicativeImperfect, Person::Second, Number::Plural) => {
            format!("{}iez", stem)
        }
        (MoodTense::IndicativeImperfect, Person::Third, Number::Plural) => {
            format!("{}aient", stem)
        }
        // Conditionnel présent (infinitif + terminaison)
        (MoodTense::ConditionalPresent, Person::First | Person::Second, Number::Singular) => {
            format!("{}erais", stem)
        }
        (MoodTense::ConditionalPresent, Person::Third, Number::Singular) => {
            format!("{}erait", stem)
        }
        (MoodTense::ConditionalPresent, Person::First, Number::Plural) => {
            format!("{}erions", stem)
        }
        (MoodTense::ConditionalPresent, Person::Second, Number::Plural) => {
            format!("{}eriez", stem)
        }
        (MoodTense::ConditionalPresent, Person::Third, Number::Plural) => {
            format!("{}eraient", stem)
        }
        // Futur simple (infinitif + terminaison)
        (MoodTense::IndicativeFuture, Person::First, Number::Singular) => {
            format!("{}erai", stem)
        }
        (MoodTense::IndicativeFuture, Person::Second, Number::Singular) => {
            format!("{}eras", stem)
        }
        (MoodTense::IndicativeFuture, Person::Third, Number::Singular) => {
            format!("{}era", stem)
        }
        (MoodTense::IndicativeFuture, Person::First, Number::Plural) => {
            format!("{}erons", stem)
        }
        (MoodTense::IndicativeFuture, Person::Second, Number::Plural) => {
            format!("{}erez", stem)
        }
        (MoodTense::IndicativeFuture, Person::Third, Number::Plural) => {
            format!("{}eront", stem)
        }
        _ => return None,
    })
}

/// Si le verbe au jeton `k` n'accorde pas avec `(person, number)`, renvoie la
/// suggestion de correction.
///
/// `tags` est la tranche des étiquettes POS (alignée sur les tokens d'origine).
/// Quand elle est fournie, un repli par suffixe du 1ᵉʳ groupe est tenté pour
/// les verbes absents du Lefff mais étiquetés `VERB`/`AUX` par le CRF.
/// Token lexical d'index d'origine `orig` dans la phrase, le cas échéant.
fn token_by_orig<'a>(sentence: &'a [(usize, &'a Token)], orig: usize) -> Option<&'a Token> {
    sentence.iter().find(|(o, _)| *o == orig).map(|(_, t)| *t)
}

/// Nombre d'un nom sujet (index d'origine `orig`) : consensus des lectures
/// nominales du lexique, sinon nombre d'un déterminant **enfant** dans l'arbre.
fn noun_number_tree(sentence: &[(usize, &Token)], tags: &[Tagged], orig: usize) -> Option<Number> {
    let tok = token_by_orig(sentence, orig)?;
    let mut num: Option<Number> = None;
    for m in morpho::lookup(&tok.text)
        .iter()
        .filter(|m| m.category == MorphCategory::Noun)
    {
        if let Some(n) = m.number {
            if n == Number::Invariable {
                continue;
            }
            match num {
                None => num = Some(n),
                Some(p) if p == n => {}
                Some(_) => return None, // contradiction
            }
        }
    }
    if num.is_some() {
        return num;
    }
    for c in dep::children(tags, orig) {
        if tags[c].dep == DepRel::Det {
            if let Some(t) = token_by_orig(sentence, c) {
                if let Some(n) = determiner_number(&t.text) {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Personne et nombre d'un token **sujet** (index d'origine `orig`) : pronom du
/// jeu fermé, ou nom/nom propre (3ᵉ personne + nombre du nom).
fn subject_pn(sentence: &[(usize, &Token)], tags: &[Tagged], orig: usize) -> Option<(Person, Number)> {
    let tok = token_by_orig(sentence, orig)?;
    if let Some(pn) = subject_pronoun(&tok.text) {
        return Some(pn);
    }
    if matches!(tags[orig].upos, Upos::Noun | Upos::Propn) {
        return Some((Person::Third, noun_number_tree(sentence, tags, orig)?));
    }
    None
}

/// Index d'origine du **sujet** du verbe `v` selon l'arbre de dépendances.
///
/// Cherche un enfant `nsubj`/`nsubj:pass`/`csubj` de `v`. Si `v` est une copule
/// ou un auxiliaire (le sujet est alors porté par le **prédicat**, tête de `v`),
/// on cherche le sujet sur cette tête (« la salle **était** pleine » : le sujet
/// `salle` est `nsubj` de `pleine`, pas de la copule `était »).
fn tree_subject_idx(tags: &[Tagged], v: usize) -> Option<usize> {
    if let Some(s) = dep::subject_of(tags, v) {
        return Some(s);
    }
    if matches!(tags[v].dep, DepRel::Cop | DepRel::Aux | DepRel::AuxPass) {
        let head = dep::head_of(tags, v)?;
        return dep::subject_of(tags, head);
    }
    None
}

/// Veto par l'arbre : vrai si le verbe d'index d'origine `v` **s'accorde déjà**
/// avec son vrai sujet (le `nsubj` de l'arbre). Dans ce cas, une suggestion
/// d'accord serait un faux positif — la détection positionnelle a confondu le
/// sujet avec un objet, un complément ou un membre d'apposition.
///
/// Renvoie `false` si le sujet ou son nombre est indéterminé (pas de veto) :
/// double garde — on n'abstient que si l'arbre **confirme** l'accord.
fn verb_agrees_with_tree_subject(sentence: &[(usize, &Token)], tags: &[Tagged], v: usize, verb_text: &str) -> bool {
    let Some(s_idx) = tree_subject_idx(tags, v) else {
        return false;
    };
    // Sujet coordonné (« Pierre et Marie ») : l'arbre ne pointe qu'un conjoint
    // (singulier), mais le sujet réel est pluriel. On ne vote pas — la détection
    // de sujet coordonné (`detect_coordinated`) doit pouvoir agir.
    if dep::children(tags, s_idx)
        .iter()
        .any(|&c| tags[c].dep == DepRel::Conj)
    {
        return false;
    }
    let Some((sp, sn)) = subject_pn(sentence, tags, s_idx) else {
        return false;
    };
    morpho::verb_forms(verb_text)
        .iter()
        .any(|vf| vf.mood_tense != MoodTense::Imperative && vf.person == sp && vf.number == sn)
}

fn agree_at(
    sentence: &[(usize, &Token)],
    k: usize,
    person: Person,
    number: Number,
    label: &str,
    tags: Option<&[Tagged]>,
) -> Option<Suggestion> {
    let verb_token = sentence.get(k)?.1;

    // Sujet disjonctif « ni … ni … » : le verbe peut légitimement être au
    // singulier OU au pluriel (« ni l'un ni l'autre ne sont/n'est venu »). On
    // s'abstient dès qu'au moins deux « ni » précèdent le verbe.
    if sentence[..k]
        .iter()
        .filter(|(_, t)| normalize(&t.text) == "ni")
        .count()
        >= 2
    {
        return None;
    }

    // Veto par l'arbre de dépendances : si le verbe s'accorde déjà avec son
    // **vrai** sujet (nsubj de l'arbre), la détection positionnelle a visé le
    // mauvais sujet → on s'abstient (réduction des faux positifs sur inversions,
    // appositions et compléments).
    if let Some(tags) = tags {
        let v = sentence[k].0;
        if verb_agrees_with_tree_subject(sentence, tags, v, &verb_token.text) {
            return None;
        }
    }

    // Chemin Lefff : formes verbales finies, hors impératif.
    let finite: Vec<_> = morpho::verb_forms(&verb_token.text)
        .into_iter()
        .filter(|v| v.mood_tense != MoodTense::Imperative)
        .collect();

    if !finite.is_empty() {
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
        return Some(Suggestion {
            span: verb_token.span,
            message: format!(
                "Accord sujet–verbe : « {} » ne s'accorde pas avec le sujet « {} ».",
                verb_token.text, label
            ),
            replacements: vec![match_case(&verb_token.text, &corrected)],
            rule_id: RULE_ID,
        });
    }

    // Repli par suffixe 1ᵉʳ groupe : le Lefff ne connaît pas cette forme
    // conjuguée, mais le CRF l'étiquette VERB/AUX.
    let tags = tags?;
    let tok_orig_idx = sentence[k].0;
    if !matches!(tags[tok_orig_idx].upos, Upos::Verb | Upos::Aux) {
        return None;
    }
    let (infinitive, mt) = guess_er_infinitive(&verb_token.text)?;
    // Guard : l'infinitif reconstitué doit être un verbe connu dans le Lefff
    // (évite les faux infinitifs produits sur des verbes du 2ᵉ/3ᵉ groupe).
    if !morpho::lookup(&infinitive)
        .iter()
        .any(|m| m.category == MorphCategory::Verb)
    {
        return None;
    }
    // Générer la forme correcte pour ce sujet.
    let corrected = morpho::conjugate(&infinitive, mt, person, number)
        .or_else(|| generate_er_form(&infinitive, mt, person, number))?;
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
    fn check_sentence(
        sentence: &[(usize, &Token)],
        tags: Option<&[Tagged]>,
        out: &mut Vec<Suggestion>,
    ) {
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
                    if !verb_candidate_ok(sentence, k, tags) {
                        continue;
                    }
                    claimed.insert(k);
                    if let Some(s) = agree_at(sentence, k, person, Number::Plural, &label, tags) {
                        local.push(s);
                    }
                }
            }
        }

        // Sujets simples (pronom ou groupe nominal).
        for i in 0..sentence.len() {
            if let Some(subject) = detect_subject(sentence, i, tags) {
                if let Some(k) = find_verb(sentence, subject.verb_start, subject.skip_adjectives) {
                    if claimed.contains(&k) {
                        continue;
                    }
                    if !verb_candidate_ok(sentence, k, tags) {
                        continue;
                    }
                    if let Some(s) =
                        agree_at(sentence, k, subject.person, subject.number, &subject.label, tags)
                    {
                        local.push(s);
                    }
                }
            }
        }

        // NB : un « override » par l'arbre (3ᵉ passe flaggant tout verbe dont le
        // `nsubj` désaccorde) a été tenté puis RETIRÉ : à 89 % UAS, les
        // attachements `nsubj` erronés triplaient les faux positifs (SVA 8 → 29
        // sur corpus correct). L'arbre sert donc uniquement de **veto** (précision),
        // pas de détecteur positif (cf. `verb_agrees_with_tree_subject`).

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
            Self::check_sentence(&sentence, None, &mut suggestions);
        }
        suggestions
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            Self::check_sentence(&sentence, Some(tags), &mut suggestions);
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
    fn nominal_plural_with_numeral() {
        // Numéral mot entre déterminant et nom → sauté.
        assert_eq!(first("les deux vigies regardait").as_deref(), Some("regardaient"));
        assert_eq!(first("les trois enfants dort").as_deref(), Some("dorment"));
    }

    #[test]
    fn nominal_plural_numeral_as_head() {
        // Numéral sans nom suivant : le numéral est la tête (« les deux regardaient »).
        assert_eq!(first("les deux regardait").as_deref(), Some("regardaient"));
        assert_eq!(first("Les 2 regardait au loin").as_deref(), Some("regardaient"));
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

    #[test]
    fn ambiguous_prenominal_noun_head() {
        // « petit » a une lecture nominale parasite : le vrai sujet est
        // « chats », et « mange » doit être corrigé.
        assert_eq!(first("les petit chats mange").as_deref(), Some("mangent"));
    }

    #[test]
    fn ambiguous_prenominal_no_false_positive() {
        // « principales » (nom/adjectif) ne doit pas être pris pour le nom tête :
        // le sujet est « mesures », « concernent » est déjà accordé.
        assert_eq!(count("les principales mesures concernent le pays"), 0);
    }

    // --- Chemin POS (`check_tagged`). ---

    fn tagged_first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SubjectVerbAgreement
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn tagged_count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SubjectVerbAgreement.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn pos_path_still_corrects_real_mismatches() {
        // Les vrais désaccords restent corrigés une fois le POS consommé.
        assert_eq!(tagged_first("ils mange").as_deref(), Some("mangent"));
        assert_eq!(tagged_first("les chats mange").as_deref(), Some("mangent"));
        assert_eq!(
            tagged_first("Pierre et Marie mange").as_deref(),
            Some("mangent")
        );
    }

    #[test]
    fn pos_path_noun_homograph_is_not_a_verb() {
        // « barres » est étiqueté NOUN : « des points/barres verticales » ne doit
        // plus déclencher un faux accord sujet–verbe.
        assert_eq!(tagged_count("des points barres verticales de couleur"), 0);
    }

    #[test]
    fn pos_path_object_of_infinitive_is_not_subject() {
        // « des postes » est COD de l'infinitif « redéployer » : pas un sujet,
        // donc pas d'accord forcé sur « compromis ».
        assert_eq!(
            tagged_count("la nécessité grandissante de pouvoir redéployer des postes compromis"),
            0
        );
    }

    #[test]
    fn pos_path_de_les_complement_is_not_subject() {
        // « des usages » (= de+les), complément de « démultiplication », ne doit
        // pas être pris pour le sujet de « alla ».
        assert_eq!(
            tagged_count("la démultiplication des usages digitaux alla de pair avec les risques"),
            0
        );
    }

    #[test]
    fn pos_path_relative_object_pronoun_is_not_subject() {
        // « nous », objet du relatif « qui », n'est pas le sujet de « a ».
        assert_eq!(tagged_count("le problème qui nous a été posé"), 0);
    }

    #[test]
    fn pos_path_object_vous_nous_is_not_subject() {
        // « vous »/« nous » sont aussi des clitiques objets : précédés d'un
        // remplisseur de la position sujet (« cela », un nom), ils ne sont pas le
        // sujet. Le verbe s'accorde avec le vrai sujet (3ᵉ sing.) — rien à signaler.
        assert_eq!(tagged_count("arrêtez la lecture si cela vous suffit"), 0);
        assert_eq!(
            tagged_count("notre expérience vous assure la réalisation"),
            0
        );
        assert_eq!(
            tagged_count("la rue de Nancy vous permettra de trouver mieux"),
            0
        );
        // Un « vous » réellement sujet reste corrigé.
        assert_eq!(tagged_first("vous mange").as_deref(), Some("mangez"));
    }

    #[test]
    fn pos_path_coordination_after_preposition_is_not_subject() {
        // « Saverne et environs » complète « producteurs » (introduit par « de ») :
        // ce n'est pas le sujet de « organise ».
        assert_eq!(
            tagged_count("l'association des producteurs de Saverne et environs organise un cours"),
            0
        );
        // Une vraie coordination sujet reste corrigée.
        assert_eq!(
            tagged_first("Pierre et Marie mange").as_deref(),
            Some("mangent")
        );
    }

    // --- Repli par suffixe 1ᵉʳ groupe (verbes absents du Lefff). ---

    fn tagged_first_sva(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SubjectVerbAgreement
            .check_tagged(&tokens, &tags)
            .into_iter()
            .find(|s| s.rule_id == "subject_verb_agreement")
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn tagged_count_sva(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        SubjectVerbAgreement
            .check_tagged(&tokens, &tags)
            .iter()
            .filter(|s| s.rule_id == "subject_verb_agreement")
            .count()
    }

    /// Comptage avec étiquettes **complètes** (POS + dépendances), comme en
    /// production : seul ce chemin active le veto par l'arbre.
    fn sva_count_full(text: &str) -> usize {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        SubjectVerbAgreement
            .check_tagged(&tokens, &tags)
            .iter()
            .filter(|s| s.rule_id == "subject_verb_agreement")
            .count()
    }

    #[test]
    fn tree_veto_does_not_hide_real_errors() {
        // Garde-fou : le veto par l'arbre (réduction des faux positifs, validée
        // sur corpus : SVA 13→8) ne doit PAS supprimer une vraie faute d'accord,
        // car le verbe n'y est précisément PAS accordé avec son nsubj.
        // « nous » (1pl) ≠ « bougeront » (3pl).
        assert_eq!(sva_count_full("Mais nous ne bougeront pas."), 1);
        // « je » (1sg) ≠ « retournes » (2sg).
        assert_eq!(sva_count_full("Je retournes souvent au marché."), 1);
    }

    #[test]
    fn er_verb_not_in_lefff_2sg_with_1pl_subject() {
        // « nous implémentes » : forme 2sg (-es) mais sujet 1pl → implémentons.
        assert_eq!(
            tagged_first_sva("nous implémentes la phase 12").as_deref(),
            Some("implémentons")
        );
    }

    #[test]
    fn er_verb_not_in_lefff_conditional_2pl_with_1sg_subject() {
        // « je conjugueriez » : forme conditionnelle 2pl (-eriez) mais sujet 1sg.
        assert_eq!(
            tagged_first_sva("je conjugueriez avec facilité").as_deref(),
            Some("conjuguerais")
        );
    }

    #[test]
    fn er_verb_not_in_lefff_correct_form_is_silent() {
        // Formes correctes avec des verbes rares → pas de faux positif.
        assert_eq!(tagged_count_sva("nous implémentons la phase 12"), 0);
        assert_eq!(tagged_count_sva("je conjuguerais avec facilité"), 0);
    }

    // --- Verbes irréguliers avec formes hors-Lefff (table de patch). ---

    #[test]
    fn aller_passe_simple_vous_3pl_form() {
        // « vous allèrent » : passé simple 3pl ≠ vous (2pl) → « allâtes ».
        assert_eq!(
            tagged_first_sva("Vous allèrent au marché hier matin.").as_deref(),
            Some("allâtes")
        );
    }

    #[test]
    fn irregular_verb_elle_prends() {
        // « elle prends » : 1/2sg de prendre ≠ 3sg → « prend ».
        assert_eq!(
            tagged_first_sva("Elle prends le train tous les matins.").as_deref(),
            Some("prend")
        );
    }
}
