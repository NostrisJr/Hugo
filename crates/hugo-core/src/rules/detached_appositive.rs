//! Règle : accord de l'adjectif ou participe passé apposé avec le sujet postposé.
//!
//! On traite la construction où un adjectif ou un participe passé, encadré par
//! des virgules, précède un verbe dont le **sujet est postposé** (inversion
//! stylistique) :
//!
//! > « Au bord du lac, endormi à l'ombre des arbres, patientaient les soldats. »
//!
//! Ici « endormi » s'accorde avec « soldats » (Masc Pl) → « endormis ».
//!
//! ## Motif
//!
//! `[,] [ADJ/PP_candidat] … [,] [VERB_fini] … [DET?] [NOUN]`
//!
//! 1. Le **premier token lexical** entre deux virgules consécutives doit être
//!    étiqueté `ADJ` (ou `VERB` par le CRF) **et** avoir une lecture adjectivale
//!    ou participiale dans le lexique.
//! 2. Après la deuxième virgule, le premier **verbe fini** est identifié.
//! 3. Après ce verbe, le premier nom (précédé ou non d'un déterminant) est
//!    retenu comme **sujet postposé**.
//! 4. Le genre et le nombre du nom sont récupérés dans le lexique ; si le genre
//!    est inconnu (nom épicène), on retient le masculin grammatical par défaut.
//! 5. Si l'adjectif/participe ne s'accorde pas déjà, on génère la forme correcte
//!    via [`morpho::decline`] (adjectifs) ou [`morpho::participle`] (participes).
//!
//! ## Anti-faux-positifs
//!
//! - L'apposé doit être le **premier token lexical** entre les virgules, pour
//!   écarter les compléments et les syntagmes non adjectivaux.
//! - Le tag POS doit être `ADJ` ou `VERB` (pas `NOUN`, `ADP`, `PRON`…).
//! - Si le sujet est ambigu ou absent de la fenêtre, on n'émet rien (précision
//!   > rappel).
//! - Si la forme correcte est introuvable dans le lexique, on n'émet rien.

use super::Rule;
use crate::dep::DepRel;
use crate::morpho::{self, Gender, Morph, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Règle d'accord de l'apposition détachée avec le sujet postposé.
pub struct DetachedAppositive;

const RULE_ID: &str = "detached_appositive";

// ---------------------------------------------------------------------------
// Utilitaires morphologiques
// ---------------------------------------------------------------------------

fn is_participle(m: &Morph) -> bool {
    m.category == MorphCategory::Verb
        && m.person.is_none()
        && m.gender.is_some()
        && m.number.is_some()
}

fn unique_lemma<'a>(analyses: &[&'a Morph]) -> Option<&'a str> {
    let mut it = analyses.iter().map(|m| m.lemma.as_str());
    let first = it.next()?;
    if it.all(|l| l == first) {
        Some(first)
    } else {
        None
    }
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

// ---------------------------------------------------------------------------
// Gardes sur l'apposé
// ---------------------------------------------------------------------------

/// Vrai si le token (avec son tag POS) est un candidat apposition détachée.
///
/// Critères : tag POS = `ADJ` ou `VERB` (le CRF peut étiqueter un PP comme
/// `VERB`), **et** le lexique confirme une lecture adjectivale ou participiale.
fn is_appositive_candidate(tok: &Token, upos: Upos) -> bool {
    if !matches!(upos, Upos::Adj | Upos::Verb) {
        return false;
    }
    morpho::lookup(&tok.text)
        .iter()
        .any(|m| m.category == MorphCategory::Adjective || is_participle(m))
}

// ---------------------------------------------------------------------------
// Recherche du verbe fini et du sujet postposé
// ---------------------------------------------------------------------------

/// Cherche le premier verbe fini dans `tokens[from..end]`.
fn find_finite_verb(tokens: &[Token], tags: &[Tagged], from: usize, end: usize) -> Option<usize> {
    (from..end).find(|&i| {
        tokens[i].is_lexical()
            && matches!(tags[i].upos, Upos::Verb | Upos::Aux)
            && (!morpho::verb_forms(&tokens[i].text).is_empty()
                || morpho::lookup(&tokens[i].text)
                    .iter()
                    .any(|m| m.category == MorphCategory::Verb && m.person.is_some()))
    })
}

