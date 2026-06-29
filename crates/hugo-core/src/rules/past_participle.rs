//! Règle : accord du participe passé employé avec l'auxiliaire « avoir » et un
//! **complément d'objet direct antéposé**.
//!
//! Avec « avoir », le participe passé s'accorde en genre et en nombre avec le
//! COD **lorsque celui-ci précède** le verbe. Deux constructions sont gérées :
//!
//! **1. Pronom clitique COD** (`la`, `les`) immédiatement avant l'auxiliaire :
//!
//! - **les** → pluriel (genre indéterminé) : on propose les deux graphies ;
//! - **la** → féminin singulier.
//!
//! **2. Relative avec « que/qu' »** : l'antécédent (nom commun au genre connu)
//! joue le rôle de COD. Ex. « Les livres que j'ai lu » → « lus ».
//!   Pattern : `[DET] NOM [que/qu'] SUJET avoir PARTICIPE`.
//!   La règle remonte jusqu'au nom immédiatement avant « que/qu' ».
//!
//! Sont volontairement écartés, faute de signal fiable :
//!
//! - `me`/`te`/`nous`/`vous`, qui peuvent être **sujets** ;
//! - `l'`, dont le genre est indéterminé.
//!
//! La construction clitique est confirmée par l'étiquette POS `PRON`
//! ([`Rule::check_tagged`]).

use super::{lexical_sentences, Rule};
use crate::dep::DepRel;
use crate::morpho::{self, Gender, Morph, MorphCategory, Number};
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;
use crate::Suggestion;

/// Vérifie l'accord du participe passé avec « avoir » et un COD antéposé.
pub struct PastParticipleAvoir;

const RULE_ID: &str = "past_participle_avoir";

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

/// Genre (éventuel) et nombre imposés par un pronom clitique objet **non
/// ambigu**. `None` pour les pronoms ambigus (sujet/objet) ou indéterminés.
fn cod_features(text: &str) -> Option<(Option<Gender>, Number)> {
    match normalize(text).as_str() {
        "les" => Some((None, Number::Plural)),
        "la" => Some((Some(Gender::Feminine), Number::Singular)),
        _ => None,
    }
}

/// Valeur unique d'un trait à travers des analyses, ou `None` si contradictoire.
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

/// Genre overrides pour les noms courants absents de Lexique383.
fn gender_override(lemma: &str) -> Option<Gender> {
    Some(match lemma {
        "livre" | "mot" | "film" | "repas" | "vin" | "poème" | "roman" | "devoir"
        | "projet" | "article" | "texte" | "rapport" | "document" | "dossier"
        | "tableau" | "dessin" | "trait" | "cœur"
        | "résultat" | "sujet" | "nombre" | "problème" | "exemple" | "chapitre"
        | "paragraphe" | "programme" | "discours" | "dialogue" | "argument"
        | "téléphone" | "billet" | "cadeau" | "repère" | "récit" | "passage" => {
            Gender::Masculine
        }
        _ => return None,
    })
}

/// Genre et nombre d'un nom commun (via le lexique), ou `None` si ambigu.
/// Les épicènes sont écartés car leur genre ne peut pas trancher l'accord.
fn noun_features(text: &str) -> Option<(Gender, Number)> {
    let nouns: Vec<_> = morpho::lookup(text)
        .into_iter()
        .filter(|m| m.category == MorphCategory::Noun)
        .collect();
    if nouns.is_empty() {
        return None;
    }
    let number = consensus(nouns.iter().map(|m| m.number))?;
    let gender_lex = consensus(nouns.iter().map(|m| m.gender));
    use morpho::Gender::Epicene;
    // Si le lexique donne un genre (non épicène), on l'utilise directement.
    // Sinon, on tente l'override statique pour les noms courants mal renseignés.
    let gender = match gender_lex {
        Some(g) if g != Epicene => g,
        _ => {
            let lemma = nouns[0].lemma.as_str();
            gender_override(lemma).or_else(|| gender_lex.filter(|g| *g != Epicene))?
        }
    };
    Some((gender, number))
}

/// Vrai si la forme est un pronom sujet courant (entre « que » et l'auxiliaire).
fn is_subject_pronoun(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "je" | "j" | "tu" | "il" | "elle" | "on" | "nous" | "vous" | "ils" | "elles"
    )
}

