//! Règle : confusion des **terminaisons homophones** des verbes du 1ᵉʳ groupe —
//! **tranche 5** du moteur de confusions de la phase 6 (cf.
//! [`corpus/confusion-terminaisons.md`](../../../../../corpus/confusion-terminaisons.md)).
//!
//! Adossée au CRF ([`crate::pos`]) et surtout aux **lectures du lexique**. Pour un
//! verbe en `-er`, l'infinitif (« manger »), le participe passé masculin singulier
//! (« mangé ») et la 2ᵉ personne du pluriel (« mangez ») se **prononcent à
//! l'identique** (/e/) : c'est la faute de français la plus répandue. Le mémo
//! Projet Voltaire : remplacer par un verbe du 3ᵉ groupe (« vendre »/« vendu ») —
//! si « vendre » convient, c'est l'infinitif (`-er`) ; si « vendu » convient, le
//! participe (`-é`).
//!
//! On tranche par le **gouverneur** du verbe (à gauche, clitiques/adverbes/« ne »
//! sautés) :
//!
//! ## … → -é (participe passé)
//!
//! Un **auxiliaire conjugué** (« avoir »/« être ») gouverne un participe passé,
//! jamais un infinitif : « j'ai **manger** » → « mangé », « elle est **tomber** »
//! → « tombée » (accord avec le sujet pour « être »).
//!
//! ## … → -er (infinitif)
//!
//! Une **préposition** (`à`, `de`, `pour`, `sans`) ou un **semi-auxiliaire**
//! conjugué (`aller`, `vouloir`, `pouvoir`, `devoir`, `aimer`…) gouverne un
//! infinitif : « il commence à **mangé** » → « manger », « je veux **mangé** » →
//! « manger ».
//!
//! ## … → -ez (2ᵉ personne du pluriel)
//!
//! Un sujet « vous » en tête de proposition appelle la forme en `-ez` :
//! « Vous **manger** trop » → « mangez ».
//!
//! Limites assumées (gaps), documentées dans le corpus : la confusion
//! **-ai/-ais/-ait** (futur ↔ conditionnel) relève d'une désambiguïsation
//! sémantique non séparable, et l'opposition **-ais/-ait** (personne) est déjà
//! traitée par l'accord sujet–verbe ([`crate::rules::conjugation`]).

use super::{is_finite_verb, normalize, upos};
use crate::morpho::{self, Gender, MoodTense, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions de terminaison en /e/ (`-er`/`-é`/`-ez`).
pub struct TerminaisonsConfusion;

const RULE_ID: &str = "confusion_terminaison";

/// Forme de surface d'un verbe du 1ᵉʳ groupe en /e/.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ErForm {
    /// Infinitif (`-er`).
    Infinitive,
    /// Participe passé (`-é`/`-ée`/`-és`/`-ées`).
    Participle,
    /// 2ᵉ personne du pluriel (`-ez`).
    Vous,
}

/// Prépositions gouvernant un infinitif.
const INF_PREPOSITIONS: &[&str] = &["à", "de", "d", "pour", "sans"];

/// Pronoms/quantifieurs interdisant la lecture infinitive après « de » : dans
/// « rien **de** changé », « quelque chose **de** cassé », « de » introduit un
/// participe/adjectif, pas un infinitif.
const DE_BLOCKERS: &[&str] = &["rien", "chose", "quoi", "personne", "autre"];

/// Lemmes des semi-auxiliaires conjugués qui gouvernent un infinitif.
const SEMI_MODALS: &[&str] = &[
    "aller",
    "vouloir",
    "pouvoir",
    "devoir",
    "savoir",
    "faire",
    "laisser",
    "aimer",
    "adorer",
    "détester",
    "préférer",
    "souhaiter",
    "désirer",
    "espérer",
    "oser",
    "faillir",
    "sembler",
    "paraître",
    "falloir",
    "venir",
    "compter",
    "daigner",
];

/// Conjonctions de coordination : licencient un « vous » sujet en tête de membre.
const COORD_CONJ: &[&str] = &["et", "mais", "ou", "donc", "or", "ni", "car"];

/// Adverbes pouvant s'intercaler entre le gouverneur et le verbe.
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
            | "trop"
            | "tant"
            | "enfin"
    )
}

/// Vrai si le jeton est la négation « ne ».
fn is_ne(text: &str) -> bool {
    matches!(normalize(text).as_str(), "ne" | "n")
}