/// Remonte la plage `tokens[from..verb_idx]` à la recherche du **sujet
/// préposé** du verbe : nom, nom propre ou pronom personnel sujet non régi
/// par une préposition ou un autre verbe.
///
/// Retourne l'index du token tête si un sujet préposé est trouvé, `None`
/// sinon.  Une préposition (`ADP`) ou un verbe interrompt la remontée : le nom
/// qui les précède est un complément, pas le sujet du verbe cible.
fn find_preposed_subject(tokens: &[Token], tags: &[Tagged], from: usize, verb_idx: usize) -> Option<usize> {
    if from >= verb_idx {
        return None;
    }
    for i in (from..verb_idx).rev() {
        if !tokens[i].is_lexical() {
            continue;
        }
        match tags[i].upos {
            Upos::Noun | Upos::Propn => return Some(i),
            Upos::Pron => {
                let lower = tokens[i].text.to_lowercase();
                let s = lower.trim_end_matches(['\'', '\u{2019}']);
                if matches!(
                    s,
                    "je" | "j" | "tu" | "il" | "elle" | "on" | "nous" | "vous" | "ils" | "elles"
                ) {
                    return Some(i);
                }
                // Clitique (se, me, te, en, y…) : continuer à remonter.
            }
            Upos::Adp | Upos::Verb | Upos::Aux => return None,
            _ => {}
        }
    }
    None
}

/// Genre et nombre d'un token sujet (pronom à genre connu ou nom commun).
fn subject_gender_number(token: &Token) -> Option<(Gender, Number)> {
    let lower = token.text.to_lowercase();
    let s = lower.trim_end_matches(['\'', '\u{2019}']);
    match s {
        "il" => return Some((Gender::Masculine, Number::Singular)),
        "elle" => return Some((Gender::Feminine, Number::Singular)),
        "ils" => return Some((Gender::Masculine, Number::Plural)),
        "elles" => return Some((Gender::Feminine, Number::Plural)),
        _ => {}
    }
    noun_features(&token.text)
}

