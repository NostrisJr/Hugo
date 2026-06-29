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
use crate::dep;
use crate::morpho::{self, Gender, Morph, MorphCategory, Number, Person};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Sujet (index d'origine) d'un attribut selon l'arbre : `nsubj` de l'attribut
/// lui-même (prédicat d'une copule), sinon `nsubj` de sa tête.
fn tree_subject_for_attr(tags: &[Tagged], adj_orig: usize) -> Option<usize> {
    if let Some(s) = dep::subject_of(tags, adj_orig) {
        return Some(s);
    }
    let head = dep::head_of(tags, adj_orig)?;
    dep::subject_of(tags, head)
}

/// Veto par l'arbre : vrai si l'attribut `adj_token` s'accorde **déjà** en genre
/// et nombre avec son vrai sujet (`nsubj` de l'arbre). La détection
/// positionnelle a alors visé le mauvais sujet (inversion, complément,
/// apposition) → la suggestion serait un faux positif.
fn attribute_agrees_with_tree_subject(
    lex: &[(usize, &Token)],
    tags: &[Tagged],
    adj_orig: usize,
    adj_token: &Token,
) -> bool {
    let Some(s_idx) = tree_subject_for_attr(tags, adj_orig) else {
        return false;
    };
    // Sujet coordonné : l'arbre ne pointe qu'un conjoint (singulier) → ne pas
    // voter, laisser la détection positionnelle gérer l'accord pluriel.
    if dep::children(tags, s_idx)
        .iter()
        .any(|&c| tags[c].dep == crate::dep::DepRel::Conj)
    {
        return false;
    }
    let Some(s_tok) = lex.iter().find(|(o, _)| *o == s_idx).map(|(_, t)| *t) else {
        return false;
    };
    let Some((g, n)) = pronoun_features(&s_tok.text).or_else(|| subject_features(s_tok)) else {
        return false;
    };
    morpho::lookup(&adj_token.text).iter().any(|m| {
        (m.category == MorphCategory::Adjective || is_participle(m))
            && m.gender.map_or(true, |mg| mg == g || mg == Gender::Epicene)
            && m.number.map_or(true, |mn| mn == n || mn == Number::Invariable)
    })
}

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
    // Si une analyse a un nombre indéterminé (forme invariable — ex. « fils »
    // qui est à la fois sg et pl), on ne peut pas choisir le nombre → abandon.
    if nouns.iter().any(|m| m.number.is_none()) {
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

/// Vrai si l'étiquette POS du jeton est compatible avec un **attribut**
/// (adjectif ou participe passé). On exclut les catégories qui ne sont jamais
/// attribut — au premier chef la **préposition** (`ADP`) — afin de neutraliser
/// les homographes préposition/adjectif : « sur » est aussi l'adjectif « acide »
/// au lexique, mais « elle est *sur* le côté » (où le CRF l'étiquette `ADP`) ne
/// doit pas devenir « sure ». Les attributs réels, même mal étiquetés `VERB` par
/// le CRF (« content », « parti », « allé »), restent acceptés.
fn is_attribute_tag(upos: Upos) -> bool {
    !matches!(
        upos,
        Upos::Adp
            | Upos::Det
            | Upos::Pron
            | Upos::Cconj
            | Upos::Sconj
            | Upos::Num
            | Upos::Part
            | Upos::Punct
            | Upos::Sym
            | Upos::Intj
    )
}

/// Vrai si le jeton est un déterminant pouvant précéder un nom sujet — utile
/// pour le saut lors de la détection de coordination (`A et B`).
fn is_subject_det(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "le" | "la" | "les" | "l"
            | "un" | "une" | "des"
            | "ce" | "cet" | "cette" | "ces"
            | "mon" | "ton" | "son" | "ma" | "ta" | "sa"
            | "mes" | "tes" | "ses"
            | "notre" | "votre" | "leur"
            | "nos" | "vos" | "leurs"
            | "du" | "au" | "aux"
    )
}

/// Genre et nombre portés par un déterminant singulier ou pluriel.
///
/// Retourne `(genre_ou_None, nombre)`. Le genre est `None` pour les
/// déterminants pluriels (« les », « des »…) ou l'article élidé « l' » où
/// le genre ne peut être inféré du déterminant seul. Sert de contexte pour
/// désambiguïser les noms épicènes (ex. « voile » = Masc ou Fém).
fn det_features(text: &str) -> Option<(Option<Gender>, Number)> {
    let s = normalize(text);
    let s = s.trim_end_matches(['\'', '\u{2019}']);
    Some(match s {
        "le" | "ce" | "cet" | "un" | "mon" | "ton" | "son" | "notre" | "votre" | "leur"
        | "du" | "au" => (Some(Gender::Masculine), Number::Singular),
        "la" | "cette" | "une" | "ma" | "ta" | "sa" => (Some(Gender::Feminine), Number::Singular),
        "les" | "ces" | "des" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" | "aux" => {
            (None, Number::Plural)
        }
        "l" => (None, Number::Singular),
        _ => return None,
    })
}

