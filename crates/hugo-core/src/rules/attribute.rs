//! Règle : accord de l'adjectif attribut du sujet (genre et nombre).
//!
//! On traite la construction *sujet + verbe attributif + adjectif*, où
//! l'adjectif s'accorde avec le sujet : « elle est content » → « contente »,
//! « ils sont content » → « contents ».
//!
//! Approche prudente, pour limiter les faux positifs :
//!
//! - le **verbe attributif** appartient à une liste fermée (`être`, `sembler`,
//!   `paraître`, `devenir`, `rester`, `demeurer`) ;
//! - le **sujet** doit livrer un genre **et** un nombre sans ambiguïté :
//!   pronoms `il/elle/ils/elles`, ou groupe nominal dont le nom porte un genre
//!   connu dans Lexique (les pronoms `je/tu/nous/vous` sont écartés, leur genre
//!   dépendant du locuteur) ;
//! - l'**attribut** doit être analysé comme adjectif ; la forme corrigée est
//!   engendrée par [`morpho::decline`]. Si elle est introuvable, on n'émet rien.
//!
//! Avec les étiquettes POS du CRF ([`Rule::check_tagged`] ; [`Rule::check`]
//! reste le repli), la remontée vers le sujet ([`find_subject`]) **refuse** un
//! nom gouverné à gauche par une préposition ou un verbe
//! ([`is_governed_left`]) : objet d'un groupe prépositionnel (« …dans une
//! nouvelle ère »), ou objet d'un **participe présent** ouvrant une proposition
//! participiale (« les filles fatiguant leur père »). Faute de sujet sûr dans
//! la fenêtre, on s'abstient (précision > rappel).

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, Morph, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord en genre et en nombre de l'adjectif attribut avec son sujet.
pub struct AttributeAdjectiveAgreement;

const RULE_ID: &str = "attribute_adjective_agreement";

/// Lemmes des verbes attributifs (copules) déclencheurs.
const COPULAS: &[&str] = &[
    "être",
    "sembler",
    "paraître",
    "devenir",
    "rester",
    "demeurer",
];

/// Fenêtre maximale (en jetons lexicaux) explorée de part et d'autre du verbe.
const MAX_WINDOW: usize = 3;

/// Genre et nombre fournis par un pronom personnel sujet, le cas échéant.
/// Seuls les pronoms qui fixent le genre sont retenus.
fn pronoun_features(text: &str) -> Option<(Gender, Number)> {
    match normalize(text).as_str() {
        "il" => Some((Gender::Masculine, Number::Singular)),
        "elle" => Some((Gender::Feminine, Number::Singular)),
        "ils" => Some((Gender::Masculine, Number::Plural)),
        "elles" => Some((Gender::Feminine, Number::Plural)),
        _ => None,
    }
}

/// Minuscules + apostrophe finale ôtée (« qu' » → « qu »).
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Vrai si le jeton est un pronom réfléchi objet préverbal (marque d'une
/// construction pronominale, déléguée à `rules::pronominal_participle`).
fn is_reflexive(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "se" | "s" | "me" | "m" | "te" | "t" | "nous" | "vous"
    )
}

/// Vrai si le jeton est un clitique préverbal (négation ou pronom).
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

/// Vrai si le jeton est un adverbe d'intensité/négation pouvant s'intercaler
/// entre la copule et l'attribut (« elle est très content »).
fn is_intensifier(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "très"
            | "si"
            | "plus"
            | "moins"
            | "aussi"
            | "bien"
            | "trop"
            | "fort"
            | "peu"
            | "assez"
            | "plutôt"
            | "vraiment"
            | "extrêmement"
            | "particulièrement"
            | "pas"
            | "jamais"
    )
}

/// Valeur unique d'un trait à travers des analyses, ou `None` si absente ou
/// contradictoire.
fn consensus<T: PartialEq + Copy>(values: impl Iterator<Item = Option<T>>) -> Option<T> {
    let mut found: Option<T> = None;
    for v in values.flatten() {
        match found {
            None => found = Some(v),
            Some(prev) if prev == v => {}
            Some(_) => return None,
        }
    }
    found
}