/// Vrai si le jeton est une forme conjuguée de l'auxiliaire « avoir ».
fn is_avoir(text: &str) -> bool {
    morpho::verb_forms(text).iter().any(|v| v.lemma == "avoir")
}

/// Antécédent d'un pronom relatif « que/qu' » situé en `rel` : le nom (commun ou
/// propre) le plus proche à sa gauche, en sautant les adjectifs/déterminants
/// intercalés (« les livres **intéressants** que… » → « livres »). S'arrête au
/// premier mot qui n'est ni nom ni modifieur antéposé (verbe, pronom, virgule).
fn antecedent_before(tokens: &[Token], tags: &[Tagged], rel: usize) -> Option<usize> {
    let mut i = rel;
    while i > 0 {
        i -= 1;
        if !tokens[i].is_lexical() {
            continue;
        }
        match tags[i].upos {
            Upos::Noun | Upos::Propn => return Some(i),
            // Modifieurs antéposés que l'on saute pour atteindre la tête nominale.
            Upos::Adj | Upos::Det | Upos::Adv => continue,
            // Tout autre mot (verbe, pronom, préposition…) : pas d'antécédent.
            _ => return None,
        }
    }
    None
}

/// Vrai si le jeton est un adverbe pouvant s'intercaler entre l'auxiliaire et le
/// participe (« je les ai déjà vus », « je ne les ai pas vus »).
fn is_skippable_adverb(text: &str) -> bool {
    matches!(
        normalize(text).as_str(),
        "pas"
            | "jamais"
            | "plus"
            | "déjà"
            | "bien"
            | "toujours"
            | "encore"
            | "vraiment"
            | "souvent"
            | "tous"
            | "tout"
            | "même"
            | "trop"
    )
}

/// Analyses « participe passé » d'une forme (verbe sans personne, porteur d'un
/// genre et d'un nombre).
///
/// Repli : pour les PP irréguliers dont le masculin singulier est homographe
/// d'une forme conjuguée dans le Lefff (« écrit » = 3e sg prés. + PP m.sg.),
/// on synthétise une entrée PP en croisant la lecture verbale (pour le lemme)
/// et la lecture adjectivale (pour le genre et le nombre).
fn participles(text: &str) -> Vec<Morph> {
    let all = morpho::lookup(text);

    let direct: Vec<_> = all
        .iter()
        .filter(|m| {
            m.category == MorphCategory::Verb
                && m.person.is_none()
                && m.gender.is_some()
                && m.number.is_some()
        })
        .cloned()
        .collect();
    if !direct.is_empty() {
        return direct;
    }

    // Repli : lemme verbal unique + lectures adjectivales portant genre/nombre.
    let verb_lemmas: std::collections::HashSet<&str> = all
        .iter()
        .filter(|m| m.category == MorphCategory::Verb)
        .map(|m| m.lemma.as_str())
        .collect();
    if verb_lemmas.len() != 1 {
        return vec![];
    }
    let lemma = verb_lemmas.into_iter().next().unwrap();
    all.iter()
        .filter(|m| {
            m.category == MorphCategory::Adjective
                && m.gender.is_some()
                && m.number.is_some()
        })
        .map(|m| Morph {
            lemma: lemma.to_string(),
            category: MorphCategory::Verb,
            gender: m.gender,
            number: m.number,
            person: None,
        })
        .collect()
}

/// Lemme commun à toutes ces analyses, ou `None` s'il y en a plusieurs.
fn unique_lemma(analyses: &[Morph]) -> Option<&str> {
    let mut it = analyses.iter().map(|m| m.lemma.as_str());
    let first = it.next()?;
    it.all(|l| l == first).then_some(first)
}

