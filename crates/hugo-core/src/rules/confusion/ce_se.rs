//! Règle : confusions **ce/se** et **c'est/s'est** — **tranche 2** du moteur de
//! confusions de la phase 6 (cf.
//! [`corpus/confusion-ce-se.md`](../../../../../corpus/confusion-ce-se.md)).
//!
//! Comme la tranche 1 (a/à), elle s'adosse au CRF ([`crate::pos`]) : on raisonne
//! sur la catégorie tranchée du voisinage. Elle reprend — en l'enrichissant — le
//! traitement ce↔se historiquement porté par [`crate::rules::homophones`].
//!
//! Mémo (Projet Voltaire) :
//! - **se** est le pronom réfléchi de la 3ᵉ personne, **toujours préverbal**
//!   (« il **se** lave ») ; **ce** est le démonstratif, devant un nom (« **ce**
//!   livre ») ou en pronom (« **ce** que »). On peut remplacer « se » par « me/te »
//!   en changeant de personne (« je **me** lave »).
//! - **s'est** = pronom réfléchi + auxiliaire, **toujours suivi d'un participe
//!   passé** (« il **s'est** levé ») ; **c'est** = « cela est » (« **c'est** beau »).
//!
//! ## ce → se (« il ce lève » → « il **se** lève »)
//!
//! « ce » coincé entre un **sujet** (pronom, éventuellement après « ne ») et un
//! **verbe conjugué** est le réfléchi « se » : le démonstratif « ce » ne s'emploie
//! pas là (il introduirait un nom).
//!
//! ## se → ce (« se livre est lourd » → « **ce** livre… »)
//!
//! « se » devant un **nom/nom propre/adjectif** (étiquette CRF) ou devant un
//! relatif **que/qui/dont** est le démonstratif « ce » : un « se » réfléchi est
//! toujours préverbal. Le « se livre » verbal légitime reste étiqueté verbe.
//!
//! ## c' → s' (« il c'est trompé » → « il **s'est** trompé »)
//!
//! Devant « est »/« était », l'élision « c' » précédée d'un **sujet de 3ᵉ
//! personne** (`il/elle/on`, relatif `qui`, ou un nom/nom propre, sans virgule
//! intercalée) et **suivie d'un participe passé** est le réfléchi « s' » :
//! « cela est » ne saurait s'intercaler entre un sujet et son participe.
//!
//! ## s' → c' (« s'est magnifique » → « **c'est** magnifique »)
//!
//! Inversement, « s' » + « est » **non suivi d'un participe passé** (adverbes
//! sautés) est « c'est » : l'auxiliaire réfléchi appelle obligatoirement un
//! participe.
//!
//! Limites assumées (cf. corpus) : **ces/ses** (deux déterminants également
//! valides devant un nom) relèvent d'une ambiguïté **structurelle** hors de portée
//! d'une règle grammaticale ; **sais/sait** sont déjà traités par l'accord
//! sujet–verbe ([`crate::rules::conjugation`]).

use super::{match_case, normalize};
use crate::morpho;
use crate::pos::{Tagged, Upos};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Détecte les confusions « ce »/« se » et « c'est »/« s'est ».
pub struct CeSeConfusion;

const RULE_ID: &str = "confusion_ce_se";

/// Pronoms personnels sujets (toutes personnes) : un « ce » entre l'un d'eux et
/// un verbe conjugué est le réfléchi « se ».
const SUBJECT_PRONOUNS: &[&str] = &[
    "je", "j", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles",
];

/// Formes singulières d'« être » (auxiliaire) qui suivent l'élision « c' »/« s' ».
const AUX_SINGULAR: &[&str] = &["est", "était"];

/// Relatifs derrière lesquels « se » est en réalité le démonstratif « ce »
/// (« se que tu dis » → « ce que… ») ; formes élidées normalisées (`qu'` → `qu`).
const RELATIVES_AFTER_CE: &[&str] = &["que", "qu", "qui", "dont"];

/// Vrai si le jeton est une élision (se termine par une apostrophe).
fn is_elided(text: &str) -> bool {
    text.ends_with('\'') || text.ends_with('\u{2019}')
}

/// Vrai si `form` admet une analyse verbale **finie** (forme conjuguée).
fn is_finite_verb(form: &str) -> bool {
    !morpho::verb_forms(form).is_empty()
}

/// Vrai si `form` admet une analyse de **participe passé** (verbe sans personne,
/// porteur d'un genre ou d'un nombre : « levé », « trompée », « venus »).
fn is_past_participle(form: &str) -> bool {
    morpho::lookup(form).iter().any(|m| {
        m.category == morpho::MorphCategory::Verb
            && m.person.is_none()
            && (m.gender.is_some() || m.number.is_some())
    })
}

