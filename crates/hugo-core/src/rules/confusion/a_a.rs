//! Règle : confusion « a » / « à » — **tranche 1** du moteur de confusions de la
//! phase 6 (cf. [`corpus/confusion-a-a.md`](../../../../corpus/confusion-a-a.md)).
//!
//! Elle remplace l'ancien veto fragile de [`crate::rules::homophones`] par une
//! désambiguïsation **adossée au CRF** ([`crate::pos`]) : on raisonne sur la
//! catégorie tranchée du voisinage plutôt que sur les seuls homographes du
//! lexique. Mémo (Projet Voltaire) : « a » est le verbe *avoir* si on peut le
//! remplacer par « avait » ; sinon c'est la préposition « à ».
//!
//! ## Direction `a → à` (« a » saisi, « à » attendu)
//!
//! - **après un verbe** : un verbe plein (tagué `VERB`, lemme ≠ *avoir*) ne peut
//!   être suivi de l'auxiliaire « a » → « il va a Paris » → « à ». Le veto POS
//!   (le mot précédent doit être *réellement* tagué verbe, pas seulement
//!   verbe-possible au lexique) neutralise l'homographe « y » de « il y a » ;
//! - **NOM + a + NOM** : entre deux noms nus, « a » est la préposition, sauf si
//!   le second nom appartient aux **idiomes *avoir* + nom** (`a faim`, `a
//!   raison`…) — liste reconstituée [`AVOIR_IDIOMS`] ;
//! - **a + infinitif** : *avoir* n'est jamais suivi d'un infinitif → « difficile
//!   a résoudre » → « à » ;
//! - **locutions** figées ([`LOC_AFTER`], [`LOC_BEFORE`], redoublements) :
//!   « a cause de », « face a », « petit a petit ».
//!
//! ## Direction `à → a` (« à » saisi, « a » attendu)
//!
//! La préposition « à » ne peut occuper la place du verbe. En remontant depuis
//! « à » au-delà des **clitiques objets** (`y / l' / les / nous / ne …`), on
//! cherche le sujet :
//!
//! - **pronom sujet fort** (`il / elle / on`, et `ils / elles` → « ont ») : « à »
//!   est forcément le verbe → « il y à trop » → « a », « on nous à promis » →
//!   « a » ;
//! - **sujet nominal** ou relatif **`qui`** : on n'agit que si « à » est suivi
//!   (adverbes sautés) d'un **participe passé** (passé composé) → « la situation
//!   à changé » → « a » ; cela épargne « une machine à laver » (infinitif) et
//!   « qui, à mon avis, … » (« à » préposition).
//!
//! Limites assumées (cf. corpus) : « tarte a la rhubarbe » et plus généralement
//! `a + déterminant + nom` restent hors de portée (« il a la grippe » est un
//! *avoir* parfaitement correct, structurellement identique).

use super::{match_case, normalize};
use crate::morpho::{self, Number};
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Détecte les confusions « a » / « à ».
pub struct AAConfusion;

const RULE_ID: &str = "confusion_a_a";

/// Idiomes *avoir* + nom nu : le nom suit « a » sans en faire une préposition.
/// Liste **reconstituée** pour ce projet (cf. corpus), à enrichir au fil de l'eau.
const AVOIR_IDIOMS: &[&str] = &[
    "accès",
    "affaire",
    "besoin",
    "chance",
    "chances",
    "charge",
    "confiance",
    "conscience",
    "cours",
    "crainte",
    "droit",
    "envie",
    "faim",
    "force",
    "froid",
    "garde",
    "hâte",
    "honte",
    "horreur",
    "intérêt",
    "lieu",
    "mal",
    "peine",
    "peur",
    "pitié",
    "raison",
    "rapport",
    "recours",
    "soif",
    "sommeil",
    "tendance",
    "tort",
    "trait",
    "vocation",
];

/// Locutions « a + mot » où « a » est la préposition « à » (« a cause de »…).
const LOC_AFTER: &[&str] = &[
    "cause",
    "travers",
    "propos",
    "nouveau",
    "savoir",
    "mesure",
    "défaut",
    "condition",
    "contrecœur",
    "outrance",
];

/// Locutions « mot + a » où « a » est la préposition « à » (« face a »…).
const LOC_BEFORE: &[&str] = &["face", "grâce", "quant", "rapport", "jusque", "vis-à-vis"];

/// Mots qui se redoublent autour de « a » préposition (« petit a petit »…).
const REDUPLICATIONS: &[&str] = &[
    "petit", "pas", "mot", "côte", "goutte", "peu", "deux", "jour",
];

/// Pronoms personnels sujets « forts » dont l'auxiliaire *avoir* est « a » : un
/// « à » qui les suit (clitiques sautés) est forcément le verbe.
const STRONG_SUBJECTS_A: &[&str] = &["il", "elle", "on"];