impl PastParticipleAvoir {
    /// Génère les suggestions d'accord pour un participe dont on connaît le
    /// genre (optionnel) et le nombre imposés par le COD.
    fn suggest(
        parts: &[Morph],
        part_token: &Token,
        gender: Option<Gender>,
        number: Number,
        msg: &str,
    ) -> Option<Suggestion> {
        // Déjà accordé ?
        if parts
            .iter()
            .any(|m| m.number == Some(number) && gender.map_or(true, |g| m.gender == Some(g)))
        {
            return None;
        }
        let lemma = unique_lemma(parts)?;
        let genders: &[Gender] = match gender {
            Some(Gender::Feminine) => &[Gender::Feminine],
            Some(Gender::Masculine) => &[Gender::Masculine],
            _ => &[Gender::Masculine, Gender::Feminine],
        };
        let mut replacements: Vec<String> = Vec::new();
        for &g in genders {
            if let Some(form) = morpho::participle(lemma, g, number) {
                let cased = match_case(&part_token.text, &form);
                if !cased.eq_ignore_ascii_case(&part_token.text) && !replacements.contains(&cased) {
                    replacements.push(cased);
                }
            }
        }
        if replacements.is_empty() {
            return None;
        }
        Some(Suggestion {
            span: part_token.span,
            message: msg.to_string(),
            replacements,
            rule_id: RULE_ID,
        })
    }

