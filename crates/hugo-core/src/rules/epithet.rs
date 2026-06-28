//! Règle : accord de l'adjectif épithète avec son nom (genre et nombre).
//!
//! Traite l'adjectif **adjacent** à son nom tête, antéposé ou postposé :
//! « les chats noir » → « noirs », « les petit chats » → « petits ».
//!
//! Approche prudente, pour limiter les faux positifs :
//!
//! - l'adjectif et son nom doivent être **immédiatement voisins** (l'adjectif
//!   précède ou suit directement le nom dans le flux lexical) ;
//! - on écarte les **homographes verbaux** (« les volets ferme » : `ferme` est
//!   ici le verbe `fermer`, pas l'adjectif — c'est l'accord sujet–verbe qui
//!   s'applique) ;
//! - la cible de genre est celle du **nom** s'il est connu, sinon on **préserve
//!   le genre de l'adjectif** et l'on ne corrige que le nombre (évite de choisir
//!   un mauvais genre quand Lexique laisse le nom épicène) ;
//! - la forme corrigée est engendrée par [`morpho::decline`].

use super::{is_number_invariable, lexical_sentences, Rule};
use crate::morpho::{self, Gender, Morph, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::{Token, TokenKind};
use crate::Suggestion;

/// Vérifie l'accord en genre et en nombre de l'adjectif épithète avec son nom.
pub struct EpithetAdjectiveAgreement;

const RULE_ID: &str = "epithet_adjective_agreement";

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

/// Vrai si le jeton admet une analyse nominale.
fn is_noun(token: &Token) -> bool {
    morpho::lookup(&token.text)
        .iter()
        .any(|m| m.category == MorphCategory::Noun)
}

/// Vrai si le jeton n'admet **que** des analyses adjectif/déterminant (pas de
/// lecture nominale) : un prémodifieur que l'on peut sauter pour atteindre la
/// tête nominale d'un complément (« marge nette **d'**intérêts » → « marge »).
fn is_modifier_only(token: &Token) -> bool {
    let cats: Vec<_> = morpho::lookup(&token.text)
        .into_iter()
        .map(|m| m.category)
        .collect();
    !cats.is_empty()
        && cats
            .iter()
            .all(|c| matches!(c, MorphCategory::Adjective | MorphCategory::Determiner))
}

/// Minuscules + apostrophe finale ôtée (« d' » → « d »).
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Prépositions « de » introduisant un complément du nom (« outils **de**
/// travail », « sac **du** voisin »).
fn is_de(text: &str) -> bool {
    matches!(normalize(text).as_str(), "de" | "d" | "du" | "des")
}

/// Genre (s'il est sûr) et nombre du nom à l'index lexical `h`, ou `None` si le
/// nombre est indéterminé.
fn noun_features(lex: &[(usize, &Token)], h: usize) -> Option<(Option<Gender>, Number)> {
    let nouns: Vec<_> = morpho::lookup(&lex[h].1.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    let number = consensus(nouns.iter().map(|m| m.number))?;
    let gender = consensus(nouns.iter().map(|m| m.gender)).filter(|g| *g != Gender::Epicene);
    Some((gender, number))
}

/// Vrai si l'une des analyses adjectivales s'accorde avec (`gender`, `number`).
/// Le genre n'est contraint que s'il est connu.
fn adj_agrees(adjectives: &[Morph], gender: Option<Gender>, number: Number) -> bool {
    adjectives.iter().any(|m| {
        m.number
            .map_or(true, |n| n == number || n == Number::Invariable)
            && match gender {
                Some(g) => m.gender.map_or(true, |mg| mg == g || mg == Gender::Epicene),
                None => true,
            }
    })
}

/// Vrai si le jeton admet une analyse verbale finie (forme conjuguée).
fn is_finite_verb(token: &Token) -> bool {
    !morpho::verb_forms(&token.text).is_empty()
}

/// Vrai si une virgule sépare les tokens à l'index `a` et `b` dans la tranche
/// d'origine. Utilisée pour empêcher l'accord d'un adjectif avec un nom qui
/// se trouve de l'autre côté d'une virgule (listes nominales en apposition).
fn comma_between(tokens: &[Token], orig_a: usize, orig_b: usize) -> bool {
    let (lo, hi) = if orig_a < orig_b {
        (orig_a + 1, orig_b)
    } else {
        (orig_b + 1, orig_a)
    };
    tokens[lo..hi]
        .iter()
        .any(|t| t.kind == TokenKind::Punctuation && t.text == ",")
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

/// Vrai si l'une des analyses adjectivales de `adj_token` s'accorde (genre connu
/// + nombre) avec `noun_token`. Sert à détecter les rattachements ambigus :
/// si l'adjectif s'accorde avec un **autre** nom du groupe nominal que sa tête
/// `amod`, la correction n'est pas sûre.
fn adj_agrees_with_token(adj_token: &Token, noun_token: &Token) -> bool {
    let adjs: Vec<_> = morpho::lookup(&adj_token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Adjective)
        .collect();
    if adjs.is_empty() {
        return false;
    }
    let nouns: Vec<_> = morpho::lookup(&noun_token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    let Some(number) = consensus(nouns.iter().map(|m| m.number)) else {
        return false;
    };
    let gender = consensus(nouns.iter().map(|m| m.gender)).filter(|g| *g != Gender::Epicene);
    adj_agrees(&adjs, gender, number)
}

/// À partir d'un adjectif et de son **nom tête**, renvoie la correction d'accord
/// nécessaire, ou `None` si l'accord est correct, ambigu (lemme ou genre/nombre
/// indéterminé) ou hors de portée. Partagé par le chemin d'adjacence (sans tags)
/// et le chemin piloté par l'arbre de dépendances (`amod`).
fn agreement_suggestion(adj_token: &Token, noun_token: &Token) -> Option<Suggestion> {
    // L'adjectif ne doit pas être un homographe verbal conjugué.
    if is_finite_verb(adj_token) {
        return None;
    }
    let adjectives: Vec<_> = morpho::lookup(&adj_token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Adjective)
        .collect();
    if adjectives.is_empty() {
        return None;
    }

    let nouns: Vec<_> = morpho::lookup(&noun_token.text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    let noun_gender = consensus(nouns.iter().map(|m| m.gender));
    // Nom **invariable en nombre** (« fois », « prix ») : on garde le nombre de
    // l'adjectif ; sinon on prend celui du nom (abstention si indéterminé).
    let number = if is_number_invariable(&nouns) {
        consensus(adjectives.iter().map(|m| m.number))?
    } else {
        consensus(nouns.iter().map(|m| m.number))?
    };

    // Genre cible : celui du nom s'il est connu, sinon celui de l'adjectif.
    let adj_gender = consensus(adjectives.iter().map(|m| m.gender));
    let gender = noun_gender.or(adj_gender)?;
    if gender == Gender::Epicene {
        return None;
    }

    // Déjà accordé ? (une analyse compatible suffit)
    let agrees = adjectives.iter().any(|m| {
        m.gender.map_or(true, |g| g == gender || g == Gender::Epicene)
            && m.number.map_or(true, |n| n == number || n == Number::Invariable)
    });
    if agrees {
        return None;
    }

    // Lemme unique (sinon ambiguïté lexicale).
    let mut lemmas: Vec<&str> = adjectives.iter().map(|m| m.lemma.as_str()).collect();
    lemmas.sort_unstable();
    lemmas.dedup();
    if lemmas.len() != 1 {
        return None;
    }
    let lemma = adjectives[0].lemma.clone();

    let corrected = morpho::decline(&lemma, gender, number)?;
    if corrected.eq_ignore_ascii_case(&adj_token.text) {
        return None;
    }
    Some(Suggestion {
        span: adj_token.span,
        message: format!(
            "Accord de l'adjectif : « {} » ne s'accorde pas avec « {} ».",
            adj_token.text, noun_token.text
        ),
        replacements: vec![match_case(&adj_token.text, &corrected)],
        rule_id: RULE_ID,
    })
}

impl EpithetAdjectiveAgreement {
    /// Cœur de la règle.
    ///
    /// `adj_ok(idx)` valide que le candidat épithète (d'index d'origine `idx`)
    /// est bien employé comme adjectif. C'est le filtre POS qui distingue le
    /// **participe présent** (verbal, invariable : « les champs bruissant sous la
    /// brise ») de l'**adjectif verbal** homographe (accordable : « des voyages
    /// fatigants ») — le CRF tague le premier `VERB`, le second `ADJ`. Sans tags,
    /// le lexique seul ne tranche pas (les deux lectures coexistent) ; le chemin
    /// non taggé passe alors `|_| true` et reste « meilleur effort ».
    ///
    /// `noun_ok(idx)` valide de même qu'un candidat nom tête en est bien un —
    /// évite les homographes nom/verbe comme « est » (l'Est vs l'auxiliaire être).
    fn run(
        &self,
        tokens: &[Token],
        adj_ok: impl Fn(usize) -> bool,
        noun_ok: impl Fn(usize) -> bool,
    ) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for lex in lexical_sentences(tokens) {
            for a in 0..lex.len() {
                let adj_token = lex[a].1;

                // Le candidat doit être employé comme adjectif ici (filtre POS) :
                // écarte le participe présent verbal (« champs bruissant sous… »,
                // tagué VERB) tout en gardant l'adjectif verbal (« voyages
                // fatigants », tagué ADJ). Cf. `adj_ok`.
                if !adj_ok(lex[a].0) {
                    continue;
                }

                // L'adjectif ne doit pas être un homographe verbal conjugué.
                if is_finite_verb(adj_token) {
                    continue;
                }
                let adjectives: Vec<_> = morpho::lookup(&adj_token.text)
                    .into_iter()
                    .filter(|m| m.category == MorphCategory::Adjective)
                    .collect();
                if adjectives.is_empty() {
                    continue;
                }

                // Nom tête adjacent : antéposé (nom suivant) ou postposé (nom
                // précédent), dans cet ordre de priorité.
                // Cas particulier : PRÉDET + DET + NOM (ex. « tous les matins »,
                // « aucune des erreurs »). Le prédet s'accorde avec le NOM, non
                // avec un nom précédent. On restreint au jeu fermé de prédet.
                const PREDETERMINERS: &[&str] =
                    &["tout", "toute", "tous", "toutes", "aucun", "aucune", "nul", "nulle",
                      "chaque", "quelque", "quelques", "certain", "certaine", "certains",
                      "certaines", "plusieurs", "même", "mêmes", "autre", "autres"];
                // Une virgule entre l'adjectif et le candidat nom coupe le lien
                // d'accord : « applications tierces, processus » → « tierces »
                // s'accorde avec « applications », pas avec « processus ».
                let noun_pos =
                    if a + 1 < lex.len()
                        && is_noun(lex[a + 1].1)
                        && noun_ok(lex[a + 1].0)
                        && !comma_between(tokens, lex[a].0, lex[a + 1].0)
                    {
                        a + 1
                    } else if PREDETERMINERS.contains(&adj_token.text.to_lowercase().as_str())
                        && a + 2 < lex.len()
                        && morpho::lookup(&lex[a + 1].1.text)
                            .iter()
                            .any(|m| m.category == MorphCategory::Determiner)
                        && is_noun(lex[a + 2].1)
                        && noun_ok(lex[a + 2].0)
                        && !comma_between(tokens, lex[a].0, lex[a + 2].0)
                    {
                        a + 2
                    } else if a > 0
                        && is_noun(lex[a - 1].1)
                        && noun_ok(lex[a - 1].0)
                        && !comma_between(tokens, lex[a - 1].0, lex[a].0)
                    {
                        a - 1
                    } else {
                        continue;
                    };
                let noun_token = lex[noun_pos].1;

                // Rattachement ambigu « N1 (de Nᵢ)+ ADJ » : un adjectif
                // **postposé** peut s'accorder avec **n'importe quelle** tête
                // nominale en remontant la **chaîne** de compléments « de N »
                // (« outils de travail de nuit professionnels » accorde
                // « professionnels » avec « outils », par-delà « travail » et
                // « nuit »). S'il s'accorde avec l'une d'elles, on s'abstient — ni
                // le lexique ni le CRF ne tranchent le rattachement (précision >
                // rappel).
                if noun_pos + 1 == a {
                    let mut n = noun_pos;
                    let mut agrees_head = false;
                    // Tant qu'on est précédé d'un « de » : la tête du complément
                    // peut porter ses propres prémodifieurs (« marge nette d'… »),
                    // que l'on saute pour atteindre le nom (déterminants/adjectifs).
                    while n >= 2 && is_de(&lex[n - 1].1.text) {
                        let mut h = n - 2;
                        while h > 0 && !is_noun(lex[h].1) && is_modifier_only(lex[h].1) {
                            h -= 1;
                        }
                        if !(is_noun(lex[h].1) && noun_ok(lex[h].0)) {
                            break;
                        }
                        if let Some((g1, num1)) = noun_features(&lex, h) {
                            if adj_agrees(&adjectives, g1, num1) {
                                agrees_head = true;
                                break;
                            }
                        }
                        n = h;
                    }
                    if agrees_head {
                        continue;
                    }
                }

                if let Some(s) = agreement_suggestion(adj_token, noun_token) {
                    suggestions.push(s);
                }
            }
        }

        suggestions
    }

    /// Variante **pilotée par l'arbre de dépendances** (chemin de production).
    ///
    /// L'adjectif épithète est relié à son nom tête par la relation `amod` : le
    /// parser l'a déjà rattaché au bon nom, **par-delà** les virgules d'une
    /// énumération (« applications tierces, processus métiers ») et les
    /// compléments « de N » (« outils de travail professionnels »). Plus besoin
    /// d'heuristiques d'adjacence, de `comma_between`, ni de remontée de chaîne :
    /// on lit la structure. On exige tout de même la confirmation morphologique
    /// du désaccord (double garde arc + morphologie) pour rester robuste aux
    /// erreurs du parser.
    fn run_tree(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for i in 0..tokens.len() {
            if !tokens[i].is_lexical() || tags[i].upos != Upos::Adj {
                continue;
            }
            // Tête nominale via l'arc `amod` (None si l'adjectif n'est pas épithète).
            let Some(h) = crate::dep::modified_noun(tags, i) else {
                continue;
            };
            if !matches!(tags[h].upos, Upos::Noun | Upos::Propn) {
                continue;
            }
            // Attachement ambigu « N1 de N2 ADJ » : si un **autre nom** s'intercale
            // entre l'adjectif et sa tête `amod`, le rattachement n'est pas sûr
            // (« un sac de pommes verte » : sac ? pommes ?). On s'abstient
            // (précision > rappel) — remplace l'ancienne remontée de chaîne « de N ».
            let (lo, hi) = (i.min(h), i.max(h));
            let intervening_noun =
                ((lo + 1)..hi).any(|k| matches!(tags[k].upos, Upos::Noun | Upos::Propn));
            if intervening_noun {
                continue;
            }

            // Rattachement « N1 de N2 ADJ » où le parser a accroché l'adjectif à
            // N2 (adjacent) alors qu'il qualifie peut-être N1 (« les moyens de
            // transport publics » : publics ~ moyens, pas transport). Si
            // l'adjectif s'accorde avec un **nom ancêtre** de sa tête, le
            // rattachement est ambigu → abstention.
            let mut ancestor = crate::dep::head_of(tags, h);
            let mut ambiguous = false;
            for _ in 0..tags.len() {
                let Some(a) = ancestor else { break };
                if matches!(tags[a].upos, Upos::Noun | Upos::Propn)
                    && adj_agrees_with_token(&tokens[i], &tokens[a])
                {
                    ambiguous = true;
                    break;
                }
                ancestor = crate::dep::head_of(tags, a);
            }
            if ambiguous {
                continue;
            }

            if let Some(s) = agreement_suggestion(&tokens[i], &tokens[h]) {
                suggestions.push(s);
            }
        }
        suggestions
    }
}

impl Rule for EpithetAdjectiveAgreement {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        // Sans tags, le lexique seul ne distingue pas participe présent et
        // adjectif verbal : on accepte tout candidat (« meilleur effort »).
        self.run(tokens, |_| true, |_| true)
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        self.run_tree(tokens, tags)
    }

    fn name(&self) -> &'static str {
        "Accord de l'adjectif épithète"
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
        EpithetAdjectiveAgreement
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        EpithetAdjectiveAgreement.check(&tokenize(text)).len()
    }

    /// Étiquettes complètes (POS + dépendances) comme en production.
    fn full_tags(tokens: &[crate::tokenizer::Token]) -> Vec<crate::pos::Tagged> {
        let mut tags = crate::pos::tag(tokens);
        crate::dep::parse(tokens, &mut tags);
        tags
    }

    /// Comptage via le chemin taggé (`check_tagged`), tel qu'utilisé en production.
    fn count_tagged(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = full_tags(&tokens);
        EpithetAdjectiveAgreement.check_tagged(&tokens, &tags).len()
    }

    /// Première suggestion via le chemin taggé (`check_tagged`).
    fn first_tagged(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = full_tags(&tokens);
        EpithetAdjectiveAgreement
            .check_tagged(&tokens, &tags)
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    #[test]
    fn postnominal_number() {
        // « les chats noir » → « noirs ».
        assert_eq!(first("les chats noir").as_deref(), Some("noirs"));
    }

    #[test]
    fn prenominal_number() {
        // « les petit chats » → « petits ».
        assert_eq!(first("les petit chats").as_deref(), Some("petits"));
    }

    #[test]
    fn prenominal_gender() {
        // « un beau table » → « belle » (table est féminin).
        assert_eq!(first("un beau table").as_deref(), Some("belle"));
    }

    #[test]
    fn correct_agreement_is_silent() {
        for ok in [
            "les chats noirs",
            "un beau chat",
            "une belle table",
            "le chat noir",
            "les petits chats",
        ] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn unknown_gender_noun_keeps_adjective_gender() {
        // « maison » a un genre vide dans Lexique : « une grande maison » est
        // déjà accordé en nombre et le genre de l'adjectif est préservé.
        assert_eq!(count("une grande maison"), 0);
        // En revanche le nombre est corrigé : « les grande maisons » → « grandes ».
        assert_eq!(first("les grande maisons").as_deref(), Some("grandes"));
    }

    #[test]
    fn present_participle_is_invariable_via_pos() {
        // La distinction participe présent (invariable) / adjectif verbal
        // (accordable) est **syntaxique**, pas lexicale : « bruissant » a la même
        // graphie dans les deux emplois. Seul le tag du CRF la tranche — c'est
        // pourquoi le gate est POS et non une heuristique de surface.
        // Participe présent (tagué VERB), invariable :
        assert_eq!(count_tagged("Les forêts bruissant sous la brise"), 0);
        assert_eq!(count_tagged("les champs bruissant sous la brise"), 0);
        assert_eq!(count_tagged("des enfants jouant dans le jardin"), 0);
        // Même graphie en emploi adjectival (tagué ADJ) : déjà accordé, silencieux.
        assert_eq!(count_tagged("Les forêts sont bruissantes sous la brise"), 0);
        // Participe présent « fatiguant » (avec u) gouvernant un objet : invariable.
        assert_eq!(
            count_tagged("les filles fatiguant leur père sont fatigantes"),
            0
        );
        // Adjectif verbal mal accordé (tagué ADJ) : toujours corrigé.
        assert_eq!(
            first_tagged("les voyages fatigant").as_deref(),
            Some("fatigants")
        );
    }

    #[test]
    fn de_complement_attachment_is_ambiguous() {
        // « N1 de N2 ADJ » : l'adjectif postposé peut s'accorder avec la tête
        // N1 (« outils ») par-delà le complément « de N2 » (« travail »). Comme
        // « professionnels » (pluriel) s'accorde avec « outils » (pluriel), on
        // s'abstient au lieu de le « corriger » vers « travail » (singulier).
        assert_eq!(count_tagged("des outils de travail professionnels"), 0);
        assert_eq!(count_tagged("les moyens de transport publics"), 0);
        assert_eq!(count_tagged("une salle de jeux spacieuse"), 0);
        // Chaîne de compléments « de N de N » : l'accord avec la tête lointaine
        // « outils » (par-delà « travail » puis « nuit ») suffit à s'abstenir.
        assert_eq!(
            count_tagged("les outils de travail de nuit professionnels"),
            0
        );
        // Attachement « N1 de N2 ADJ » authentiquement ambigu : le parser
        // rattache « verte » à « sac » (un sac vert), un nom s'intercale
        // (« pommes ») → on s'abstient au lieu de risquer une correction vers le
        // mauvais nom. Précision > rappel.
        assert_eq!(count_tagged("un sac de pommes verte"), 0);
    }

    #[test]
    fn enumeration_with_commas_no_false_positive() {
        // Énumération nominale séparée par des virgules : chaque adjectif
        // s'accorde avec SON nom (« tierces » ~ « applications », « légales » ~
        // « contraintes »), jamais avec le nom voisin de l'autre côté de la
        // virgule. L'arbre (`amod`) le sait — plus de faux positif, sans recourir
        // à une heuristique de virgule.
        assert_eq!(
            count_tagged("Les applications tierces, processus métiers, contraintes légales"),
            0
        );
        assert_eq!(count_tagged("des données fiables, traitements rapides"), 0);
    }

    #[test]
    fn verb_homograph_is_not_flagged() {
        // « les volets ferme » : « ferme » est le verbe (accord sujet–verbe),
        // pas un adjectif épithète → la règle ne touche à rien.
        assert_eq!(count("les volets ferme"), 0);
        // Idem, un vrai verbe après le nom ne déclenche pas.
        assert_eq!(count("les chats dorment"), 0);
    }
}