/// Clitiques objets préverbaux, sautés entre le gouverneur et le verbe.
fn is_object_clitic(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "le" | "la"
            | "les"
            | "l"
            | "lui"
            | "leur"
            | "me"
            | "m"
            | "te"
            | "t"
            | "se"
            | "s"
            | "en"
            | "y"
            | "nous"
            | "vous"
    )
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « avoir ».
fn is_avoir(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "avoir")
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « être ».
fn is_etre(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "être")
}

/// Si `text` est une forme en /e/ d'un verbe du 1ᵉʳ groupe (lemme en `-er`),
/// renvoie son lemme (l'infinitif) et sa forme de surface. On s'appuie sur les
/// **lectures du lexique** : les noms/adjectifs homographes en `-er`/`-é`/`-ez`
/// (« fer », « clé », « nez », « côté ») et les verbes des autres groupes
/// (« partir »/« parti », « faire »/« fait ») sont ainsi écartés.
fn er_verb_form(text: &str) -> Option<(String, ErForm)> {
    let lower = text.to_lowercase();
    let morphs = morpho::lookup(text);

    if lower.ends_with("er") && morphs.iter().any(|m| is_verb_lemma(m, &lower)) {
        return Some((lower, ErForm::Infinitive));
    }

    if lower.ends_with('é')
        || lower.ends_with("ée")
        || lower.ends_with("és")
        || lower.ends_with("ées")
    {
        if let Some(m) = morphs.iter().find(|m| {
            m.category == MorphCategory::Verb
                && m.person.is_none()
                && (m.gender.is_some() || m.number.is_some())
                && m.lemma.ends_with("er")
        }) {
            return Some((m.lemma.clone(), ErForm::Participle));
        }
    }

    if lower.ends_with("ez") {
        if let Some(v) = morpho::verb_forms(text).iter().find(|v| {
            v.lemma.ends_with("er") && v.person == Person::Second && v.number == Number::Plural
        }) {
            return Some((v.lemma.clone(), ErForm::Vous));
        }
    }

    None
}

/// Vrai si `m` analyse `form` comme l'infinitif d'un verbe (lemme = la forme).
fn is_verb_lemma(m: &morpho::Morph, form: &str) -> bool {
    m.category == MorphCategory::Verb && m.lemma == form
}

/// Position du premier jeton lexical à gauche de `i` qui n'est ni un adverbe ni
/// « ne » (les clitiques objets ne sont **pas** sautés : ils marqueraient un
/// « vous » objet, pas sujet).
fn adverbial_left(sentence: &[(usize, &Token)], i: usize) -> Option<usize> {
    let mut k = i;
    while k > 0 {
        k -= 1;
        let t = sentence[k].1.text.as_str();
        if is_skippable_adverb(t) || is_ne(t) {
            continue;
        }
        return Some(k);
    }
    None
}

/// Position du **gouverneur** du verbe : premier jeton lexical à gauche de `i`
/// qui n'est ni adverbe, ni « ne », ni clitique objet préverbal.
fn governor_left(sentence: &[(usize, &Token)], i: usize) -> Option<usize> {
    let mut k = i;
    while k > 0 {
        k -= 1;
        let t = sentence[k].1.text.as_str();
        if is_skippable_adverb(t) || is_ne(t) || is_object_clitic(t) {
            continue;
        }
        return Some(k);
    }
    None
}

/// Vrai si le jeton en position `g` gouverne un **infinitif** : préposition
/// infinitive (avec garde sur « de ») ou semi-auxiliaire conjugué.
fn is_infinitive_governor(sentence: &[(usize, &Token)], g: usize) -> bool {
    let gov = normalize(sentence[g].1.text.as_str());

    if INF_PREPOSITIONS.contains(&gov.as_str()) {
        if gov == "de" || gov == "d" {
            if let Some(b) = adverbial_left(sentence, g) {
                if DE_BLOCKERS.contains(&normalize(sentence[b].1.text.as_str()).as_str()) {
                    return false;
                }
            }
        }
        return true;
    }

    let gov_text = sentence[g].1.text.as_str();
    is_finite_verb(gov_text)
        && morpho::verb_forms(gov_text)
            .iter()
            .any(|v| SEMI_MODALS.contains(&v.lemma.as_str()))
}

