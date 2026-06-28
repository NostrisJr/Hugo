//! Règle : accords spéciaux — adjectifs de couleur invariables (simples et
//! composés), *même*, *quelque*.
//!
//! Quatre accords particuliers, traités avec prudence (confirmation POS via
//! [`Rule::check_tagged`]) :
//!
//! - **couleurs invariables simples** : un nom employé comme adjectif de couleur
//!   reste invariable — « des gants marrons » → « marron » ([`INVARIABLE_COLORS`]).
//!   Exclut les couleurs devenues vrais adjectifs (rose, mauve…). Ne se déclenche
//!   que **postposée à un nom** (écarte « des oranges », fruits) ;
//! - **couleurs composées invariables** : quand deux mots forment ensemble un
//!   adjectif de couleur ([`COMPOUND_COLORS`]), le composé est invariable en genre
//!   et en nombre — « des robes bleues ciel » → « bleu ciel », « des manteaux verts
//!   bouteille » → « vert bouteille ». Détecté par le lemme adjectival du premier
//!   élément + correspondance sur le modificateur. Ne se déclenche que **postposé à
//!   un nom** ;
//! - **même** : adjectif accordé en nombre (« les même livres » → « mêmes ») ;
//!   adverbe (« même les enfants ») non touché ;
//! - **quelque** : devant un nom pluriel → « quelques » (« quelque livres ») ;
//!   emplois invariables (« quelque chose ») épargnés.

use super::{lexical_sentences, Rule};
use crate::morpho::{self, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::{Span, Suggestion};

/// Accords spéciaux (couleurs invariables, *même*, *quelque*).
pub struct SpecialAgreement;

const RULE_ID: &str = "special_agreement";

/// Couleurs **issues de noms**, invariables en emploi adjectival. Exclut les
/// couleurs devenues de vrais adjectifs (rose, mauve, pourpre, écarlate, fauve),
/// qui s'accordent.
const INVARIABLE_COLORS: &[&str] = &[
    "marron",
    "orange",
    "turquoise",
    "émeraude",
    "marine",
    "crème",
    "cerise",
    "chocolat",
    "noisette",
    "olive",
    "saumon",
    "corail",
    "ivoire",
    "kaki",
    "ocre",
    "grenat",
    "lavande",
    "moutarde",
    "caramel",
    "anthracite",
    "azur",
    "brique",
];

/// Couleurs composées (deux mots) invariables en emploi adjectival.
/// Le premier élément est un adjectif de couleur variable quand il est seul
/// (bleu → bleue, vert → verte…) ; le composé, lui, reste invariable.
/// Les composés dont le second mot figure déjà dans [`INVARIABLE_COLORS`]
/// sont inclus pour capturer les erreurs sur le **premier** mot.
const COMPOUND_COLORS: &[(&str, &str)] = &[
    ("bleu", "ciel"),
    ("bleu", "marine"),
    ("bleu", "nuit"),
    ("bleu", "roi"),
    ("bleu", "électrique"),
    ("bleu", "pétrole"),
    ("rouge", "sang"),
    ("rouge", "brique"),
    ("vert", "pomme"),
    ("vert", "bouteille"),
    ("vert", "anis"),
    ("vert", "olive"),
    ("vert", "amande"),
    ("vert", "forêt"),
    ("jaune", "citron"),
    ("jaune", "paille"),
    ("jaune", "soleil"),
    ("gris", "perle"),
    ("gris", "souris"),
    ("gris", "ardoise"),
    ("gris", "acier"),
    ("noir", "charbon"),
    ("blanc", "cassé"),
    ("blanc", "neige"),
    ("blanc", "crème"),
    ("rose", "bonbon"),
    ("rose", "saumon"),
];

/// Renvoie le lemme de base (`c1`) si `form` est un adjectif de couleur
/// composée — i.e. son lemme adjectival figure comme premier élément dans
/// [`COMPOUND_COLORS`].
fn compound_color_base(form: &str) -> Option<&'static str> {
    for morph in &morpho::lookup(form) {
        if morph.category != MorphCategory::Adjective {
            continue;
        }
        for &(c1, _) in COMPOUND_COLORS {
            if morph.lemma == c1 {
                return Some(c1);
            }
        }
    }
    None
}

