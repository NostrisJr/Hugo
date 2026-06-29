//! Règle : accord déterminant–nom (genre et nombre).
//!
//! Approche prudente, pensée pour minimiser les faux positifs :
//!
//! - on ne part que d'un **déterminant d'une classe fermée** connue (articles
//!   indéfinis/définis, démonstratifs) — les possessifs (`son`, `leur`…), trop
//!   homographes, sont écartés ;
//! - on cherche le **nom tête** en sautant les adjectifs antéposés ;
//! - on corrige **le déterminant** d'après le genre/nombre du nom (le genre
//!   d'un nom est inhérent), et on ne propose rien si ce genre est inconnu.
//!
//! Exemples : « un belle table » → « une », « les chat » → « le »,
//! « le chats » → « les ».

use super::{is_number_invariable, lexical_tokens, Rule};
use crate::dep::DepRel;
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Famille de déterminant (détermine les formes de correction).
#[derive(Clone, Copy)]
enum Family {
    Indefinite,
    Definite,
    Demonstrative,
    /// Article partitif / contracté avec *de* : du / de la / de l' / des.
    PartitiveDe,
    /// Article contracté avec *à* : au / à la / à l' / aux.
    ContractedA,
}

/// Contrainte de genre portée par un déterminant.
#[derive(Clone, Copy, PartialEq)]
enum GenderC {
    Masc,
    Fem,
    Any,
}

/// Description d'un déterminant connu.
struct DetInfo {
    family: Family,
    gender: GenderC,
    /// Nombre du déterminant (les déterminants de la table sont soit
    /// singuliers, soit pluriels).
    plural: bool,
}

/// Reconnaît un déterminant de la classe fermée traitée.
fn classify(form_lower: &str) -> Option<DetInfo> {
    use Family::*;
    use GenderC::*;
    let (family, gender, plural) = match form_lower {
        "un" => (Indefinite, Masc, false),
        "une" => (Indefinite, Fem, false),
        "des" => (Indefinite, Any, true),
        "le" => (Definite, Masc, false),
        "la" => (Definite, Fem, false),
        "les" => (Definite, Any, true),
        "ce" | "cet" => (Demonstrative, Masc, false),
        "cette" => (Demonstrative, Fem, false),
        "ces" => (Demonstrative, Any, true),
        // Articles contractés (un seul jeton ; le féminin se corrige en deux
        // mots : « du » → « de la », « au » → « à la »).
        "du" => (PartitiveDe, Masc, false),
        "au" => (ContractedA, Masc, false),
        "aux" => (ContractedA, Any, true),
        _ => return None,
    };
    Some(DetInfo {
        family,
        gender,
        plural,
    })
}

/// Forme correcte du déterminant pour un genre/nombre cibles.
///
/// Renvoie `None` au singulier quand le genre est indéterminé (impossible de
/// choisir entre « le » et « la »).
fn corrected_form(family: Family, gender: Option<Gender>, plural: bool) -> Option<&'static str> {
    if plural {
        return Some(match family {
            Family::Indefinite => "des",
            Family::Definite => "les",
            Family::Demonstrative => "ces",
            Family::PartitiveDe => "des",
            Family::ContractedA => "aux",
        });
    }
    match gender? {
        Gender::Masculine => Some(match family {
            Family::Indefinite => "un",
            Family::Definite => "le",
            Family::Demonstrative => "ce",
            Family::PartitiveDe => "du",
            Family::ContractedA => "au",
        }),
        Gender::Feminine => Some(match family {
            Family::Indefinite => "une",
            Family::Definite => "la",
            Family::Demonstrative => "cette",
            Family::PartitiveDe => "de la",
            Family::ContractedA => "à la",
        }),
        Gender::Epicene => None,
    }
}

/// Valeur unique d'un trait à travers les analyses « nom », ou `None` si
/// absente ou ambiguë.
fn consensus<T: PartialEq + Copy>(values: impl Iterator<Item = Option<T>>) -> Option<T> {
    let mut found: Option<T> = None;
    for v in values.flatten() {
        match found {
            None => found = Some(v),
            Some(prev) if prev == v => {}
            Some(_) => return None, // contradiction → indéterminé
        }
    }
    found
}

