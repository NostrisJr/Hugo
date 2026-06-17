//! Règle : confusion **quel(s)/quelle(s) / qu'elle(s)** — **tranche 4** du moteur
//! de confusions de la phase 6 (cf.
//! [`corpus/confusion-quel-quelle.md`](../../../../../corpus/confusion-quel-quelle.md)).
//!
//! Adossée au CRF ([`crate::pos`]). Mémo (Projet Voltaire) :
//! - **quel/quelle/quels/quelles** = adjectif interrogatif ou exclamatif,
//!   **accordé avec un nom** (« **quelle** heure ? », « **quels** beaux jours ! ») ;
//! - **qu'elle(s)** = « que/qu' » + pronom « elle(s) », **suivi d'un verbe**
//!   (« je crois **qu'elle** vient », « pour **qu'elles** partent »).
//!
//! ## qu'elle(s) → quel(le)(s) (« qu'elle heure » → « **quelle** heure »)
//!
//! Un pronom « elle(s) » n'est jamais immédiatement suivi d'un **nom** (ni d'un
//! **adjectif**) : « qu'elle » + groupe nominal est l'adjectif interrogatif
//! « quel », à accorder avec ce nom. On fusionne « qu' elle(s) » en la forme
//! accordée (genre du nom/adjectif, nombre d'elle/elles).
//!
//! ## quelle(s) → qu'elle(s) (« je crois quelle vient » → « qu'elle vient »)
//!
//! « quelle »/« quelles » (formes féminines, homophones de « qu'elle(s) ») suivi
//! d'un **verbe conjugué** (clitiques « ne/se/… » sautés) qui **n'est pas** une
//! forme d'*être*/*avoir* est « qu'elle(s) » : l'adjectif interrogatif appelle un
//! nom, pas un verbe. Les formes d'*être*/*avoir* sont **écartées** car
//! « quelle est/a été… » est aussi l'interrogatif (« quelle est la réponse »),
//! ambiguïté **structurelle**.
//!
//! Limites assumées : les formes masculines « quel/quels » → « qu'il(s) » relèvent
//! d'une autre famille (hors tranche 4) ; « qu'elle est/a… » (être/avoir) n'a pas
//! de signal séparable.

use super::{is_finite_verb, is_infinitive, match_case, normalize};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::Tagged;
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::{Span, Suggestion};

/// Détecte les confusions « quel(s)/quelle(s) » / « qu'elle(s) ».
pub struct QuelConfusion;

const RULE_ID: &str = "confusion_quel";

/// Clitiques préverbaux sautés entre « quelle » et le verbe (« quelle ne vient
/// pas », « quelle se trompe »). Formes élidées normalisées (`s'` → `s`).
const PREVERBAL_CLITICS: &[&str] = &[
    "ne", "n", "se", "s", "me", "m", "te", "t", "le", "la", "l", "les", "lui", "leur", "y", "en",
    "nous", "vous",
];

/// Idiomes « tel/telle/tels/telles quel(le)(s) » à ne pas corriger.
const TEL_FORMS: &[&str] = &["tel", "telle", "tels", "telles"];

/// Vrai si le jeton est une élision (se termine par une apostrophe).
fn is_elided(text: &str) -> bool {
    text.ends_with('\'') || text.ends_with('\u{2019}')
}

/// Vrai si `form` est une tête **nominale** (nom/adjectif) qui n'est pas aussi un
/// verbe. À la Grammalecte, on consulte les lectures possibles du lexique plutôt
/// que l'unique tag CRF, leurré par l'erreur (« qu'elle heure » fait étiqueter
/// « heure » VERB). Écarte le « qu'elle » légitime suivi d'un verbe (« qu'elle
/// vient », « qu'elle peine à finir »).
fn is_nominal_head(form: &str) -> bool {
    let has_nominal = morpho::lookup(form)
        .iter()
        .any(|m| matches!(m.category, MorphCategory::Noun | MorphCategory::Adjective));
    has_nominal && !is_finite_verb(form) && !is_infinitive(form)
}

/// Genre de la tête nominale/adjectivale : masculin si le lexique en donne une
/// lecture masculine **sans** lecture féminine, sinon féminin (défaut, cohérent
/// avec le « elle(s) » écrit).
fn head_gender(form: &str) -> Gender {
    let morphs = morpho::lookup(form);
    let nominal =
        |m: &morpho::Morph| matches!(m.category, MorphCategory::Noun | MorphCategory::Adjective);
    let masc = morphs
        .iter()
        .any(|m| nominal(m) && m.gender == Some(Gender::Masculine));
    let fem = morphs
        .iter()
        .any(|m| nominal(m) && m.gender == Some(Gender::Feminine));
    if masc && !fem {
        Gender::Masculine
    } else {
        Gender::Feminine
    }
}

/// Forme « quel » accordée en genre et nombre (le genre vaut masculin ou
/// féminin ; tout autre cas est traité comme féminin, cohérent avec « elle »).
fn quel_form(gender: Gender, number: Number) -> &'static str {
    let plural = number == Number::Plural;
    match gender {
        Gender::Masculine => {
            if plural {
                "quels"
            } else {
                "quel"
            }
        }
        _ => {
            if plural {
                "quelles"
            } else {
                "quelle"
            }
        }
    }
}

/// Vrai si `form` admet une forme finie d'*être* ou d'*avoir* (auxiliaires).
fn is_etre_avoir(form: &str) -> bool {
    morpho::verb_forms(form)
        .iter()
        .any(|v| v.lemma == "être" || v.lemma == "avoir")
}