/// Vrai si `form` correspond au modificateur `c2` (exact, pluriel simple `-s`,
/// ou via le lemme adjectival pour les formes en `-ée/-ées/-és`).
fn modifier_matches(form: &str, c2: &str) -> bool {
    let low = form.to_lowercase();
    if low == c2 {
        return true;
    }
    if low.trim_end_matches('s') == c2 {
        return true;
    }
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == MorphCategory::Adjective && m.lemma == c2)
}

/// Vérifie si les jetons `i` et `i+1` forment une couleur composée mal accordée
/// postposée à un nom étiqueté NOUN/PROPN. Renvoie une `Suggestion` couvrant les
/// deux jetons si au moins l'un d'eux dévie de sa forme de base.
fn compound_color_suggestion(
    lex: &[(usize, &Token)],
    i: usize,
    tags: &[Tagged],
) -> Option<Suggestion> {
    let Some(c1) = compound_color_base(lex[i].1.text.as_str()) else {
        return None;
    };
    let form_i = lex[i].1.text.to_lowercase();

    let &(_, tok_j) = lex.get(i + 1)?;
    let form_j = tok_j.text.to_lowercase();

    // Trouver le composé c1 + c2 dont le modificateur correspond au jeton suivant.
    let matching_c2 = COMPOUND_COLORS
        .iter()
        .filter(|&&(base, _)| base == c1)
        .find(|&&(_, c2)| modifier_matches(&form_j, c2))
        .map(|&(_, c2)| c2)?;

    // Doit être postposé à un nom.
    if i == 0 || !matches!(tags[lex[i - 1].0].upos, Upos::Noun | Upos::Propn) {
        return None;
    }

    let c1_wrong = form_i != c1;
    let c2_wrong = form_j != matching_c2;
    if !c1_wrong && !c2_wrong {
        return None;
    }

    let tok_i = lex[i].1;
    let original = format!("{} {}", tok_i.text, tok_j.text);
    let corrected = format!("{} {}", c1, matching_c2);
    Some(Suggestion {
        span: Span::new(tok_i.span.start, tok_j.span.end),
        message: format!(
            "Couleur composée invariable : « {original} » devrait être « {corrected} »."
        ),
        replacements: vec![corrected],
        rule_id: RULE_ID,
    })
}

/// Minuscules + apostrophe finale ôtée.
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

/// Si `cur` (minuscule) est la forme **pluralisée** d'une couleur invariable,
/// renvoie sa forme de base (invariable). « marrons » → « marron ».
fn pluralized_color(cur: &str) -> Option<&'static str> {
    let base = cur.strip_suffix('s')?;
    INVARIABLE_COLORS.iter().copied().find(|&c| c == base)
}

fn is_plural_determiner(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs"
    )
}

fn is_singular_determiner(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "le" | "la"
            | "l"
            | "un"
            | "une"
            | "ce"
            | "cet"
            | "cette"
            | "mon"
            | "ton"
            | "son"
            | "ma"
            | "ta"
            | "sa"
            | "notre"
            | "votre"
            | "leur"
            | "du"
    )
}