/// Calque la casse de `original` (initiale) sur `replacement`.
fn match_case(original: &str, replacement: &str) -> String {
    let starts_upper = original.chars().next().is_some_and(|c| c.is_uppercase());
    if !starts_upper {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => replacement.to_string(),
    }
}

/// Vérifie l'accord en genre et en nombre entre un déterminant et le nom qu'il
/// introduit.
pub struct DeterminerNounAgreement;

const RULE_ID: &str = "determiner_noun_agreement";

/// Nombre maximal d'adjectifs antéposés sautés entre le déterminant et le nom.
const MAX_PRENOMINAL: usize = 3;

/// Construit la correction du déterminant d'après le nom tête, ou `None` si
/// l'accord est déjà correct (ou le genre du nom indéterminé).
fn agree(det_token: &Token, det: &DetInfo, noun: &Token) -> Option<Suggestion> {
    let det_lower = det_token.text.to_lowercase();
    let noun_analyses: Vec<_> = morpho::lookup(&noun.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();

    let noun_gender = consensus(noun_analyses.iter().map(|m| m.gender));
    let noun_number = consensus(noun_analyses.iter().map(|m| m.number));

    // Genre/nombre cibles : ceux du nom s'ils sont connus, sinon ceux du
    // déterminant (auquel cas la correction restera identique).
    let det_gender = match det.gender {
        GenderC::Masc => Some(Gender::Masculine),
        GenderC::Fem => Some(Gender::Feminine),
        GenderC::Any => None,
    };
    let target_gender = noun_gender.or(det_gender);
    // Un nom **invariable en nombre** (singulier déjà terminé par -s/-x/-z :
    // « cours », « fois », « fils », « prix ») a la même forme aux deux nombres ;
    // Lexique ne lui prête souvent qu'une analyse « pluriel ». On ne corrige donc
    // pas le nombre du déterminant (« au cours », « à la fois » sont corrects) —
    // seul un désaccord de **genre** subsiste.
    let target_plural = if is_number_invariable(&noun_analyses) {
        det.plural
    } else {
        match noun_number {
            Some(Number::Singular) => false,
            Some(Number::Plural) => true,
            _ => det.plural,
        }
    };

    let corrected = corrected_form(det.family, target_gender, target_plural)?;
    if corrected.eq_ignore_ascii_case(&det_lower) {
        return None; // déjà accordé
    }
    // Allomorphes « ce » / « cet » : « cet » est le démonstratif masculin
    // singulier devant voyelle ou h-muet (« cet homme », « cet été »). Il est
    // correct — ne pas le « corriger » en « ce » (la distinction est phonétique,
    // pas un accord).
    if det_lower == "cet" && corrected == "ce" {
        return None;
    }
    Some(Suggestion {
        span: det_token.span,
        message: format!(
            "Accord déterminant–nom : « {} » ne s'accorde pas avec « {} ».",
            det_token.text, noun.text
        ),
        replacements: vec![match_case(&det_token.text, corrected)],
        rule_id: RULE_ID,
    })
}

impl Rule for DeterminerNounAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let lex = lexical_tokens(tokens);
        let mut suggestions = Vec::new();

        for i in 0..lex.len() {
            let det_token = lex[i].1;
            let det_lower = det_token.text.to_lowercase();
            let Some(det) = classify(&det_lower) else {
                continue;
            };

            // Rechercher le nom tête : sauter les adjectifs antéposés.
            let mut head: Option<&Token> = None;
            let mut steps = 0;
            let mut k = i + 1;
            while k < lex.len() && steps <= MAX_PRENOMINAL {
                let analyses = morpho::lookup(&lex[k].1.text);
                let has_noun = analyses.iter().any(|m| m.category == MorphCategory::Noun);
                let has_adj = analyses
                    .iter()
                    .any(|m| m.category == MorphCategory::Adjective);
                if has_noun {
                    // Un mot à la fois nom et adjectif (« douce », « vieux »…)
                    // suivi d'un autre nom est en réalité un adjectif antéposé :
                    // on continue pour atteindre le vrai nom tête.
                    let next_is_noun = k + 1 < lex.len()
                        && morpho::lookup(&lex[k + 1].1.text)
                            .iter()
                            .any(|m| m.category == MorphCategory::Noun);
                    if has_adj && next_is_noun {
                        k += 1;
                        steps += 1;
                        continue;
                    }
                    head = Some(lex[k].1);
                    break;
                }
                if has_adj {
                    k += 1;
                    steps += 1;
                    continue;
                }
                break; // ni nom ni adjectif → on arrête
            }

            let Some(noun) = head else { continue };
            if let Some(s) = agree(det_token, &det, noun) {
                suggestions.push(s);
            }
        }

        suggestions
    }

    /// Chemin **piloté par l'arbre** (production), **hybride**. Le nom tête est
    /// trouvé en priorité par l'arc `det` (par-delà **tout** nombre d'adjectifs
    /// antéposés, sans borne ni balayage) ; mais sur les fragments courts
    /// agrammaticaux le CRF mé-étiquette parfois les déterminants contractés
    /// (« aux »→ADJ, « au »→NUM), faussant l'arc. On **replie** alors sur le
    /// balayage POS (sauter les adjectifs jusqu'au nom). La garde pronom objet
    /// (« je les ferme ») reste assurée par l'étiquette `PRON`.
    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let lex = lexical_tokens(tokens);
        let mut suggestions = Vec::new();

        for i in 0..lex.len() {
            let (det_idx, det_token) = lex[i];
            let det_lower = det_token.text.to_lowercase();
            let Some(det) = classify(&det_lower) else {
                continue;
            };
            // Pronom objet homographe (« je les ferme ») : pas un déterminant.
            if tags[det_idx].upos == Upos::Pron {
                continue;
            }

            // Nom tête : d'abord par l'arc `det` (robuste, sans borne) ;
            // sinon balayage POS des adjectifs antéposés (repli mé-étiquetage).
            let head: Option<&Token> = if tags[det_idx].dep == DepRel::Det {
                crate::dep::head_of(tags, det_idx)
                    .filter(|&h| h > det_idx && matches!(tags[h].upos, Upos::Noun | Upos::Propn))
                    .map(|h| &tokens[h])
            } else {
                None
            }
            .or_else(|| {
                let mut steps = 0;
                let mut k = i + 1;
                while k < lex.len() && steps <= MAX_PRENOMINAL {
                    match tags[lex[k].0].upos {
                        Upos::Adj => {
                            k += 1;
                            steps += 1;
                        }
                        Upos::Noun | Upos::Propn => return Some(lex[k].1),
                        _ => break,
                    }
                }
                None
            });

            let Some(noun) = head else { continue };
            if let Some(s) = agree(det_token, &det, noun) {
                suggestions.push(s);
            }
        }

        suggestions
    }

    fn name(&self) -> &'static str {
        "Accord déterminant–nom"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn replacements(text: &str) -> Vec<(String, Vec<String>)> {
        DeterminerNounAgreement
            .check(&tokenize(text))
            .into_iter()
            .map(|s| (s.message, s.replacements))
            .collect()
    }

    fn first_replacement(text: &str) -> Option<String> {
        DeterminerNounAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    #[test]
    fn gender_mismatch_corrects_determiner() {
        // « un table » → « une » (table est féminin).
        assert_eq!(first_replacement("un table").as_deref(), Some("une"));
    }

    #[test]
    fn gender_mismatch_with_prenominal_adjective() {
        // « un belle table » → « une ».
        assert_eq!(first_replacement("un belle table").as_deref(), Some("une"));
    }

    #[test]
    fn number_mismatch_singular_to_plural() {
        // « le chats » → « les ».
        assert_eq!(first_replacement("le chats").as_deref(), Some("les"));
    }

    #[test]
    fn number_mismatch_plural_to_singular() {
        // « les chat » → « le » (chat est masculin singulier).
        assert_eq!(first_replacement("les chat").as_deref(), Some("le"));
    }

    #[test]
    fn correct_phrases_yield_nothing() {
        for ok in ["la table", "le chat", "les chats", "une table", "des chats"] {
            assert!(
                replacements(ok).is_empty(),
                "faux positif sur « {ok} » : {:?}",
                replacements(ok)
            );
        }
    }

    #[test]
    fn pronoun_le_before_verb_is_ignored() {
        // « je le mange » : « le » est pronom, suivi d'un verbe, pas d'un nom.
        assert!(replacements("je le mange").is_empty());
    }

    #[test]
    fn preserves_capitalization() {
        // En début de phrase : « Un table » → « Une ».
        assert_eq!(first_replacement("Un table").as_deref(), Some("Une"));
    }

    #[test]
    fn unknown_gender_noun_is_not_flagged() {
        // Un nom **épicène** (« camarade » : un/une) reste sans genre dans le
        // lexique : aucune fausse alerte d'accord en genre.
        assert!(replacements("un camarade").is_empty());
    }

    #[test]
    fn gender_override_enables_agreement_check() {
        // « maison » a un genre vide dans Lexique383, comblé par l'override curé
        // (cf. tools/compile-morpho/gender-overrides.tsv) : l'accord en genre
        // s'applique désormais (« un maison » → « une »).
        assert_eq!(first_replacement("un maison").as_deref(), Some("une"));
    }

    // --- Déterminants composés / contractés. ---

    #[test]
    fn partitive_gender_mismatch_is_two_words() {
        // « du table » → « de la » (table est féminin).
        assert_eq!(first_replacement("du table").as_deref(), Some("de la"));
    }

    #[test]
    fn contracted_a_gender_mismatch_is_two_words() {
        // « au table » → « à la ».
        assert_eq!(first_replacement("au table").as_deref(), Some("à la"));
    }

    #[test]
    fn contracted_number_mismatch() {
        // « au chats » → « aux » (chats est pluriel).
        assert_eq!(first_replacement("au chats").as_deref(), Some("aux"));
        // « aux chat » → « au » (chat est masculin singulier).
        assert_eq!(first_replacement("aux chat").as_deref(), Some("au"));
    }

    #[test]
    fn ambiguous_prenominal_adjective_is_skipped() {
        // « douce »/« vieux » ont une lecture nominale parasite : le vrai nom
        // tête est celui qui suit. Pas de fausse alerte sur le déterminant.
        for ok in [
            "une douce lumière",
            "le vieux canapé",
            "une belle grande maison",
        ] {
            assert!(
                replacements(ok).is_empty(),
                "faux positif sur « {ok} » : {:?}",
                replacements(ok)
            );
        }
    }

    #[test]
    fn correct_composed_determiners_are_silent() {
        for ok in ["du pain", "au chat", "aux chats", "de la table"] {
            assert!(
                replacements(ok).is_empty(),
                "faux positif sur « {ok} » : {:?}",
                replacements(ok)
            );
        }
    }

    // --- Chemin POS (`check_tagged`). ---

    /// Tags de production : POS + arbre de dépendances (lu par `check_tagged`).
    fn tagged(text: &str) -> Vec<Suggestion> {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        DeterminerNounAgreement.check_tagged(&tokens, &tags)
    }

    fn tagged_first(text: &str) -> Option<String> {
        tagged(text)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn tagged_count(text: &str) -> usize {
        tagged(text).len()
    }

    #[test]
    fn pos_path_object_pronoun_is_not_determiner() {
        // « je les ferme » : « les » est pronom objet (POS), pas déterminant —
        // l'heuristique morpho seule produisait ici un faux positif (« la »).
        assert_eq!(tagged_count("je les ferme"), 0);
        assert_eq!(tagged_count("il la mange"), 0);
    }

    #[test]
    fn pos_path_still_corrects_real_mismatches() {
        assert_eq!(tagged_first("un table").as_deref(), Some("une"));
        assert_eq!(tagged_first("les chat").as_deref(), Some("le"));
        assert_eq!(tagged_first("un belle table").as_deref(), Some("une"));
    }

    #[test]
    fn pos_path_silent_on_correct_phrases() {
        for ok in [
            "la table",
            "les chats",
            "une douce lumière",
            "le vieux canapé",
        ] {
            assert_eq!(tagged_count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn tree_path_reaches_head_past_adjectives() {
        // L'arc `det` relie le déterminant au nom tête par-delà les adjectifs
        // antéposés (« un belle grande table » → « une »), sans balayage ni
        // borne, et reste silencieux quand l'accord est correct.
        assert_eq!(tagged_first("un belle grande table").as_deref(), Some("une"));
        assert_eq!(tagged_count("une belle grande table"), 0);
    }
}
