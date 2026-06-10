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

use super::{lexical_sentences, Rule};
use crate::morpho::{self, Gender, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
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

/// Vrai si le jeton admet une analyse verbale finie (forme conjuguée).
fn is_finite_verb(token: &Token) -> bool {
    !morpho::verb_forms(&token.text).is_empty()
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
                let noun_token =
                    if a + 1 < lex.len() && is_noun(lex[a + 1].1) && noun_ok(lex[a + 1].0) {
                        lex[a + 1].1
                    } else if a > 0 && is_noun(lex[a - 1].1) && noun_ok(lex[a - 1].0) {
                        lex[a - 1].1
                    } else {
                        continue;
                    };

                let nouns: Vec<_> = morpho::lookup(&noun_token.text)
                    .into_iter()
                    .filter(|m| m.category == MorphCategory::Noun)
                    .collect();
                let noun_gender = consensus(nouns.iter().map(|m| m.gender));
                let Some(number) = consensus(nouns.iter().map(|m| m.number)) else {
                    continue; // nombre du nom indéterminé → on s'abstient
                };

                // Genre cible : celui du nom s'il est connu, sinon on préserve
                // celui de l'adjectif (correction de nombre seule).
                let adj_gender = consensus(adjectives.iter().map(|m| m.gender));
                let Some(gender) = noun_gender.or(adj_gender) else {
                    continue;
                };
                if gender == Gender::Epicene {
                    continue;
                }

                // Déjà accordé ? (une analyse compatible suffit)
                let agrees = adjectives.iter().any(|m| {
                    m.gender
                        .map_or(true, |g| g == gender || g == Gender::Epicene)
                        && m.number
                            .map_or(true, |n| n == number || n == Number::Invariable)
                });
                if agrees {
                    continue;
                }

                // Lemme unique (sinon ambiguïté lexicale).
                let mut lemmas: Vec<&str> = adjectives.iter().map(|m| m.lemma.as_str()).collect();
                lemmas.sort_unstable();
                lemmas.dedup();
                if lemmas.len() != 1 {
                    continue;
                }
                let lemma = adjectives[0].lemma.clone();

                let Some(corrected) = morpho::decline(&lemma, gender, number) else {
                    continue;
                };
                if corrected.eq_ignore_ascii_case(&adj_token.text) {
                    continue;
                }

                suggestions.push(Suggestion {
                    span: adj_token.span,
                    message: format!(
                        "Accord de l'adjectif : « {} » ne s'accorde pas avec « {} ».",
                        adj_token.text, noun_token.text
                    ),
                    replacements: vec![match_case(&adj_token.text, &corrected)],
                    rule_id: RULE_ID,
                });
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
        self.run(
            tokens,
            |idx| tags[idx].upos == Upos::Adj,
            |idx| matches!(tags[idx].upos, Upos::Noun | Upos::Propn),
        )
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

    /// Comptage via le chemin POS (`check_tagged`), tel qu'utilisé en production.
    fn count_tagged(text: &str) -> usize {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
        EpithetAdjectiveAgreement.check_tagged(&tokens, &tags).len()
    }

    /// Première suggestion via le chemin POS (`check_tagged`).
    fn first_tagged(text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let tags = crate::pos::tag(&tokens);
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
        assert_eq!(first_tagged("les voyages fatigant").as_deref(), Some("fatigants"));
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