/// Idem, mais à la 3ᵉ personne du pluriel : l'auxiliaire est « ont ».
const STRONG_SUBJECTS_ONT: &[&str] = &["ils", "elles"];

/// Clitiques objets (et négation) traversés en remontant vers le sujet. Les
/// formes élidées sont normalisées sans apostrophe (`l'` → `l`).
const OBJECT_CLITICS: &[&str] = &[
    "y", "en", "le", "la", "les", "l", "lui", "leur", "me", "m", "te", "t", "se", "s", "nous",
    "vous", "ne", "n",
];

/// Déterminants pluriels (indice de sujet nominal pluriel → « ont »).
const PLURAL_DETERMINERS: &[&str] = &[
    "les",
    "des",
    "mes",
    "tes",
    "ses",
    "ces",
    "nos",
    "vos",
    "leurs",
    "certains",
    "plusieurs",
];

/// Vrai si une analyse de `form` est le verbe *avoir* (forme finie).
fn is_avoir(form: &str) -> bool {
    morpho::verb_forms(form).iter().any(|v| v.lemma == "avoir")
}

/// Vrai si `form` admet une analyse **infinitive** : un enregistrement verbal
/// dont le lemme est la forme elle-même (« pleuvoir », « résoudre »). Les
/// participes (« mangé », lemme « manger ») et les formes finies en sont exclus.
fn is_infinitive(form: &str) -> bool {
    let lower = form.to_lowercase();
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == morpho::MorphCategory::Verb && m.lemma == lower)
}

/// Vrai si `form` admet une analyse de **participe passé** : enregistrement
/// verbal sans personne mais porteur d'un genre ou d'un nombre (« changé »,
/// « prévenue », « venus »).
fn is_past_participle(form: &str) -> bool {
    morpho::lookup(form).iter().any(|m| {
        m.category == morpho::MorphCategory::Verb
            && m.person.is_none()
            && (m.gender.is_some() || m.number.is_some())
    })
}

/// Vrai si `form` admet une lecture de **nom commun** au lexique.
fn has_noun_reading(form: &str) -> bool {
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == morpho::MorphCategory::Noun)
}

/// Catégorie POS du jeton lexical à la position `k` de la phrase.
fn upos(sentence: &[(usize, &Token)], k: usize, tags: &[Tagged]) -> Upos {
    tags[sentence[k].0].upos
}