/// Genre et nombre d'un jeton candidat sujet (pronom ou nom à genre connu).
fn subject_features(token: &Token) -> Option<(Gender, Number)> {
    if let Some(f) = pronoun_features(&token.text) {
        return Some(f);
    }
    // Pronoms personnels au genre indéterminé : on s'arrête là (Lexique leur
    // prête parfois une analyse nominale parasite, ex. « je » → nom féminin).
    if matches!(
        normalize(&token.text).as_str(),
        "je" | "j" | "tu" | "nous" | "vous" | "on" | "me" | "m" | "te" | "t" | "se" | "s"
    ) {
        return None;
    }
    let nouns: Vec<_> = morpho::lookup(&token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if nouns.is_empty() {
        return None;
    }
    let gender = consensus(nouns.iter().map(|m| m.gender))?;
    let number = consensus(nouns.iter().map(|m| m.number))?;
    // Un genre épicène ne permet pas de choisir la forme de l'adjectif.
    if gender == Gender::Epicene {
        return None;
    }
    Some((gender, number))
}

/// Vrai si l'analyse est un participe passé : enregistrement verbal sans
/// personne, mais porteur d'un genre et d'un nombre (« mangée », « partis »).
fn is_participle(m: &Morph) -> bool {
    m.category == MorphCategory::Verb
        && m.person.is_none()
        && m.gender.is_some()
        && m.number.is_some()
}

/// Lemme commun à toutes ces analyses, ou `None` s'il y en a plusieurs (ou
/// aucune).
fn unique_lemma<'a>(analyses: &[&'a Morph]) -> Option<&'a str> {
    let mut it = analyses.iter().map(|m| m.lemma.as_str());
    let first = it.next()?;
    it.all(|l| l == first).then_some(first)
}