/// Cherche le **sujet postposé** dans la plage `lex[from..]`, en sautant les
/// déterminants. S'arrête sur le premier nom ou nom propre et retourne ses
/// traits (genre, nombre). Si le nom est épicène ou sans genre dans le
/// lexique, le genre du déterminant précédent sert de contexte.
///
/// Cette recherche ne s'active qu'en repli lorsque la recherche arrière a
/// échoué (pas de sujet préposé trouvé dans le chemin normal).
fn find_postposed_subject(
    lex: &[(usize, &Token)],
    from: usize,
    tags: Option<&[Tagged]>,
) -> Option<(Gender, Number)> {
    let mut det_g: Option<Gender> = None;
    let mut det_n: Option<Number> = None;
    let mut steps = 0;
    let mut j = from;

    while j < lex.len() && steps <= MAX_WINDOW {
        let (tok_idx, tok) = lex[j];

        // Déterminant : noter le genre/nombre pour lever les ambiguïtés du nom.
        if let Some((g, n)) = det_features(&tok.text) {
            det_g = g;
            det_n = Some(n);
            j += 1;
            steps += 1;
            continue;
        }

        // Nom ou nom propre : traits du lexique, ou traits du déterminant si
        // le nom est épicène (genre ambigu dans Lexique).
        let is_nominal = match tags {
            Some(t) => matches!(t[tok_idx].upos, Upos::Noun | Upos::Propn),
            None => morpho::lookup(&tok.text)
                .iter()
                .any(|m| m.category == MorphCategory::Noun),
        };
        if !is_nominal {
            return None;
        }

        return subject_features(tok).or_else(|| Some((det_g?, det_n?)));
    }
    None
}