/// Catégorie POS du jeton lexical à la position `k` de la phrase.
fn upos(sentence: &[(usize, &Token)], k: usize, tags: &[Tagged]) -> Upos {
    tags[sentence[k].0].upos
}

/// Position du sujet candidat à gauche de `i`, en sautant la négation « ne »/« n' ».
/// Renvoie `None` si l'on atteint le début de la phrase.
fn subject_before(sentence: &[(usize, &Token)], i: usize) -> Option<usize> {
    let mut k = i;
    while k > 0 {
        k -= 1;
        match normalize(sentence[k].1.text.as_str()).as_str() {
            "ne" | "n" => continue,
            _ => return Some(k),
        }
    }
    None
}

/// Vrai si une ponctuation (virgule, tiret…) sépare les jetons d'origine `a` et
/// `b` (a < b) dans la tranche complète : on refuse alors le rattachement
/// sujet → clitique (« Lui, c'est différent »).
fn punctuation_between(tokens: &[Token], a: usize, b: usize) -> bool {
    tokens[a + 1..b]
        .iter()
        .any(|t| t.kind == TokenKind::Punctuation)
}

/// Cherche un participe passé à droite de la position `i` (le « est »), en sautant
/// les adverbes (« s'est bien passé », « s'est enfin levé »).
fn participle_follows(sentence: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> bool {
    let mut k = i + 1;
    while let Some((idx, tok)) = sentence.get(k) {
        if tags[*idx].upos == Upos::Adv {
            k += 1;
            continue;
        }
        return is_past_participle(&tok.text);
    }
    false
}

/// Correction d'un « ce »/« se » (mot plein), d'après son voisinage tagué.
fn correction_word(
    sentence: &[(usize, &Token)],
    i: usize,
    tags: &[Tagged],
) -> Option<&'static str> {
    match normalize(sentence[i].1.text.as_str()).as_str() {
        // ce → se : sujet (pronom, « ne » sauté) + ce + verbe conjugué.
        "ce" => {
            let subj = subject_before(sentence, i)?;
            let is_subj =
                SUBJECT_PRONOUNS.contains(&normalize(sentence[subj].1.text.as_str()).as_str());
            let next_is_verb = sentence
                .get(i + 1)
                .is_some_and(|(_, t)| is_finite_verb(&t.text));
            (is_subj && next_is_verb).then_some("se")
        }

        // se → ce : se devant un nom/nom propre/adjectif, ou un relatif que/qui/dont.
        "se" => {
            let next = sentence.get(i + 1)?;
            let next_norm = normalize(next.1.text.as_str());
            let next_is_nominal = matches!(
                upos(sentence, i + 1, tags),
                Upos::Noun | Upos::Propn | Upos::Adj
            );
            (next_is_nominal || RELATIVES_AFTER_CE.contains(&next_norm.as_str())).then_some("ce")
        }

        _ => None,
    }
}

/// Correction de l'élision « c' »/« s' » devant « est »/« était ».
fn correction_clitic(
    sentence: &[(usize, &Token)],
    tokens: &[Token],
    i: usize,
    tags: &[Tagged],
) -> Option<&'static str> {
    let cur = sentence[i].1;
    if !is_elided(&cur.text) {
        return None;
    }
    // Doit être immédiatement suivi de l'auxiliaire « est »/« était ».
    let aux = sentence.get(i + 1)?;
    if !AUX_SINGULAR.contains(&normalize(aux.1.text.as_str()).as_str()) {
        return None;
    }
    let pp_follows = participle_follows(sentence, i + 1, tags);

    match normalize(cur.text.as_str()).as_str() {
        // c' → s' : sujet de 3ᵉ personne (sans virgule) + c'est + participe passé.
        "c" if pp_follows => {
            let subj = subject_before(sentence, i)?;
            if punctuation_between(tokens, sentence[subj].0, sentence[i].0) {
                return None;
            }
            let subj_norm = normalize(sentence[subj].1.text.as_str());
            let third_person = matches!(subj_norm.as_str(), "il" | "elle" | "on" | "qui")
                || matches!(upos(sentence, subj, tags), Upos::Noun | Upos::Propn);
            third_person.then_some("s'")
        }

        // s' → c' : « s'est » non suivi d'un participe passé → « c'est ».
        "s" if !pp_follows => Some("c'"),

        _ => None,
    }
}