/// Vrai si toutes les analyses nominales du jeton sont au pluriel (et il en
/// existe au moins une).
fn noun_is_plural(text: &str) -> bool {
    let nouns: Vec<_> = morpho::lookup(text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    !nouns.is_empty() && nouns.iter().all(|m| m.number == Some(Number::Plural))
}

/// Cherche la correction d'accord spécial pour le jeton lexical `i`.
fn correction(lex: &[(usize, &Token)], i: usize, tags: &[Tagged]) -> Option<String> {
    let cur = normalize(lex[i].1.text.as_str());
    let tag_of = |k: usize| tags[lex[k].0].upos;
    let prev_is_noun = i > 0 && matches!(tag_of(i - 1), Upos::Noun | Upos::Propn);

    // --- Couleur invariable postposée à un nom. ---
    if let Some(base) = pluralized_color(&cur) {
        if prev_is_noun {
            return Some(base.to_string());
        }
    }

    match cur.as_str() {
        // même → mêmes : adjectif après un pluriel ; pas l'adverbe « même les ».
        "même" => {
            let next_is_det = lex.get(i + 1).is_some_and(|(_, t)| {
                is_plural_determiner(&t.text) || is_singular_determiner(&t.text)
            });
            if next_is_det {
                return None; // « même les enfants » : adverbe « even ».
            }
            let prev_plural = i > 0
                && (is_plural_determiner(&lex[i - 1].1.text)
                    || (matches!(tag_of(i - 1), Upos::Noun) && noun_is_plural(&lex[i - 1].1.text)));
            prev_plural.then(|| "mêmes".to_string())
        }
        // mêmes → même : après un déterminant singulier (« le mêmes »).
        "mêmes" => {
            (i > 0 && is_singular_determiner(&lex[i - 1].1.text)).then(|| "même".to_string())
        }
        // quelque → quelques : devant un nom pluriel.
        "quelque" => {
            let next_plural_noun = lex.get(i + 1).is_some_and(|&(idx, t)| {
                matches!(tags[idx].upos, Upos::Noun) && noun_is_plural(&t.text)
            });
            next_plural_noun.then(|| "quelques".to_string())
        }
        _ => None,
    }
}

impl Rule for SpecialAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.check_tagged(tokens, &crate::pos::tag(tokens))
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for i in 0..lex.len() {
                // Couleur composée (deux jetons) — vérifiée en premier pour
                // pouvoir sauter i+1 si les deux mots sont couverts.
                if let Some(s) = compound_color_suggestion(&lex, i, tags) {
                    suggestions.push(s);
                    continue;
                }
                let Some(corrected) = correction(&lex, i, tags) else {
                    continue;
                };
                let token = lex[i].1;
                suggestions.push(Suggestion {
                    span: token.span,
                    message: format!(
                        "Accord spécial : « {} » devrait être « {} ».",
                        token.text, corrected
                    ),
                    replacements: vec![match_case(&token.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Accords spéciaux (couleurs, même, quelque)"
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
        SpecialAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        SpecialAgreement.check(&tokenize(text)).len()
    }

    #[test]
    fn invariable_color_postnominal() {
        assert_eq!(first("des gants marrons").as_deref(), Some("marron"));
        assert_eq!(first("les yeux noisettes").as_deref(), Some("noisette"));
    }

    #[test]
    fn color_as_noun_is_not_flagged() {
        // « des oranges » (fruits) / « des marrons » (châtaignes) : noms têtes.
        assert_eq!(count("des oranges"), 0);
        assert_eq!(count("des marrons"), 0);
    }

    #[test]
    fn correct_invariable_color_is_silent() {
        assert_eq!(count("des gants marron"), 0);
        assert_eq!(count("des yeux noisette"), 0);
    }

    #[test]
    fn meme_agrees_in_number() {
        assert_eq!(first("les même livres").as_deref(), Some("mêmes"));
        assert_eq!(first("les livres même").as_deref(), Some("mêmes"));
    }

    #[test]
    fn adverbial_meme_is_not_flagged() {
        // « même les enfants » (even) : adverbe invariable.
        assert_eq!(count("même les enfants sont venus"), 0);
        assert_eq!(count("le même livre"), 0);
    }

    #[test]
    fn quelque_before_plural_noun() {
        assert_eq!(first("quelque livres").as_deref(), Some("quelques"));
    }

    #[test]
    fn invariable_quelque_is_silent() {
        for ok in ["quelque chose", "quelque part", "quelques livres"] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    // --- Couleurs composées ---

    #[test]
    fn compound_color_first_word_inflected() {
        assert_eq!(first("des robes bleues ciel").as_deref(), Some("bleu ciel"));
        assert_eq!(first("des manteaux verts bouteille").as_deref(), Some("vert bouteille"));
        assert_eq!(first("des gants rouges sang").as_deref(), Some("rouge sang"));
        assert_eq!(first("des chaussures noires charbon").as_deref(), Some("noir charbon"));
        assert_eq!(first("des murs blancs cassé").as_deref(), Some("blanc cassé"));
    }

    #[test]
    fn compound_color_both_words_inflected() {
        assert_eq!(first("des robes bleues ciels").as_deref(), Some("bleu ciel"));
        assert_eq!(first("des manteaux gris perles").as_deref(), Some("gris perle"));
    }

    #[test]
    fn compound_color_only_second_word_inflected() {
        assert_eq!(first("des robes bleu ciels").as_deref(), Some("bleu ciel"));
    }

    #[test]
    fn compound_color_already_correct_is_silent() {
        for ok in [
            "des robes bleu ciel",
            "un manteau vert bouteille",
            "des gants gris perle",
            "une veste rouge sang",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn compound_color_without_preceding_noun_is_silent() {
        // Pas de nom avant → ne pas déclencher.
        assert_eq!(count("bleues ciel"), 0);
    }
}