/// Cherche, à droite de la position `i`, un participe passé en sautant les
/// adverbes (« à beaucoup changé », « à enfin réparé »). Renvoie vrai si trouvé.
fn participle_follows(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> bool {
    let mut k = i + 1;
    while let Some((idx, tok)) = sentence.get(k) {
        if tags[*idx].upos == Upos::Adv || matches!(normalize(&tok.text).as_str(), "ne" | "n") {
            k += 1;
            continue;
        }
        return is_past_participle(&tok.text);
    }
    false
}

/// Correction d'un « a » en « à », d'après son voisinage tagué. `i` est la
/// position du « a » dans la phrase lexicale.
fn correction_a(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<&'static str> {
    let prev = (i > 0).then(|| sentence[i - 1].1.text.as_str());
    let prev_norm = prev.map(normalize);
    let prev_upos = (i > 0).then(|| upos(sentence, i - 1, tags));
    let next = sentence.get(i + 1).map(|(_, t)| t.text.as_str());
    let next_norm = next.map(normalize);
    let next_upos = sentence.get(i + 1).map(|_| upos(sentence, i + 1, tags));

    // 1. Verbe plein + a : « a » ne peut être l'auxiliaire → préposition « à ».
    //    Le veto POS (prev *réellement* tagué VERB) écarte l'homographe « y ».
    if prev_upos == Some(Upos::Verb) && !prev.is_some_and(is_avoir) {
        return Some("à");
    }

    // 2. a + infinitif : *avoir* n'est jamais suivi d'un infinitif → « à ».
    if next_upos == Some(Upos::Verb) && next.is_some_and(is_infinitive) {
        return Some("à");
    }

    // 3. NOM (commun) + a + NOM/NOM-propre nu, hors idiomes *avoir* + nom.
    //    Le mot suivant compte comme nom s'il est tagué NOUN/PROPN, ou — quand le
    //    CRF le tague VERB — s'il a une lecture nominale au lexique **sans** lecture
    //    de participe passé (à la Grammalecte, on consulte les analyses possibles).
    //    C'est ce qui rattrape l'homographe « moulin a poivre » (poivre = nom mal
    //    tagué VERB) tout en épargnant « le chat a marché » (marché = participe). On
    //    n'étend volontairement pas aux tags DET/ADV/PRON, dont les lectures
    //    nominales parasites (« la » note de musique, « peu », « pas »…) casseraient
    //    « le chat a la rage » ou « il a peu de temps ».
    let next_is_nominal = match next_upos {
        Some(Upos::Noun | Upos::Propn) => true,
        Some(Upos::Verb) => next.is_some_and(|w| has_noun_reading(w) && !is_past_participle(w)),
        _ => false,
    };
    if prev_upos == Some(Upos::Noun)
        && next_is_nominal
        && !next_norm
            .as_deref()
            .is_some_and(|n| AVOIR_IDIOMS.contains(&n))
    {
        return Some("à");
    }

    // 4. Locutions figées et redoublements.
    if next_norm.as_deref().is_some_and(|n| LOC_AFTER.contains(&n))
        || prev_norm
            .as_deref()
            .is_some_and(|p| LOC_BEFORE.contains(&p))
        || (prev_norm.is_some() && prev_norm == next_norm)
            && prev_norm
                .as_deref()
                .is_some_and(|p| REDUPLICATIONS.contains(&p))
    {
        return Some("à");
    }

    None
}

/// Vrai si le sujet nominal à la position `k` est au pluriel (déterminant
/// pluriel ou nombre morphologique pluriel sans lecture singulière).
fn subject_is_plural(sentence: &[(usize, &Token)], k: usize) -> bool {
    if k > 0 {
        let det = normalize(sentence[k - 1].1.text.as_str());
        if PLURAL_DETERMINERS.contains(&det.as_str()) {
            return true;
        }
    }
    let numbers: Vec<Number> = morpho::lookup(sentence[k].1.text.as_str())
        .iter()
        .filter_map(|m| m.number)
        .collect();
    !numbers.is_empty() && numbers.iter().all(|&n| n == Number::Plural)
}

/// Correction d'un « à » en « a » / « ont », d'après le sujet de la proposition.
fn correction_a_grave(
    sentence: &[(usize, &Token)],
    i: usize,
    tags: &[Tagged],
) -> Option<&'static str> {
    // Remonter vers le sujet en sautant les clitiques objets.
    let mut k = i;
    loop {
        if k == 0 {
            return None;
        }
        k -= 1;
        let t = normalize(sentence[k].1.text.as_str());
        if OBJECT_CLITICS.contains(&t.as_str()) {
            continue;
        }
        break;
    }

    let subj = normalize(sentence[k].1.text.as_str());
    let subj_upos = upos(sentence, k, tags);

    // Pronom sujet fort : « à » est forcément l'auxiliaire.
    if STRONG_SUBJECTS_A.contains(&subj.as_str()) {
        return Some("a");
    }
    if STRONG_SUBJECTS_ONT.contains(&subj.as_str()) {
        return Some("ont");
    }

    // Sujet nominal, pronom sujet (« personne », « celui »…, les clitiques objets
    // ayant été sautés) ou relatif « qui » : on exige un participe passé après
    // « à » (passé composé), pour épargner « machine à laver » (infinitif) et
    // « qui, à mon avis, … » (« à » préposition).
    let nominal = subj == "qui" || matches!(subj_upos, Upos::Noun | Upos::Propn | Upos::Pron);
    if nominal && participle_follows(sentence, i, tags) {
        if subj != "qui" && subject_is_plural(sentence, k) {
            return Some("ont");
        }
        return Some("a");
    }

    None
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « a »/« à » : « {} » devrait être « {} ».",
            token.text, corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for AAConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        // La règle est intrinsèquement adossée au POS : sans tags, on les calcule.
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let token = sentence[i].1;
                let corrected = match normalize(&token.text).as_str() {
                    "a" => correction_a(&sentence, i, tags),
                    "à" => correction_a_grave(&sentence, i, tags),
                    _ => None,
                };
                if let Some(c) = corrected {
                    suggestions.push(suggestion(token, c));
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « a » / « à »"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    /// Première correction (chemin tagué, comme en production).
    fn first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AAConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AAConfusion.check_tagged(&tokens, &tags).len()
    }

    /// Toutes les corrections d'un texte, dans l'ordre des positions.
    fn all(text: &str) -> Vec<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AAConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .filter_map(|s| s.replacements.into_iter().next())
            .collect()
    }

    // --- a → à ---

    #[test]
    fn a_to_grave_after_verb() {
        assert_eq!(first("il va a Paris").as_deref(), Some("à"));
        assert_eq!(first("elle pense a lui").as_deref(), Some("à"));
        assert_eq!(first("il commence a manger").as_deref(), Some("à"));
        assert_eq!(
            first("Le train repart a Bordeaux dans dix minutes").as_deref(),
            Some("à")
        );
    }

    #[test]
    fn a_to_grave_noun_plus_noun() {
        assert_eq!(
            first("Range les tasses a café sur l'étagère").as_deref(),
            Some("à")
        );
        assert_eq!(
            first("Mon grand-père collectionne les fers a repasser").as_deref(),
            Some("à")
        );
        assert_eq!(
            first("ses bottes a clous en montagne").as_deref(),
            Some("à")
        );
        // Homographe nom/verbe mal tagué VERB par le CRF : rattrapé par la
        // lecture nominale au lexique (poivre = nom, sans lecture de participe).
        assert_eq!(first("un moulin a poivre en bois").as_deref(), Some("à"));
        assert_eq!(first("un bateau a moteur").as_deref(), Some("à"));
        assert_eq!(first("une robe a fleurs").as_deref(), Some("à"));
    }

    #[test]
    fn noun_plus_past_participle_stays_avoir() {
        // Le garde « pas de participe passé » épargne les vrais passés composés
        // dont le participe est aussi un nom (« marché »), et les idiomes.
        for ok in [
            "Le chat a marché sur la table",
            "La cloche a sonné trois fois",
            "Le moteur a tourné toute la nuit",
            "Le professeur a cours ce matin",
            "Le prix a doublé cette année",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn a_to_grave_before_infinitive() {
        assert_eq!(
            first("Cette énigme est difficile a résoudre").as_deref(),
            Some("à")
        );
        assert_eq!(
            first("Tu n'as rien a craindre de lui").as_deref(),
            Some("à")
        );
        assert_eq!(first("il commence a pleuvoir").as_deref(), Some("à"));
    }

    #[test]
    fn a_to_grave_in_locutions() {
        assert_eq!(
            first("Le marché est fermé a cause de la tempête").as_deref(),
            Some("à")
        );
        assert_eq!(first("Assis face a l'océan").as_deref(), Some("à"));
        assert_eq!(first("A partir de lundi").as_deref(), Some("À"));
    }

    #[test]
    fn a_to_grave_reduplication() {
        // « Petit a petit » : seul le « a » entre les deux redoublements vire à « à ».
        assert_eq!(
            first("Petit a petit le jardin a repris vie").as_deref(),
            Some("à")
        );
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Il va a Paris").as_deref(), Some("à"));
    }

    // --- à → a ---

    #[test]
    fn grave_to_a_after_strong_pronoun() {
        assert_eq!(first("il à faim").as_deref(), Some("a"));
        assert_eq!(first("elle à un chat").as_deref(), Some("a"));
        assert_eq!(first("ils à mangé").as_deref(), Some("ont"));
    }

    #[test]
    fn grave_to_a_after_clitic() {
        assert_eq!(
            first("Il y à trop de bruit dans cette salle").as_deref(),
            Some("a")
        );
        assert_eq!(first("On nous à promis une réponse").as_deref(), Some("a"));
        assert_eq!(
            first("Personne ne l'à prévenue du changement").as_deref(),
            Some("a")
        );
        assert_eq!(first("Qui à laissé la porte ouverte").as_deref(), Some("a"));
    }

    #[test]
    fn grave_to_a_nominal_subject_with_participle() {
        assert_eq!(
            first("La situation à beaucoup changé depuis hier").as_deref(),
            Some("a")
        );
        assert_eq!(
            first("Mon voisin à enfin réparé sa clôture").as_deref(),
            Some("a")
        );
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "Le chiot a faim depuis ce matin",
            "Cette demande a peu de chances d'aboutir",
            "Le spectacle a lieu samedi soir",
            "Elle a besoin d'un peu de repos",
            "Il y a du monde sur la place",
            "Le chat a renversé son bol d'eau",
            "Personne n'a rien remarqué",
            "Nous partons à Lyon en début d'après-midi",
            "C'est une vraie machine à laver le linge",
            "Il faut penser à tout",
            "son chat dort",
            "il va à Paris",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn minimal_pair_flags_only_the_preposition() {
        // « Elle pense a tout mais a peu de temps » : 1er « a » → à, 2e « a » OK.
        assert_eq!(all("Elle pense a tout mais a peu de temps"), vec!["à"]);
    }

    #[test]
    fn det_noun_after_a_is_a_known_gap() {
        // Limite **structurelle** : « a + déterminant + nom » est productif et
        // correct comme avoir (« il a la grippe », « le gâteau a la forme d'un
        // cœur ») ; aucune liste d'exceptions ne sépare « la rhubarbe » de « la
        // grippe ». On préfère manquer « tarte a la rhubarbe » que sur-corriger.
        assert_eq!(count("On a servi une tarte a la rhubarbe"), 0);
        // Contrepartie : aucun faux positif sur l'avoir + déterminant + nom.
        assert_eq!(count("le chat a la rage"), 0);
        assert_eq!(count("le gâteau a la forme d'un cœur"), 0);
    }

    #[test]
    fn no_cross_sentence_leak() {
        // « va » (phrase 1) ne doit pas déclencher « a » de la phrase 2.
        assert_eq!(count("il va. a b c"), 0);
    }
}