/// Genre et nombre du sujet de l'auxiliaire « être » en position `g`, pour
/// accorder le participe. Sujet : à gauche de l'auxiliaire, « ne »/réfléchi
/// sautés. Défaut prudent (masculin singulier) si le sujet est inconnu.
fn etre_subject_features(sentence: &[(usize, &Token)], g: usize) -> (Gender, Number) {
    let mut s = g;
    while s > 0 {
        let t = sentence[s - 1].1.text.as_str();
        if is_ne(t) || is_object_clitic(t) {
            s -= 1;
        } else {
            break;
        }
    }
    if s == 0 {
        return (Gender::Masculine, Number::Singular);
    }
    let subj = normalize(sentence[s - 1].1.text.as_str());
    match subj.as_str() {
        "il" => return (Gender::Masculine, Number::Singular),
        "elle" => return (Gender::Feminine, Number::Singular),
        "ils" => return (Gender::Masculine, Number::Plural),
        "elles" => return (Gender::Feminine, Number::Plural),
        "nous" | "vous" => return (Gender::Masculine, Number::Plural),
        "je" | "j" | "tu" | "on" => return (Gender::Masculine, Number::Singular),
        _ => {}
    }
    let nouns: Vec<_> = morpho::lookup(sentence[s - 1].1.text.as_str())
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    let gender = unanimous(nouns.iter().map(|m| m.gender)).filter(|g| *g != Gender::Epicene);
    let number = unanimous(nouns.iter().map(|m| m.number));
    (
        gender.unwrap_or(Gender::Masculine),
        number.unwrap_or(Number::Singular),
    )
}

/// Valeur commune à toutes les analyses, ou `None` si elles divergent / manquent.
fn unanimous<T: PartialEq>(mut it: impl Iterator<Item = Option<T>>) -> Option<T> {
    let first = it.next()??;
    for v in it {
        if v? != first {
            return None;
        }
    }
    Some(first)
}

/// Vrai si « vous » en position `p` est en **position sujet** : début de
/// proposition (rien à gauche) ou après une conjonction de coordination. Un
/// pronom/verbe/préposition à gauche en ferait un objet (« il veut vous voir »).
fn is_subject_vous(sentence: &[(usize, &Token)], p: usize) -> bool {
    match adverbial_left(sentence, p) {
        None => true,
        Some(q) => COORD_CONJ.contains(&normalize(sentence[q].1.text.as_str()).as_str()),
    }
}