/// Fusionne genre et nombre de plusieurs sujets coordonnés.
///
/// Le masculin l'emporte sur le féminin (`géants et collines` → Masc) ;
/// le nombre est toujours pluriel pour une coordination à deux membres ou plus.
fn merge_subject_features(subjects: &[(Gender, Number)]) -> (Gender, Number) {
    if subjects.len() == 1 {
        return subjects[0];
    }
    let gender = if subjects.iter().any(|(g, _)| *g == Gender::Masculine) {
        Gender::Masculine
    } else {
        Gender::Feminine
    };
    (gender, Number::Plural)
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

/// Genre et nombre d'un nom **invariable** (entrées Lexique avec `number=None`)
/// résolus grâce au déterminant qui le précède immédiatement dans la phrase
/// lexicale.
///
/// Ex. `lex[s-1]="Mon"`, `lex[s]="fils"` → "mon" = Masc. Sg. → (Masc, Sg).
/// Ex. `lex[s-1]="les"`, `lex[s]="fils"` → "les" = Pl. → (Masc, Pl).
///
/// Renvoie `None` si le token n'a pas d'analyse nominale invariable, si aucun
/// déterminant ne précède, ou si le genre est incompatible / épicène.
fn resolve_with_det(lex: &[(usize, &Token)], s: usize) -> Option<(Gender, Number)> {
    let tok = lex[s].1;
    // Ne s'applique qu'aux noms dont au moins une analyse a number=None.
    let nouns: Vec<_> = morpho::lookup(&tok.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun && m.number.is_none())
        .collect();
    if nouns.is_empty() {
        return None;
    }
    let gender = consensus(nouns.iter().map(|m| m.gender))?;
    if gender == Gender::Epicene {
        return None;
    }
    // Remonter en arrière en sautant les adjectifs (qu'ils soient invariables
    // ou non — on cherche uniquement le déterminant, pas les traits de l'adjectif)
    // jusqu'à trouver un déterminant, ou s'arrêter sur n'importe quel autre token.
    let mut j = s;
    while j > 0 {
        j -= 1;
        if let Some((det_gender, det_number)) = det_features(&lex[j].1.text) {
            if let Some(dg) = det_gender {
                if dg != gender {
                    return None;
                }
            }
            return Some((gender, det_number));
        }
        // Ce token n'est pas un déterminant : continuer seulement si c'est un
        // adjectif (invariable ou non). Tout autre token (verbe, adverbe…)
        // indique qu'on a quitté le groupe nominal → on s'arrête.
        let is_adj = morpho::lookup(&lex[j].1.text)
            .iter()
            .any(|m| m.category == MorphCategory::Adjective);
        if !is_adj {
            return None;
        }
    }
    None
}

/// Cherche le(s) sujet(s) en reculant depuis la copule à l'index lexical `k`.
///
/// **Phase 1** (logique originale) : remonte en sautant clitiques et adjectifs
/// jusqu'au premier nom/pronom sujet. Avec les tags POS, refuse un nom régi par
/// une préposition (`is_governed_left`) et continue à remonter.
///
/// **Phase 2** (coordination) : depuis le sujet trouvé, cherche un `et`/`ou`
/// vers l'arrière (en sautant les déterminants) puis un second sujet — et ainsi
/// de suite. Les sujets ainsi collectés sont fusionnés : le nombre est pluriel,
/// le genre masculin si l'un au moins est masculin (règle du masculin-l'emporte).
///
/// Faute de sujet sûr dans la fenêtre, renvoie `None` (précision > rappel).
fn find_subject(
    lex: &[(usize, &Token)],
    k: usize,
    tags: Option<&[Tagged]>,
) -> Option<(Gender, Number)> {
    // --- Phase 1 : sujet le plus proche. ---
    let mut s = k;
    let mut steps = 0;
    let first = loop {
        if s == 0 || steps > MAX_WINDOW {
            return None;
        }
        s -= 1;
        steps += 1;
        let tok = lex[s].1;
        if let Some(f) = subject_features(tok)
            .or_else(|| resolve_with_det(lex, s))
        {
            if let Some(tags) = tags {
                if is_governed_left(lex, s, tags) {
                    continue;
                }
            }
            break f;
        }
        let is_adj = morpho::lookup(&tok.text)
            .iter()
            .any(|m| m.category == MorphCategory::Adjective);
        if is_clitic(&tok.text) || is_adj {
            continue;
        }
        return None;
    };
    // `s` : position dans lex du premier sujet trouvé.

    // --- Phase 2 : détection de coordination A et B [et C…]. ---
    let mut all = vec![first];
    let mut pos = s;

    'coord: loop {
        // Chercher "et"/"ou" en reculant depuis `pos`, en sautant les
        // déterminants (cas « les X et les Y »).
        let mut peek = pos;
        loop {
            if peek == 0 {
                break 'coord;
            }
            peek -= 1;
            let peek_lower = normalize(&lex[peek].1.text);
            if peek_lower == "et" || peek_lower == "ou" {
                // "et" trouvé : chercher le sujet qui le précède.
                let et_pos = peek;
                let mut q = et_pos;
                loop {
                    if q == 0 {
                        break 'coord;
                    }
                    q -= 1;
                    let qtok = lex[q].1;
                    if let Some(f) = subject_features(qtok)
                        .or_else(|| resolve_with_det(lex, q))
                    {
                        if let Some(tags) = tags {
                            if is_governed_left(lex, q, tags) {
                                break 'coord;
                            }
                        }
                        all.push(f);
                        pos = q;
                        continue 'coord;
                    }
                    // Sauter déterminants et adjectifs antéposés.
                    if is_subject_det(&normalize(&qtok.text)) {
                        continue;
                    }
                    let is_adj = morpho::lookup(&qtok.text)
                        .iter()
                        .any(|m| m.category == MorphCategory::Adjective);
                    if is_adj {
                        continue;
                    }
                    break 'coord;
                }
            } else if is_subject_det(&peek_lower) {
                // Déterminant avant le sujet déjà trouvé : continuer.
                continue;
            } else {
                break; // ni "et" ni déterminant → pas de coordination
            }
        }
        break;
    }

    Some(merge_subject_features(&all))
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

                // Sujet disjonctif « ni … ni … » : l'attribut/le participe peut
                // s'accorder au singulier OU au pluriel (« ni l'un ni l'autre ne
                // sont/n'est venu(s) »). On s'abstient dès deux « ni » avant la copule.
                if lex[..k]
                    .iter()
                    .filter(|(_, t)| normalize(&t.text) == "ni")
                    .count()
                    >= 2
                {
                    continue;
                }

                // --- Attribut : avancer en sautant les adverbes d'intensité/
                // négation (« très », « pas »…). On cherche l'attribut en
                // premier afin de pouvoir, en cas d'échec de la recherche du
                // sujet en arrière, lancer une recherche en avant (sujet
                // postposé immédiatement après l'attribut).
                let mut a = k + 1;
                let mut attr_steps = 0;
                while a < lex.len() && attr_steps < MAX_WINDOW && is_intensifier(&lex[a].1.text) {
                    a += 1;
                    attr_steps += 1;
                }
                if a >= lex.len() {
                    continue;
                }
                let adj_token = lex[a].1;

                // Garde POS : un attribut adjectival/participial n'est jamais
                // étiqueté préposition, déterminant, pronom… Écarte les
                // homographes (« sur » = préposition mais aussi adjectif au
                // lexique : « elle est sur le côté » ne doit pas devenir « sure »).
                if let Some(tags) = tags {
                    if !is_attribute_tag(tags[lex[a].0].upos) {
                        continue;
                    }
                    // Veto par l'arbre : si l'attribut s'accorde déjà avec son
                    // vrai sujet (nsubj), la détection positionnelle a visé le
                    // mauvais sujet → abstention (réduit les faux positifs sur
                    // inversions, compléments et appositions).
                    if attribute_agrees_with_tree_subject(&lex, tags, lex[a].0, adj_token) {
                        continue;
                    }
                }

                // --- Sujet : d'abord en reculant depuis la copule (ordre
                // direct), puis en avançant depuis l'attribut (sujet postposé)
                // si la recherche arrière échoue.
                // Exemple de sujet postposé : « Sous le vent sera déployée
                // ma voile » → « sera » copule, « déployée » attribut, « ma
                // voile » sujet postposé.
                let Some((gender, number)) = find_subject(&lex, k, tags)
                    .or_else(|| find_postposed_subject(&lex, a + 1, tags))
                else {
                    continue;
                };

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
        AttributeAdjectiveAgreement
            .check_tagged(&tokens, &tags)
            .len()
    }

    /// Comptage avec étiquettes complètes (POS + dépendances) : active le veto.
    fn count_full(text: &str) -> usize {
        let tokens = tokenize(text);
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        AttributeAdjectiveAgreement.check_tagged(&tokens, &tags).len()
    }

    #[test]
    fn tree_veto_suppresses_wrong_subject_attr() {
        // « la salle était pleine » : « pleine » s'accorde avec « salle » (fém sg) ;
        // la détection positionnelle visait « clients/habitués » (pl). Le veto par
        // l'arbre (attribut accordé avec son nsubj) supprime le faux positif.
        assert_eq!(
            count_full("Les clients n'étaient que des habitués, et la salle était pleine."),
            0
        );
        // Le veto ne masque pas une vraie faute : « Il » (nsubj, masc sg) ≠
        // « formés » (masc pl) → toujours signalé.
        assert_eq!(count_full("Il sont formés à vous arnaquer."), 1);
    }

    #[test]
    fn pos_path_still_corrects_real_mismatches() {
        assert_eq!(
            tagged_first("elle est content").as_deref(),
            Some("contente")
        );
        assert_eq!(
            tagged_first("ils sont content").as_deref(),
            Some("contents")
        );
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
    fn pos_path_preposition_attribute_is_ignored() {
        // « sur » est aussi l'adjectif « acide » au lexique, mais ici c'est la
        // préposition (étiquetée ADP) : « elle est sur le côté » ne doit pas
        // devenir « sure ». Idem après « rester » (verbe attributif).
        assert_eq!(tagged_count("elle est sur le côté"), 0);
        assert_eq!(tagged_count("la table est sur le côté"), 0);
        assert_eq!(tagged_count("elle reste sur ses gardes"), 0);
        // Le vrai désaccord adjectival reste corrigé.
        assert_eq!(
            tagged_first("elle est content").as_deref(),
            Some("contente")
        );
    }

    #[test]
    fn pos_path_predicate_noun_after_copula_is_ignored() {
        // « sont la partie… » : « la » est un article introduisant un attribut
        // **nominal**, pas le clitique objet « la ». « partie » (nom) ne doit pas
        // être accordé comme le participe de « partir » (« parti »).
        assert_eq!(
            tagged_count("les données sont la partie la plus visible"),
            0
        );
        assert_eq!(tagged_count("ce sont les meilleurs du groupe"), 0);
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

    // --- Sujet postposé (find_postposed_subject). ---

    #[test]
    fn postposed_subject_triggers_agreement() {
        // Être au futur + participe + sujet postposé : « ma voile » est féminin,
        // « déployé » (Masc Sg) doit devenir « déployée ».
        assert_eq!(
            tagged_first("Sous le vent sera déployé ma voile.").as_deref(),
            Some("déployée")
        );
    }

    #[test]
    fn postposed_subject_already_correct_is_silent() {
        // Déjà accordé.
        assert_eq!(tagged_count("Sous le vent sera déployée ma voile."), 0);
    }

    // --- Sujets coordonnés (accord masculin-l'emporte). ---

    #[test]
    fn coordinated_masc_fem_gives_masc() {
        // Genre mixte : le masculin l'emporte.
        assert_eq!(
            first("des géants et collines étaient prises").as_deref(),
            Some("pris")
        );
        assert_eq!(
            first("les cadres et les toiles étaient prêtes").as_deref(),
            Some("prêts")
        );
    }

    #[test]
    fn coordinated_all_fem_stays_fem() {
        // Tous féminins : le résultat reste féminin.
        assert_eq!(count("les tables et les chaises étaient propres"), 0);
    }

    #[test]
    fn coordinated_all_masc_stays_masc() {
        // Tous masculins : pas d'erreur sur la forme masculine.
        assert_eq!(count("les chats et les chiens étaient contents"), 0);
    }

    #[test]
    fn coordinated_correct_form_is_silent() {
        // Forme déjà correcte (masculin pluriel pour genre mixte) → silence.
        assert_eq!(count("les géants et les collines étaient pris"), 0);
    }

    #[test]
    fn single_subject_still_works() {
        // La phase de coordination ne perturbe pas le cas à sujet unique.
        assert_eq!(first("elle est content").as_deref(), Some("contente"));
        assert_eq!(count("ils sont contents"), 0);
    }
}