/// Construit la suggestion de correction d'un jeton vers sa forme corrigée.
fn suggestion(token: &Token, corrected: &str) -> Suggestion {
    Suggestion {
        span: token.span,
        message: format!(
            "Confusion « {} »/« {} » : « {} » devrait être « {} ».",
            token.text,
            corrected.trim_end_matches(['\'', '\u{2019}']),
            token.text,
            corrected
        ),
        replacements: vec![match_case(&token.text, corrected)],
        rule_id: RULE_ID,
    }
}

impl Rule for CeSeConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        // Intrinsèquement adossée au POS : sans tags, on les calcule.
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let corrected = correction_word(&sentence, i, tags)
                    .or_else(|| correction_clitic(&sentence, tokens, i, tags));
                if let Some(c) = corrected {
                    suggestions.push(suggestion(sentence[i].1, c));
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « ce »/« se », « c'est »/« s'est »"
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
        CeSeConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        CeSeConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- ce → se ---

    #[test]
    fn ce_to_se_between_subject_and_verb() {
        assert_eq!(first("il ce lève").as_deref(), Some("se"));
        assert_eq!(first("elle ce demande pourquoi").as_deref(), Some("se"));
        assert_eq!(first("il ne ce lève pas").as_deref(), Some("se"));
    }

    // --- se → ce ---

    #[test]
    fn se_to_ce_before_noun_or_adjective() {
        assert_eq!(first("il aime se chien").as_deref(), Some("ce"));
        assert_eq!(first("se petit chat dort").as_deref(), Some("ce"));
    }

    #[test]
    fn se_to_ce_before_relative() {
        assert_eq!(first("se que tu dis est faux").as_deref(), Some("ce"));
        assert_eq!(first("je sais se qui se passe").as_deref(), Some("ce"));
    }

    #[test]
    fn se_to_ce_preserves_case() {
        assert_eq!(first("Se petit chat dort").as_deref(), Some("Ce"));
    }

    // --- c' → s' ---

    #[test]
    fn c_to_s_after_subject_with_participle() {
        assert_eq!(first("il c'est trompé").as_deref(), Some("s'"));
        assert_eq!(first("elle c'est levée tôt").as_deref(), Some("s'"));
        assert_eq!(first("le vase c'est cassé").as_deref(), Some("s'"));
        assert_eq!(first("il c'était endormi").as_deref(), Some("s'"));
    }

    #[test]
    fn c_to_s_keeps_lowercase_clitic() {
        // Le clitique « c' » reste minuscule en milieu de phrase, même après une
        // majuscule de début de phrase portée par le sujet « Il ».
        assert_eq!(first("Il c'est trompé").as_deref(), Some("s'"));
    }

    // --- s' → c' ---

    #[test]
    fn s_to_c_without_participle() {
        assert_eq!(first("s'est magnifique").as_deref(), Some("c'"));
        assert_eq!(first("s'est un beau jour").as_deref(), Some("c'"));
        assert_eq!(first("s'est vraiment dommage").as_deref(), Some("c'"));
        // En tête de phrase, la majuscule du clitique est calquée.
        assert_eq!(first("S'est magnifique").as_deref(), Some("C'"));
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "il se lève",               // « se » réfléchi correct
            "il se livre à la lecture", // « se » réfléchi (homographe livre)
            "ce livre est lourd",       // « ce » démonstratif
            "je lis ce livre",          // « ce » après un verbe
            "il s'est trompé",          // « s'est » + participe
            "elle s'est bien amusée",   // « s'est » + adverbe + participe
            "c'est magnifique",         // « c'est » démonstratif
            "et c'est tant mieux",      // « c'est » après coordination
            "je pense que c'est juste", // « c'est » après « que »
            "Lui, c'est différent",     // virgule : pas de rattachement sujet
            "ce sont mes amis",         // « ce sont » (clitique non élidé)
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn ces_ses_is_a_known_gap() {
        // Limite **structurelle** : « ces » (démonstratif) et « ses » (possessif)
        // sont deux déterminants également valides devant un nom ; aucune règle
        // grammaticale ne les sépare. On ne signale donc rien.
        assert_eq!(count("ces livres sont neufs"), 0);
        assert_eq!(count("ses amis sont venus"), 0);
        assert_eq!(count("il range ses affaires"), 0);
    }

    #[test]
    fn no_cross_sentence_leak() {
        // « il » (phrase 1) ne doit pas servir de sujet au « c'est » de la phrase 2.
        assert_eq!(count("il dort. c'est fini"), 0);
    }
}