/// Cherche la tête nominale du **sujet postposé** dans `tokens[from..end]`.
///
/// Saute les déterminants, adjectifs et adverbes. S'arrête au premier nom ou
/// nom propre. Abandonne sur un verbe ou une préposition.
fn find_inverted_subject(tokens: &[Token], tags: &[Tagged], from: usize, end: usize) -> Option<usize> {
    for i in from..end {
        if !tokens[i].is_lexical() {
            continue;
        }
        match tags[i].upos {
            Upos::Det | Upos::Adj | Upos::Adv => {}
            Upos::Noun | Upos::Propn => return Some(i),
            _ => return None,
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Genre et nombre du nom sujet
// ---------------------------------------------------------------------------

/// Genre et nombre du nom `text` d'après le lexique.
///
/// - Nombre : consensus requis entre les lectures nominales.
/// - Genre : consensus si disponible ; masculin grammatical par défaut pour les
///   noms épicènes ou sans genre enregistré (convention française pour les
///   groupes de genre inconnu ou mixte).
/// - Retourne `None` si le nombre est ambigu ou si le token n'est pas un nom.
fn noun_features(text: &str) -> Option<(Gender, Number)> {
    let analyses = morpho::lookup(text);
    let nouns: Vec<_> = analyses
        .iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if nouns.is_empty() {
        return None;
    }

    // Nombre : consensus requis.
    let mut number: Option<Number> = None;
    for m in &nouns {
        if let Some(n) = m.number {
            if n == Number::Invariable {
                continue;
            }
            match number {
                None => number = Some(n),
                Some(prev) if prev == n => {}
                Some(_) => return None, // contradiction
            }
        }
    }
    let number = number?;

    // Genre : consensus ou masculin par défaut.
    let mut gender: Option<Gender> = None;
    let mut contradictory = false;
    for m in &nouns {
        if let Some(g) = m.gender {
            if g == Gender::Epicene {
                continue;
            }
            match gender {
                None => gender = Some(g),
                Some(prev) if prev == g => {}
                Some(_) => {
                    contradictory = true;
                    break;
                }
            }
        }
    }
    if contradictory {
        return None;
    }
    Some((gender.unwrap_or(Gender::Masculine), number))
}

// ---------------------------------------------------------------------------
// Accord et génération de la forme corrigée
// ---------------------------------------------------------------------------

/// Vrai si l'adjectif/participe `tok` s'accorde déjà avec (gender, number).
fn agrees(tok: &Token, gender: Gender, number: Number) -> bool {
    morpho::lookup(&tok.text).iter().any(|m| {
        if m.category != MorphCategory::Adjective && !is_participle(m) {
            return false;
        }
        let g_ok = m.gender.map_or(true, |g| g == gender || g == Gender::Epicene);
        let n_ok = m.number.map_or(true, |n| n == number || n == Number::Invariable);
        g_ok && n_ok
    })
}

/// Génère la forme correcte pour (gender, number).
///
/// Essaie d'abord via [`morpho::decline`] (adjectif), puis via
/// [`morpho::participle`] (participe passé).
fn generate_form(tok: &Token, gender: Gender, number: Number) -> Option<String> {
    let analyses = morpho::lookup(&tok.text);
    let adjs: Vec<&Morph> = analyses
        .iter()
        .filter(|m| m.category == MorphCategory::Adjective)
        .collect();
    let parts: Vec<&Morph> = analyses.iter().filter(|m| is_participle(m)).collect();

    unique_lemma(&adjs)
        .and_then(|l| morpho::decline(l, gender, number))
        .or_else(|| unique_lemma(&parts).and_then(|l| morpho::participle(l, gender, number)))
}

// ---------------------------------------------------------------------------
// Ponctuation utilitaire
// ---------------------------------------------------------------------------

fn is_sentence_terminator(tok: &Token) -> bool {
    tok.kind == TokenKind::Punctuation
        && matches!(tok.text.as_str(), "." | "!" | "?" | "…" | ";" | ":")
}

fn is_comma(tok: &Token) -> bool {
    tok.kind == TokenKind::Punctuation && tok.text == ","
}

/// Vrai si le token `i` est enchâssé dans une **proposition subordonnée**
/// adverbiale ou complétive plus profonde que la principale : un de ses
/// gouverneurs (en **excluant** `i` lui-même) porte la relation `advcl`, `ccomp`
/// ou `csubj`.
///
/// Remplace l'ancienne garde SCONJ positionnelle : un apposé enchâssé dans une
/// subordonnée (« Si un SI peut sembler imbriqué, **perclus**…, … se dégagent » :
/// perclus → imbriqué → sembler → peut[advcl]) ne doit pas s'accorder avec le
/// sujet de la principale.
///
/// On exclut le token lui-même : un apposé détaché en tête de phrase est souvent
/// étiqueté `advcl` de la **principale** (« Épuisé…, s'allongea la voyageuse »)
/// tout en s'accordant légitimement avec le sujet (inversé) de cette principale.
/// On ne classe pas non plus `acl` comme bloquant (proposition adjective d'un
/// nom : « tapie sous un rocher »).
fn in_subordinate_clause(tags: &[Tagged], i: usize) -> bool {
    let mut cur = match crate::dep::head_of(tags, i) {
        Some(h) => h,
        None => return false,
    };
    for _ in 0..tags.len() {
        if matches!(
            tags[cur].dep,
            DepRel::Advcl | DepRel::Ccomp | DepRel::Csubj
        ) {
            return true;
        }
        match crate::dep::head_of(tags, cur) {
            Some(h) => cur = h,
            None => break,
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Cœur de la règle
// ---------------------------------------------------------------------------

impl DetachedAppositive {
    fn scan(tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let mut sent_start = 0usize;

        for i in 0..tokens.len() {
            if is_sentence_terminator(&tokens[i]) {
                Self::check_range(tokens, tags, sent_start, i, &mut suggestions);
                sent_start = i + 1;
            }
        }
        Self::check_range(tokens, tags, sent_start, tokens.len(), &mut suggestions);

        suggestions
    }

    /// Analyse la tranche `tokens[start..end]` à la recherche de deux motifs :
    ///
    /// - **Motif 1** : `[,] apposé [,] verbe-fini sujet-postposé`
    ///   (apposé encadré par deux virgules).
    /// - **Motif 2** : `[début] apposé [,] verbe-fini sujet-postposé`
    ///   (apposé en tête de phrase, avant la première virgule).
    fn check_range(
        tokens: &[Token],
        tags: &[Tagged],
        start: usize,
        end: usize,
        suggestions: &mut Vec<Suggestion>,
    ) {
        let commas: Vec<usize> = (start..end)
            .filter(|&i| is_comma(&tokens[i]))
            .collect();

        // Motif 1 : apposé entre deux virgules consécutives.
        for window in commas.windows(2) {
            let c1 = window[0];
            let c2 = window[1];

            let Some(app_idx) = (c1 + 1..c2).find(|&i| tokens[i].is_lexical()) else {
                continue;
            };
            if !is_appositive_candidate(&tokens[app_idx], tags[app_idx].upos) {
                continue;
            }
            // Garde subordonnée (par l'arbre) : un apposé enchâssé dans une
            // proposition subordonnée n'accorde pas avec le sujet de la
            // principale. Remplace l'ancienne garde SCONJ de surface.
            if in_subordinate_clause(tags, app_idx) {
                continue;
            }
            // Garde « proposition antérieure » : si une proposition **finie**
            // précède l'apposé dans la phrase (« Si un SI peut sembler imbriqué,
            // perclus…, … se dégagent »), l'apposé prolonge cette proposition et
            // ne s'accorde pas avec le sujet qui suit. Une amorce non finie
            // (groupe prépositionnel « Dans la brume…, ») ne bloque pas. Robuste
            // à la variation d'arbre (le parser peut enraciner l'apposé lui-même).
            if find_finite_verb(tokens, tags, start, c1).is_some() {
                continue;
            }
            let sugg = Self::try_inverted(tokens, tags, app_idx, c2 + 1, end)
                .or_else(|| Self::try_direct(tokens, tags, app_idx, c2 + 1, end));
            if let Some(sugg) = sugg {
                suggestions.push(sugg);
            }
        }

        // Motif 2 : apposé en tête de phrase (premier token lexical avant la
        // première virgule). Écarte le cas où le premier token est un déterminant,
        // un pronom ou une préposition (ce n'est pas une apposition).
        if let Some(&first_comma) = commas.first() {
            if let Some(app_idx) = (start..first_comma).find(|&i| tokens[i].is_lexical()) {
                // Une apposition détachée est non-finie : « Épuisé par le voyage, ».
                // Si le segment avant la première virgule contient un verbe fini,
                // c'est une proposition complète (« Tout cela les amusait beaucoup, »)
                // avec son propre sujet — pas une apposition.
                let segment_is_clause =
                    find_finite_verb(tokens, tags, app_idx, first_comma).is_some();
                if !segment_is_clause
                    && is_appositive_candidate(&tokens[app_idx], tags[app_idx].upos)
                    && !in_subordinate_clause(tags, app_idx)
                {
                    let sugg =
                        Self::try_inverted(tokens, tags, app_idx, first_comma + 1, end)
                        .or_else(|| Self::try_direct(tokens, tags, app_idx, first_comma + 1, end));
                    if let Some(sugg) = sugg {
                        suggestions.push(sugg);
                    }
                }
            }
        }
    }

    /// Motif **sujet postposé** : `[apposé ,] VERB NP[sujet]`.
    ///
    /// Ne se déclenche que si aucun sujet préposé n'est détecté entre
    /// `search_from` et le verbe. Renvoie `None` si le verbe a déjà un sujet
    /// devant lui — `try_direct` prend alors le relais.
    fn try_inverted(
        tokens: &[Token],
        tags: &[Tagged],
        app_idx: usize,
        search_from: usize,
        end: usize,
    ) -> Option<Suggestion> {
        let verb_idx = find_finite_verb(tokens, tags, search_from, end)?;

        // Sujet préposé présent → le nom après le verbe est un COD, pas le
        // sujet inversé. On délègue à try_direct.
        if find_preposed_subject(tokens, tags, search_from, verb_idx).is_some() {
            return None;
        }

        let noun_idx = find_inverted_subject(tokens, tags, verb_idx + 1, end)?;
        let (gender, number) = noun_features(&tokens[noun_idx].text)?;

        if agrees(&tokens[app_idx], gender, number) {
            return None;
        }

        let corrected = generate_form(&tokens[app_idx], gender, number)?;
        if corrected.eq_ignore_ascii_case(&tokens[app_idx].text) {
            return None;
        }

        Some(Suggestion {
            span: tokens[app_idx].span,
            message: format!(
                "Accord de l'apposition détachée : « {} » doit s'accorder avec « {} ».",
                tokens[app_idx].text,
                tokens[noun_idx].text,
            ),
            replacements: vec![match_case(&tokens[app_idx].text, &corrected)],
            rule_id: RULE_ID,
        })
    }

    /// Motif **sujet préposé** : `[apposé ,] NP[sujet] VERB`.
    ///
    /// Se déclenche uniquement quand un sujet préposé est trouvé entre
    /// `search_from` et le verbe fini. L'apposé doit s'accorder avec ce sujet.
    fn try_direct(
        tokens: &[Token],
        tags: &[Tagged],
        app_idx: usize,
        search_from: usize,
        end: usize,
    ) -> Option<Suggestion> {
        let verb_idx = find_finite_verb(tokens, tags, search_from, end)?;
        let subj_idx = find_preposed_subject(tokens, tags, search_from, verb_idx)?;

        // Le sujet préposé doit former, avec l'apposé, une structure contiguë
        // « [apposé], [sujet] [verbe] ». Une virgule entre l'apposé et le sujet
        // signale que ce sujet appartient à une autre proposition :
        // « …construite, prête, puis écrasée au sol, l'homme l'accusa… » — « homme »
        // est le sujet de « accusa » (principale), pas de l'apposé « prête » (qui
        // relève du prédicat de « machine » en amont).
        if (search_from..subj_idx).any(|i| is_comma(&tokens[i])) {
            return None;
        }

        let (gender, number) = subject_gender_number(&tokens[subj_idx])?;

        if agrees(&tokens[app_idx], gender, number) {
            return None;
        }

        let corrected = generate_form(&tokens[app_idx], gender, number)?;
        if corrected.eq_ignore_ascii_case(&tokens[app_idx].text) {
            return None;
        }

        Some(Suggestion {
            span: tokens[app_idx].span,
            message: format!(
                "Accord de l'apposition détachée : « {} » doit s'accorder avec « {} ».",
                tokens[app_idx].text,
                tokens[subj_idx].text,
            ),
            replacements: vec![match_case(&tokens[app_idx].text, &corrected)],
            rule_id: RULE_ID,
        })
    }
}

impl Rule for DetachedAppositive {
    fn check(&self, _tokens: &[Token]) -> Vec<Suggestion> {
        Vec::new() // nécessite les tags POS
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        Self::scan(tokens, tags)
    }

    fn name(&self) -> &'static str {
        "Accord de l'apposition détachée avec sujet postposé"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn tagged_suggestions(text: &str) -> Vec<Suggestion> {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        DetachedAppositive.check_tagged(&tokens, &tags)
    }

    fn tagged_first(text: &str) -> Option<String> {
        tagged_suggestions(text)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn tagged_count(text: &str) -> usize {
        tagged_suggestions(text).len()
    }

    // --- Cas fautifs ---

    #[test]
    fn masc_sg_appose_masc_pl_subject() {
        // Participe Masc Sg → Masc Pl pour "soldats".
        assert_eq!(
            tagged_first(
                "Au bord du lac, endormi sous les arbres, patientaient les soldats."
            )
            .as_deref(),
            Some("endormis")
        );
    }

    #[test]
    fn masc_sg_appose_fem_sg_subject() {
        // Adjectif Masc Sg → Fém Sg pour "voyageuse".
        assert_eq!(
            tagged_first("Épuisé par le voyage, s'allongea la voyageuse.").as_deref(),
            Some("Épuisée")
        );
    }

    #[test]
    fn fem_sg_appose_masc_pl_subject() {
        // Adjectif Fém Sg → Masc Pl pour "soldats".
        assert_eq!(
            tagged_first("Heureuse de partir, coururent les soldats.").as_deref(),
            Some("Heureux")
        );
    }

    // --- Cas corrects (silence) ---

    #[test]
    fn already_correct_masc_pl() {
        assert_eq!(
            tagged_count("Épuisés par la route, s'arrêtèrent les soldats."),
            0
        );
    }

    #[test]
    fn already_correct_fem_sg() {
        assert_eq!(
            tagged_count("Épuisée par le voyage, s'allongea la voyageuse."),
            0
        );
    }

    // --- Non-déclenchement ---

    #[test]
    fn no_inverted_subject_no_trigger() {
        // Sujet avant le verbe (ordre direct) → pas de déclenchement.
        // "épuisé" s'accorde avec "le soldat" (Masc Sg) : déjà correct.
        assert_eq!(
            tagged_count("Le soldat, épuisé par la route, s'arrêta."),
            0
        );
    }

    #[test]
    fn noun_between_commas_no_trigger() {
        // Le premier token entre les virgules est un nom, pas un adj/PP.
        assert_eq!(tagged_count("Pierre, général de l'armée, repartit le matin."), 0);
    }

    #[test]
    fn non_inverted_subject_no_trigger() {
        // "il" précède le verbe → sujet non postposé → pas de déclenchement.
        assert_eq!(tagged_count("Épuisé, il s'arrêta."), 0);
    }

    #[test]
    fn sentence_no_verb_after_comma2_no_trigger() {
        // Pas de verbe après la deuxième virgule.
        assert_eq!(tagged_count("Épuisé par la route, fatigué."), 0);
    }

    #[test]
    fn preposed_subject_before_verb_no_trigger() {
        // Sujet préposé (« le renard ») + COD postposé (« les enfants ») :
        // le nom après le verbe n'est pas le sujet inversé → pas de déclenchement.
        assert_eq!(
            tagged_count(
                "Au bord du lac, endormi à l'ombre des arbres, le renard regardait les enfants."
            ),
            0
        );
    }

    #[test]
    fn inverted_subject_still_detected() {
        // Construction à sujet postposé : « enfants » est bien le sujet inversé.
        assert_eq!(
            tagged_first(
                "Au bord du lac, endormi à l'ombre des arbres, patientaient les enfants."
            )
            .as_deref(),
            Some("endormis")
        );
    }

    // --- Motif sujet préposé (try_direct). ---

    #[test]
    fn preposed_subject_appositive_is_corrected() {
        // Sujet préposé « le garçon » (Masc Sg) → « tapie » (Fém Sg) doit
        // devenir « tapi ».
        assert_eq!(
            tagged_first(
                "Dans la brume et dans le silence, tapie sous un rocher, le garçon attendait."
            )
            .as_deref(),
            Some("tapi")
        );
    }

    #[test]
    fn subordinate_clause_appositive_no_trigger() {
        // « perclus » est enchâssé dans la subordonnée « Si un SI peut sembler
        // imbriqué, perclus… » (gouverneur `advcl`) : il ne doit PAS s'accorder
        // avec « catégories », sujet de la principale. C'est la STRUCTURE
        // (chaîne de gouverneurs passant par advcl) qui l'écarte, plus une
        // détection SCONJ de surface.
        assert_eq!(
            tagged_count(
                "Si un SI peut sembler imbriqué, perclus de dépendances circulaires, \
                 différentes catégories se dégagent."
            ),
            0
        );
    }

    #[test]
    fn sentence_initial_clause_not_appositive() {
        // « Tout cela les amusait beaucoup, » est une proposition complète (verbe
        // fini « amusait »), pas une apposition détachée. « Tout » ne doit pas
        // s'accorder avec « ils ».
        assert_eq!(
            tagged_count(
                "Tout cela les amusait beaucoup, mais ce qu'ils préféraient, c'était la baignoire."
            ),
            0
        );
    }

    #[test]
    fn enumerated_predicative_not_appositive_subject() {
        // « prête » coordonne avec « construite » dans le prédicat de « machine » ;
        // « homme » (sujet de « accusa », après une virgule) ne doit pas être pris
        // pour le sujet de l'apposé.
        assert_eq!(
            tagged_count(
                "Mais dès que la machine fut construite, prête, puis écrasée au sol, \
                 l'homme l'accusa d'avoir mal fait son travail."
            ),
            0
        );
    }

    #[test]
    fn preposed_subject_already_correct_is_silent() {
        // « tapi sous un rocher, le garçon attendait » : déjà accordé.
        assert_eq!(
            tagged_count("Dans la brume, tapi sous un rocher, le garçon attendait."),
            0
        );
    }
}