/// Construit la suggestion de correction d'un verbe en position `i`.
fn correction(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<Suggestion> {
    let token = sentence[i].1;
    let (lemma, surface) = er_verb_form(&token.text)?;

    // Garde POS : le candidat doit être lu comme un **verbe** par le CRF. Les
    // formes en /e/ homographes d'un nom (« la volée », « à la portée », « de la
    // durée ») sont étiquetées `NOUN` et ne doivent pas être « corrigées » en
    // infinitif. Le CRF tranche l'homographie nom/verbe ; on le consomme, à la
    // Grammalecte, plutôt que de raisonner sur la seule terminaison.
    if !matches!(upos(sentence, i, tags), Upos::Verb | Upos::Aux) {
        return None;
    }

    // 1. Sujet « vous » en tête → 2ᵉ personne du pluriel (-ez).
    if let Some(p) = adverbial_left(sentence, i) {
        if normalize(sentence[p].1.text.as_str()) == "vous" && is_subject_vous(sentence, p) {
            if surface == ErForm::Vous {
                return None;
            }
            let target = morpho::conjugate(
                &lemma,
                MoodTense::IndicativePresent,
                Person::Second,
                Number::Plural,
            )?;
            return build(token, &target);
        }
    }

    let g = governor_left(sentence, i)?;
    let gov = sentence[g].1.text.as_str();

    // 2. Auxiliaire « avoir »/« être » → participe passé (-é).
    if is_avoir(gov) {
        if surface == ErForm::Participle {
            return None;
        }
        let target = morpho::participle(&lemma, Gender::Masculine, Number::Singular)?;
        return build(token, &target);
    }
    if is_etre(gov) {
        if surface == ErForm::Participle {
            return None;
        }
        let (gender, number) = etre_subject_features(sentence, g);
        let target = morpho::participle(&lemma, gender, number)?;
        return build(token, &target);
    }

    // 3. Préposition / semi-auxiliaire → infinitif (-er).
    if is_infinitive_governor(sentence, g) {
        if surface == ErForm::Infinitive {
            return None;
        }
        return build(token, &lemma);
    }

    None
}

/// Assemble la suggestion (casse calquée, no-op écarté).
fn build(token: &Token, target: &str) -> Option<Suggestion> {
    if target.eq_ignore_ascii_case(&token.text) {
        return None;
    }
    let replacement = super::match_case(&token.text, target);
    Some(Suggestion {
        span: token.span,
        message: format!(
            "Terminaison en -er/-é/-ez : « {} » devrait s'écrire « {} ».",
            token.text, replacement
        ),
        replacements: vec![replacement],
        rule_id: RULE_ID,
    })
}

impl Rule for TerminaisonsConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                if let Some(s) = correction(&sentence, i, tags) {
                    suggestions.push(s);
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion des terminaisons en -er / -é / -ez"
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
        TerminaisonsConfusion
            .check(&tokens)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        TerminaisonsConfusion.check(&tokens).len()
    }

    // --- … → -é (participe passé) ---

    #[test]
    fn infinitive_after_avoir_becomes_participle() {
        assert_eq!(first("il a manger une pomme").as_deref(), Some("mangé"));
        assert_eq!(first("j'ai bien manger ce soir").as_deref(), Some("mangé"));
        assert_eq!(
            first("elle a chanter toute la nuit").as_deref(),
            Some("chanté")
        );
        assert_eq!(first("il ne l'a pas manger").as_deref(), Some("mangé"));
    }

    #[test]
    fn infinitive_after_etre_agrees_with_subject() {
        assert_eq!(
            first("elle est tomber dans l'escalier").as_deref(),
            Some("tombée")
        );
        assert_eq!(first("ils sont arriver hier").as_deref(), Some("arrivés"));
    }

    // --- … → -er (infinitif) ---

    #[test]
    fn participle_after_preposition_becomes_infinitive() {
        assert_eq!(first("il commence à mangé").as_deref(), Some("manger"));
        assert_eq!(
            first("il décide de mangé maintenant").as_deref(),
            Some("manger")
        );
        assert_eq!(first("c'est facile à réalisé").as_deref(), Some("réaliser"));
        assert_eq!(first("il part sans mangé").as_deref(), Some("manger"));
    }

    #[test]
    fn participle_after_semi_modal_becomes_infinitive() {
        assert_eq!(first("je veux mangé maintenant").as_deref(), Some("manger"));
        assert_eq!(
            first("il doit travaillé demain").as_deref(),
            Some("travailler")
        );
        assert_eq!(
            first("nous allons mangé bientôt").as_deref(),
            Some("manger")
        );
    }

    // --- … → -ez (2ᵉ pers. pluriel) ---

    #[test]
    fn subject_vous_becomes_ez() {
        assert_eq!(
            first("vous manger trop de sucre").as_deref(),
            Some("mangez")
        );
        assert_eq!(first("Vous mangé trop").as_deref(), Some("mangez"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "il a mangé une pomme",       // avoir + participe correct
            "elle est tombée",            // être + participe accordé
            "ils sont arrivés hier",      // être + participe accordé
            "il commence à manger",       // préposition + infinitif correct
            "je veux manger",             // semi-modal + infinitif correct
            "il va manger ce soir",       // aller + infinitif correct
            "vous mangez trop",           // sujet vous + -ez correct
            "il veut vous voir demain",   // « vous » objet, pas sujet
            "il n'y a rien de changé",    // « de » + participe adjectival
            "le saumon fumé est bon",     // participe épithète, pas gouverné
            "un travail à finir",         // « à » + infinitif (3ᵉ groupe)
            "il a un déjeuner important", // nom homographe après déterminant
            "il est venu hier",           // participe en -ir, hors champ
            "nous avons fait le travail", // participe en -re, hors champ
            // Nom en /e/ homographe d'un verbe, après un déterminant : le CRF
            // l'étiquette NOUN, la garde POS l'écarte (« à la volée » ne devient
            // pas « voler »).
            "il faut formater à la volée",
            "c'est à la portée de tous",
            "pendant toute la durée du film",
            "il défend la liberté de la pensée",
            "une montée raide vers le sommet",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn case_is_preserved() {
        // La casse calque le jeton verbe (ici minuscule), pas le début de phrase.
        assert_eq!(first("Il a manger").as_deref(), Some("mangé"));
        // Jeton verbe capitalisé (titre) → correction capitalisée.
        assert_eq!(first("Vous Mangez").as_deref(), None); // déjà correct
    }
}