/// Vrai si le jeton porte une copule conjuguée **à la 3ᵉ personne**.
///
/// Tous les sujets que cette règle accepte (noms à genre connu, pronoms
/// `il/elle/ils/elles`) sont à la 3ᵉ personne. Une forme attributive
/// exclusivement 1ʳᵉ/2ᵉ personne — « (je) suis », « (tu) es », « sois » à
/// l'impératif ou au subjonctif — n'a donc pas de sujet nominal : son sujet est
/// le locuteur ou l'interlocuteur, au genre indéterminé. On s'abstient alors,
/// ce qui évite de prendre un nom précédent pour le sujet (« La flamme de nos
/// lanternes, sois exaucé ! » ne déclenche plus « exaucées »).
fn is_third_person_copula(token: &Token) -> bool {
    morpho::verb_forms(&token.text)
        .iter()
        .any(|v| COPULAS.contains(&v.lemma.as_str()) && v.person == Person::Third)
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

/// Vrai si le nom à l'index lexical `s` est **gouverné à gauche** par une
/// préposition ou un verbe — il n'est donc pas le sujet de la copule.
///
/// En remontant par-dessus ses prémodifieurs (déterminants, adjectifs), on
/// atteint :
/// - une **préposition** (`ADP`) → objet d'un groupe prépositionnel
///   (« …dans une nouvelle ère ») ;
/// - un **verbe** (`VERB`/`AUX`), typiquement un **participe présent** ouvrant
///   une proposition participiale (« les filles *fatiguant* leur père sont… »
///   → « père » est l'objet de « fatiguant », pas le sujet de « sont »).
///
/// Dans les deux cas le nom appartient à une autre tête syntaxique : on le
/// refuse comme sujet.
fn is_governed_left(lex: &[(usize, &Token)], s: usize, tags: &[Tagged]) -> bool {
    let mut j = s;
    while j > 0 {
        j -= 1;
        match tags[lex[j].0].upos {
            Upos::Det | Upos::Adj => continue,
            Upos::Adp | Upos::Verb | Upos::Aux => return true,
            _ => return false,
        }
    }
    false
}

/// Cherche le sujet en reculant depuis la copule à l'index lexical `k`, en
/// sautant clitiques et adjectifs antéposés. Avec les tags POS, **refuse** un
/// nom objet d'une préposition (cf. [`is_prepositional_object`]) et continue à
/// remonter ; faute de sujet sûr dans la fenêtre, renvoie `None` (on s'abstient
/// plutôt que d'accorder sur un faux sujet — précision > rappel).
fn find_subject(
    lex: &[(usize, &Token)],
    k: usize,
    tags: Option<&[Tagged]>,
) -> Option<(Gender, Number)> {
    let mut s = k;
    let mut steps = 0;
    loop {
        if s == 0 || steps > MAX_WINDOW {
            return None;
        }
        s -= 1;
        steps += 1;
        let tok = lex[s].1;
        if let Some(f) = subject_features(tok) {
            if let Some(tags) = tags {
                if is_governed_left(lex, s, tags) {
                    continue;
                }
            }
            return Some(f);
        }
        // Sauter clitiques et adjectifs antéposés ; sinon abandonner.
        let is_adj = morpho::lookup(&tok.text)
            .iter()
            .any(|m| m.category == MorphCategory::Adjective);
        if is_clitic(&tok.text) || is_adj {
            continue;
        }
        return None;
    }
}

impl AttributeAdjectiveAgreement {
    fn scan(tokens: &[Token], tags: Option<&[Tagged]>) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for lex in lexical_sentences(tokens) {
            for k in 0..lex.len() {
                if !is_third_person_copula(lex[k].1) {
                    continue;
                }

                // Construction pronominale (« elle s'est levé ») : le réfléchi
                // précède immédiatement l'auxiliaire « être » ; on délègue à
                // `rules::pronominal_participle` (accord avec le sujet, garde du
                // COD postposé) plutôt que de traiter le participe en attribut.
                if k > 0 && is_reflexive(&lex[k - 1].1.text) {
                    continue;
                }

                // --- Sujet : reculer en sautant clitiques et adjectifs. ---
                let Some((gender, number)) = find_subject(&lex, k, tags) else {
                    continue;
                };

                // --- Attribut : avancer en sautant clitiques et adverbes. ---
                let mut a = k + 1;
                let mut steps = 0;
                while a < lex.len()
                    && steps < MAX_WINDOW
                    && (is_clitic(&lex[a].1.text) || is_intensifier(&lex[a].1.text))
                {
                    a += 1;
                    steps += 1;
                }
                if a >= lex.len() {
                    continue;
                }
                let adj_token = lex[a].1;

                // L'attribut peut être un adjectif (« content ») ou un participe
                // passé (« parti »), avec la copule « être » : « elle est parti »
                // → « partie ». Les participes ne sont pas toujours étiquetés
                // adjectifs dans Lexique, d'où la génération dédiée.
                let analyses = morpho::lookup(&adj_token.text);
                let adjectives: Vec<&Morph> = analyses
                    .iter()
                    .filter(|m| m.category == MorphCategory::Adjective)
                    .collect();
                let participles: Vec<&Morph> =
                    analyses.iter().filter(|m| is_participle(m)).collect();
                if adjectives.is_empty() && participles.is_empty() {
                    continue;
                }

                // Déjà accordé ? (un adjectif ou un participe compatible suffit)
                let agrees = |m: &Morph| {
                    m.gender
                        .map_or(true, |g| g == gender || g == Gender::Epicene)
                        && m.number
                            .map_or(true, |n| n == number || n == Number::Invariable)
                };
                if adjectives.iter().any(|m| agrees(m)) || participles.iter().any(|m| agrees(m)) {
                    continue;
                }

                // Génération : forme adjectivale (decline), sinon participe passé.
                let corrected = unique_lemma(&adjectives)
                    .and_then(|l| morpho::decline(l, gender, number))
                    .or_else(|| {
                        unique_lemma(&participles)
                            .and_then(|l| morpho::participle(l, gender, number))
                    });
                let Some(corrected) = corrected else {
                    continue;
                };
                if corrected.eq_ignore_ascii_case(&adj_token.text) {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: adj_token.span,
                    message: format!(
                        "Accord de l'attribut : « {} » ne s'accorde pas avec le sujet.",
                        adj_token.text
                    ),
                    replacements: vec![match_case(&adj_token.text, &corrected)],
                    rule_id: RULE_ID,
                });
            }
        }

        suggestions
    }
}