    /// Cœur de la règle. `pron_ok(idx)` confirme que le jeton d'index d'origine
    /// `idx` est bien un pronom (filtre POS optionnel).
    fn run(&self, tokens: &[Token], pron_ok: impl Fn(usize) -> bool) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for lex in lexical_sentences(tokens) {
            for p in 0..lex.len() {
                let parts = participles(&lex[p].1.text);
                // Repli : pour les formes verbales finies sans lecture PP ni
                // adjectivale (ex. « prit » = PS 3sg de « prendre »), on garde
                // quand même le lemme pour tenter la correction — mais seulement
                // via le chemin relatif (chemin 2) où le contexte est plus sûr.
                let verb_lemma_fallback: Option<String> = if parts.is_empty() {
                    let all = morpho::lookup(&lex[p].1.text);
                    let vl: std::collections::HashSet<&str> = all
                        .iter()
                        .filter(|m| m.category == MorphCategory::Verb)
                        .map(|m| m.lemma.as_str())
                        .collect();
                    if vl.len() == 1 {
                        Some(vl.into_iter().next().unwrap().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                if parts.is_empty() && verb_lemma_fallback.is_none() {
                    continue;
                }

                // Auxiliaire : jeton lexical précédent, en sautant les adverbes.
                let mut a = p;
                let aux = loop {
                    if a == 0 {
                        break None;
                    }
                    a -= 1;
                    if is_skippable_adverb(&lex[a].1.text) {
                        continue;
                    }
                    break Some(a);
                };
                let Some(a) = aux else { continue };
                if !is_avoir(&lex[a].1.text) || a == 0 {
                    continue;
                }

                // --- Chemin 1 : pronom clitique COD (« la »/« les ») avant l'auxiliaire. ---
                let cod = lex[a - 1];
                if let Some((gender, number)) = cod_features(&cod.1.text) {
                    if pron_ok(cod.0) {
                        if let Some(s) = Self::suggest(
                            &parts,
                            lex[p].1,
                            gender,
                            number,
                            &format!(
                                "Accord du participe passé : « {} » doit s'accorder avec le \
                                 complément d'objet direct antéposé.",
                                lex[p].1.text
                            ),
                        ) {
                            suggestions.push(s);
                        }
                    }
                    continue; // clitique trouvé, chemin relatif inapplicable
                }

                // --- Chemin 2 : relatif « que/qu' » — NOM [que/qu'] SUJET avoir PARTICIPE. ---
                // Structure : lex[a-2] = que/qu', lex[a-1] = sujet, lex[a-3…] = nom antécédent.
                if a < 2 {
                    continue;
                }
                let que_tok = &lex[a - 2].1.text;
                if !matches!(normalize(que_tok).as_str(), "que" | "qu") {
                    continue;
                }
                // Le token entre « que » et l'auxiliaire doit être un sujet courant.
                if !is_subject_pronoun(&lex[a - 1].1.text) {
                    continue;
                }
                // Antécédent : nom immédiatement avant « que/qu' ».
                if a < 3 {
                    continue;
                }
                let antecedent = lex[a - 3].1;
                let Some((gender, number)) = noun_features(&antecedent.text) else {
                    continue;
                };
                // Chemin normal : parts non vide → suggestion classique.
                if !parts.is_empty() {
                    if let Some(s) = Self::suggest(
                        &parts,
                        lex[p].1,
                        Some(gender),
                        number,
                        &format!(
                            "Accord du participe passé : « {} » doit s'accorder avec le nom \
                             antécédent « {} » (COD du relatif).",
                            lex[p].1.text, antecedent.text
                        ),
                    ) {
                        suggestions.push(s);
                    }
                } else if let Some(ref lemma) = verb_lemma_fallback {
                    // Repli : forme finie (ex. « prit » = PS) en position PP →
                    // on génère directement la forme correcte attendue.
                    if let Some(expected) = morpho::participle(lemma, gender, number) {
                        if !expected.eq_ignore_ascii_case(&lex[p].1.text) {
                            suggestions.push(Suggestion {
                                span: lex[p].1.span,
                                message: format!(
                                    "Accord du participe passé : « {} » doit s'accorder avec le \
                                     nom antécédent « {} » (COD du relatif).",
                                    lex[p].1.text, antecedent.text
                                ),
                                replacements: vec![match_case(&lex[p].1.text, &expected)],
                                rule_id: RULE_ID,
                            });
                        }
                    }
                }
            }
        }
        suggestions
    }

    /// Chemin **piloté par l'arbre** (production). Le COD antéposé est l'enfant
    /// `obj` du participe situé **avant** lui : un clitique (`la`/`les`), un
    /// relatif (`que`/`qu'` → on remonte à l'antécédent nominal), ou un nom
    /// antéposé (« quels livres as-tu lus »). L'auxiliaire `avoir` est confirmé
    /// par l'arc `aux`. Cette structure capte des cas hors de portée du chemin
    /// positionnel (modifieurs intercalés, adverbes) et **n'accorde jamais avec
    /// un COD postposé** (`j'ai mangé une pomme`), l'arc `obj` étant alors à
    /// droite du participe.
    fn run_tree(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for p in 0..tokens.len() {
            if !tokens[p].is_lexical() {
                continue;
            }
            let parts = participles(&tokens[p].text);
            let verb_lemma_fallback: Option<String> = if parts.is_empty() {
                let all = morpho::lookup(&tokens[p].text);
                let vl: std::collections::HashSet<&str> = all
                    .iter()
                    .filter(|m| m.category == MorphCategory::Verb)
                    .map(|m| m.lemma.as_str())
                    .collect();
                (vl.len() == 1).then(|| vl.into_iter().next().unwrap().to_string())
            } else {
                None
            };
            if parts.is_empty() && verb_lemma_fallback.is_none() {
                continue;
            }

            // Auxiliaire « avoir » : enfant `aux` du participe (exclut « être »).
            let Some(aux) = crate::dep::child_with(tags, p, &[DepRel::Aux]) else {
                continue;
            };
            if !is_avoir(&tokens[aux].text) {
                continue;
            }

            // COD = enfant `obj` **antéposé** du participe.
            let Some(obj) = crate::dep::child_with(tags, p, &[DepRel::Obj]) else {
                continue;
            };
            if obj >= p {
                continue; // COD postposé → pas d'accord avec « avoir ».
            }

            // Genre/nombre du COD selon sa nature.
            let (gender, number, antecedent): (Option<Gender>, Number, Option<usize>) =
                if let Some((g, n)) = cod_features(&tokens[obj].text) {
                    (g, n, None) // clitique « la »/« les »
                } else if matches!(normalize(&tokens[obj].text).as_str(), "que" | "qu") {
                    let Some(ant) = antecedent_before(tokens, tags, obj) else {
                        continue;
                    };
                    let Some((g, n)) = noun_features(&tokens[ant].text) else {
                        continue;
                    };
                    (Some(g), n, Some(ant))
                } else if matches!(tags[obj].upos, Upos::Noun | Upos::Propn) {
                    let Some((g, n)) = noun_features(&tokens[obj].text) else {
                        continue;
                    };
                    (Some(g), n, Some(obj))
                } else {
                    continue; // « l' », « me », « te »… : genre indéterminé.
                };

            let msg = match antecedent {
                Some(ant) => format!(
                    "Accord du participe passé : « {} » doit s'accorder avec le complément \
                     d'objet direct antéposé « {} ».",
                    tokens[p].text, tokens[ant].text
                ),
                None => format!(
                    "Accord du participe passé : « {} » doit s'accorder avec le complément \
                     d'objet direct antéposé.",
                    tokens[p].text
                ),
            };

            if !parts.is_empty() {
                if let Some(s) = Self::suggest(&parts, &tokens[p], gender, number, &msg) {
                    suggestions.push(s);
                }
            } else if let (Some(lemma), Some(g)) = (&verb_lemma_fallback, gender) {
                // Forme finie homographe (« prit ») en position de PP : on
                // engendre directement la forme accordée attendue.
                if let Some(expected) = morpho::participle(lemma, g, number) {
                    if !expected.eq_ignore_ascii_case(&tokens[p].text) {
                        suggestions.push(Suggestion {
                            span: tokens[p].span,
                            message: msg,
                            replacements: vec![match_case(&tokens[p].text, &expected)],
                            rule_id: RULE_ID,
                        });
                    }
                }
            }
        }
        suggestions
    }
}

impl Rule for PastParticipleAvoir {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        self.run(tokens, |_| true)
    }

    fn check_tagged(&self, tokens: &[Token], tags: &[Tagged]) -> Vec<Suggestion> {
        self.run_tree(tokens, tags)
    }

    fn name(&self) -> &'static str {
        "Accord du participe passé (avoir + COD antéposé)"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn run(text: &str) -> Vec<Suggestion> {
        let tokens = tokenize(text);
        // Le chemin de production lit l'arbre : on l'analyse comme le Checker.
        let mut tags = crate::pos::tag(&tokens);
        crate::dep::parse(&tokens, &mut tags);
        PastParticipleAvoir.check_tagged(&tokens, &tags)
    }

    fn first(text: &str) -> Option<Vec<String>> {
        run(text).into_iter().next().map(|s| s.replacements)
    }

    fn count(text: &str) -> usize {
        run(text).len()
    }

    #[test]
    fn les_offers_both_plural_genders() {
        assert_eq!(
            first("je les ai vu").as_deref(),
            Some(["vus".to_string(), "vues".to_string()].as_slice())
        );
    }

    #[test]
    fn les_with_intervening_adverb() {
        assert_eq!(
            first("je ne les ai pas vu").as_deref(),
            Some(["vus".to_string(), "vues".to_string()].as_slice())
        );
    }

    #[test]
    fn la_gives_feminine_singular() {
        assert_eq!(
            first("il la a vu").as_deref(),
            Some(["vue".to_string()].as_slice())
        );
    }

    #[test]
    fn already_agreed_is_silent() {
        for ok in ["je les ai vus", "je les ai vues", "il la a vue"] {
            assert_eq!(count(ok), 0, "faux positif sur « {ok} »");
        }
    }

    #[test]
    fn subject_pronoun_is_not_cod() {
        // « nous »/« vous » sujets : pas de COD antéposé, aucun accord.
        assert_eq!(count("nous avons mangé"), 0);
        assert_eq!(count("vous avez compris"), 0);
    }

    #[test]
    fn etre_auxiliary_is_out_of_scope() {
        // L'accord avec être relève d'une autre règle ; rien ici.
        assert_eq!(count("elles sont parties"), 0);
    }

    #[test]
    fn relative_antecedent_basic() {
        // Antécédent du relatif via l'arbre (obj « que ») + scan nominal.
        assert_eq!(
            first("les livres que j'ai lu").as_deref(),
            Some(["lus".to_string()].as_slice())
        );
    }

    #[test]
    fn relative_antecedent_with_intervening_adjective() {
        // GAIN DE RAPPEL : le chemin positionnel ratait l'antécédent à cause de
        // l'adjectif intercalé ; l'arbre + scan nominal le retrouve.
        assert_eq!(
            first("les livres intéressants que j'ai lu").as_deref(),
            Some(["lus".to_string()].as_slice())
        );
    }

    #[test]
    fn postposed_cod_is_silent() {
        // COD postposé (« une pomme » après le participe) : aucun accord avec
        // « avoir ». L'arc `obj` est à droite du participe → abstention.
        assert_eq!(count("j'ai mangé une pomme"), 0);
        assert_eq!(count("nous avons lu les livres"), 0);
    }
}