/// Correction « qu'elle(s) » → « quel(le)(s) » : élision « qu' » + elle/elles +
/// nom/adjectif. Fusionne les deux jetons en la forme accordée.
fn correction_elision(sentence: &[(usize, &Token)], i: usize) -> Option<Suggestion> {
    let cur = sentence[i].1;
    if !is_elided(&cur.text) || normalize(&cur.text) != "qu" {
        return None;
    }
    let (_, pron) = sentence.get(i + 1)?;
    let number = match normalize(&pron.text).as_str() {
        "elle" => Number::Singular,
        "elles" => Number::Plural,
        _ => return None,
    };
    // La tête (nom ou adjectif) qui suit immédiatement « elle(s) » : un pronom
    // n'est jamais suivi d'un groupe nominal.
    let (_, head) = sentence.get(i + 2)?;
    if !is_nominal_head(&head.text) {
        return None;
    }
    let corrected = quel_form(head_gender(&head.text), number);
    let span = Span::new(cur.span.start, pron.span.end);
    let original = format!("{}{}", cur.text, pron.text);
    Some(Suggestion {
        span,
        message: format!(
            "Confusion « quel »/« qu'elle » : « {original} » devrait être « {corrected} »."
        ),
        replacements: vec![match_case(&original, corrected)],
        rule_id: RULE_ID,
    })
}

/// Correction « quelle(s) » → « qu'elle(s) » : forme féminine + verbe conjugué
/// (non *être*/*avoir*), clitiques sautés.
fn correction_word(sentence: &[(usize, &Token)], i: usize) -> Option<Suggestion> {
    let cur = sentence[i].1;
    let corrected = match normalize(&cur.text).as_str() {
        "quelle" => "qu'elle",
        "quelles" => "qu'elles",
        _ => return None,
    };
    // Idiome « telle quelle » : ne pas corriger.
    if i > 0 && TEL_FORMS.contains(&normalize(sentence[i - 1].1.text.as_str()).as_str()) {
        return None;
    }
    // Premier jeton non clitique à droite : il doit être un verbe conjugué qui
    // n'est ni « être » ni « avoir ».
    let mut k = i + 1;
    while let Some((_, t)) = sentence.get(k) {
        if PREVERBAL_CLITICS.contains(&normalize(&t.text).as_str()) {
            k += 1;
            continue;
        }
        break;
    }
    let (_, verb) = sentence.get(k)?;
    if !super::is_finite_verb(&verb.text) || is_etre_avoir(&verb.text) {
        return None;
    }
    Some(Suggestion {
        span: cur.span,
        message: format!(
            "Confusion « quel »/« qu'elle » : « {} » devrait être « {corrected} ».",
            cur.text
        ),
        replacements: vec![match_case(&cur.text, corrected)],
        rule_id: RULE_ID,
    })
}

impl Rule for QuelConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], _tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            for i in 0..sentence.len() {
                let s = correction_elision(&sentence, i).or_else(|| correction_word(&sentence, i));
                if let Some(s) = s {
                    suggestions.push(s);
                }
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « quel/quelle » / « qu'elle »"
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
        QuelConfusion
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        QuelConfusion.check_tagged(&tokens, &tags).len()
    }

    // --- qu'elle(s) → quel(le)(s) ---

    #[test]
    fn elision_to_quel_agrees_with_noun() {
        assert_eq!(
            first("je me demande qu'elle heure il est").as_deref(),
            Some("quelle")
        );
        assert_eq!(first("qu'elle joie de te voir").as_deref(), Some("quelle"));
        assert_eq!(first("qu'elles belles fleurs").as_deref(), Some("quelles"));
    }

    #[test]
    fn elision_to_quel_masculine_noun() {
        assert_eq!(first("qu'elle homme courageux").as_deref(), Some("quel"));
        assert_eq!(first("qu'elles beaux jardins").as_deref(), Some("quels"));
    }

    // --- quelle(s) → qu'elle(s) ---

    #[test]
    fn word_to_elision_before_verb() {
        assert_eq!(
            first("je crois quelle vient demain").as_deref(),
            Some("qu'elle")
        );
        assert_eq!(
            first("il faut quelles partent vite").as_deref(),
            Some("qu'elles")
        );
        assert_eq!(
            first("je pense quelle ne vient pas").as_deref(),
            Some("qu'elle")
        );
        assert_eq!(
            first("je sais quelle se trompe").as_deref(),
            Some("qu'elle")
        );
    }

    // --- pas de faux positif ---

    #[test]
    fn correct_usages_yield_nothing() {
        for ok in [
            "quelle heure est-il",       // interrogatif + nom
            "quelle belle maison",       // exclamatif + adjectif + nom
            "quel livre lis-tu",         // interrogatif masculin
            "je crois qu'elle vient",    // « qu'elle » correct + verbe
            "pour qu'elles partent",     // « qu'elles » correct
            "quelle est la réponse",     // interrogatif « quelle est » (gap être)
            "quelle a été ta surprise",  // interrogatif « quelle a » (gap avoir)
            "il le prend tel quel",      // idiome « tel quel »
            "elle le rend telle quelle", // idiome « telle quelle »
            "quelle fut sa joie",        // exclamatif (être, sans participe)
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(first("Quelle vient demain").as_deref(), Some("Qu'elle"));
        assert_eq!(first("Qu'elle heure il est").as_deref(), Some("Quelle"));
    }
}