impl Rule for AttributeAdjectiveAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        Self::scan(tokens, None)
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        Self::scan(tokens, Some(tags))
    }

    fn name(&self) -> &'static str {
        "Accord de l'adjectif attribut"
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
        AttributeAdjectiveAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        AttributeAdjectiveAgreement.check(&tokenize(text)).len()
    }

    #[test]
    fn feminine_subject_pronoun() {
        assert_eq!(first("elle est content").as_deref(), Some("contente"));
    }

    #[test]
    fn plural_subject_pronoun() {
        assert_eq!(first("ils sont content").as_deref(), Some("contents"));
        assert_eq!(first("elles sont content").as_deref(), Some("contentes"));
    }

    #[test]
    fn intensifier_is_skipped() {
        assert_eq!(first("elle est très content").as_deref(), Some("contente"));
    }

    #[test]
    fn other_copulas() {
        assert_eq!(first("elle semble content").as_deref(), Some("contente"));
        assert_eq!(first("elle paraît content").as_deref(), Some("contente"));
        assert_eq!(first("elle devient content").as_deref(), Some("contente"));
    }

    #[test]
    fn nominal_subject_with_known_gender() {
        // « table » est féminin : « la table est content » → « contente ».
        assert_eq!(first("la table est content").as_deref(), Some("contente"));
    }

    #[test]
    fn correct_agreement_yields_nothing() {
        for ok in [
            "elle est contente",
            "il est content",
            "ils sont contents",
            "elles sont contentes",
            "elle est rouge",
            "ils sont rouges",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn ambiguous_gender_pronoun_is_ignored() {
        // « je suis content » : genre du locuteur inconnu → aucune correction.
        assert_eq!(count("je suis content"), 0);
        assert_eq!(count("vous êtes content"), 0);
    }

    #[test]
    fn noun_attribute_is_ignored() {
        // « elle est professeur » : attribut nominal, pas adjectival.
        assert_eq!(count("elle est professeur"), 0);
    }

    #[test]
    fn capitalization_preserved() {
        assert_eq!(first("Elle est Content").as_deref(), Some("Contente"));
    }

    // --- Participe passé avec être. ---

    #[test]
    fn past_participle_with_etre() {
        // « elle est parti » → « partie » (participe non étiqueté adjectif).
        assert_eq!(first("elle est parti").as_deref(), Some("partie"));
        assert_eq!(first("ils sont parti").as_deref(), Some("partis"));
        assert_eq!(first("elles sont allé").as_deref(), Some("allées"));
    }

    #[test]
    fn past_participle_correct_is_silent() {
        for ok in [
            "elle est partie",
            "ils sont partis",
            "elles sont allées",
            "il est parti",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn imperative_copula_has_no_nominal_subject() {
        // « sois » (impératif/subjonctif, 1ʳᵉ/2ᵉ pers.) n'a pas de sujet nominal :
        // le nom qui précède n'est pas le sujet. Le correctif tient sur la phrase
        // complète comme sur le fragment — c'est la personne de la copule, non la
        // ponctuation, qui tranche.
        assert_eq!(
            count("Quand s'allume la flamme de nos lanternes, sois exaucé."),
            0
        );
        assert_eq!(count("La flamme de nos lanternes, sois exaucé !"), 0);
        assert_eq!(count("sois exaucé"), 0);
        assert_eq!(count("sois prêt"), 0);
    }

    #[test]
    fn avoir_auxiliary_is_not_subject_agreement() {
        // Avec « avoir », le participe ne s'accorde pas avec le sujet.
        // « elle a mangé » ne doit rien déclencher.
        assert_eq!(count("elle a mangé"), 0);
        assert_eq!(count("ils ont mangé"), 0);
    }

    // --- Chemin POS (`check_tagged`). ---

    fn tagged_first(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AttributeAdjectiveAgreement
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn tagged_count(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        AttributeAdjectiveAgreement.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn pos_path_still_corrects_real_mismatches() {
        assert_eq!(tagged_first("elle est content").as_deref(), Some("contente"));
        assert_eq!(tagged_first("ils sont content").as_deref(), Some("contents"));
        assert_eq!(tagged_first("elle est parti").as_deref(), Some("partie"));
    }

    #[test]
    fn pos_path_prepositional_object_is_not_subject() {
        // « ère » est l'objet de « dans » : ce n'est pas le sujet de « nommé »,
        // donc aucune fausse correction « nommée ». Le vrai sujet (« outil »)
        // est hors fenêtre : on s'abstient plutôt que d'accorder à tort.
        assert_eq!(
            tagged_count("l'outil propulsé dans une nouvelle ère fut nommé Intune"),
            0
        );
        // Cas court : « le chat de la voisine est content » — « voisine »
        // (objet de « de ») n'est pas le sujet ; « content » s'accorde déjà au
        // vrai sujet masculin « chat ».
        assert_eq!(tagged_count("le chat de la voisine est content"), 0);
    }

    #[test]
    fn pos_path_present_participle_clause_is_not_subject() {
        // Bug historique : « père », objet du participe présent « fatiguant »,
        // n'est pas le sujet de « sont » → pas de fausse correction « fatigant »
        // (l'accord « fatigantes » avec « filles » est correct).
        assert_eq!(
            tagged_count("les filles fatiguant leur père sont fatigantes"),
            0
        );
    }
}
