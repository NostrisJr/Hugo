//! # Hugo — correcteur orthographique et grammatical français
//!
//! `hugo-core` est la bibliothèque centrale du projet Hugo. Elle fournit un
//! correcteur français fonctionnant **entièrement en local**, sans réseau ni
//! LLM. Le pipeline combine :
//!
//! - un [`tokenizer`] français robuste (élisions, tirets d'inversion, nombres,
//!   ponctuation, Unicode) ;
//! - une analyse morphologique ([`morpho`], adossée au Lefff — en cours) ;
//! - un moteur de [`rules`] grammaticales écrites en Rust pur ;
//! - un correcteur orthographique ([`spelling`], adossé à Dicollecte — en cours).
//!
//! Le point d'entrée est [`Checker`].
//!
//! ```
//! use hugo_core::Checker;
//!
//! let checker = Checker::new();
//! let suggestions = checker.check("il il mange");
//! assert!(!suggestions.is_empty());
//! ```

pub mod clause;
pub mod dep;
pub mod morpho;
pub mod pos;
pub mod rules;
pub mod spelling;
pub mod tokenizer;

pub use dep::DepRel;
pub use morpho::{Morph, MorphCategory, Number, Person};
pub use pos::{Tagged, Upos};
pub use rules::Rule;
pub use spelling::SpellChecker;
pub use tokenizer::{Token, TokenKind};

/// Identifiant de règle des suggestions orthographiques.
pub const SPELLING_RULE_ID: &str = "spelling";

/// Nombre de corrections orthographiques proposées par mot inconnu.
const SPELLING_SUGGESTIONS: usize = 5;

/// Position d'un fragment dans le texte source, en **offsets d'octets**.
///
/// Les bornes sont à comprendre comme la plage `start..end` (demi-ouverte) et
/// pointent toujours sur des frontières de caractères UTF-8 valides, de sorte
/// que `&texte[span.start..span.end]` ne panique jamais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span {
    /// Offset d'octet du premier caractère (inclus).
    pub start: usize,
    /// Offset d'octet juste après le dernier caractère (exclu).
    pub end: usize,
}

impl Span {
    /// Construit un span à partir de ses bornes.
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Longueur du span en octets.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Indique si le span est vide (longueur nulle).
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// Une suggestion de correction produite par une règle ou le correcteur
/// orthographique.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Suggestion {
    /// Position de l'erreur dans le texte source.
    pub span: Span,
    /// Message lisible expliquant l'erreur.
    pub message: String,
    /// Corrections possibles, triées de la plus à la moins pertinente.
    pub replacements: Vec<String>,
    /// Identifiant stable de la règle ayant produit cette suggestion.
    pub rule_id: &'static str,
}

/// Métadonnées d'une règle : identifiant stable et nom lisible.
///
/// Produit par [`Checker::rule_catalog`] pour alimenter une interface
/// d'activation/désactivation des règles.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RuleInfo {
    /// Identifiant stable de la règle (à passer à [`CheckOptions`]).
    pub id: &'static str,
    /// Nom lisible de la règle.
    pub name: &'static str,
}

/// Options d'une vérification : ensemble des règles **désactivées**.
///
/// Par défaut, toutes les règles sont actives (ensemble vide). On désigne une
/// règle par son `rule_id` (cf. [`Checker::rule_catalog`]). Choix d'une
/// liste de *désactivation* (plutôt que d'activation) : une règle nouvellement
/// ajoutée est active par défaut, sans rien casser côté hôte.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckOptions {
    disabled: std::collections::HashSet<String>,
}

impl CheckOptions {
    /// Options par défaut : toutes les règles actives.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construit des options désactivant les règles dont l'identifiant est
    /// fourni.
    pub fn with_disabled<I, S>(rules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            disabled: rules.into_iter().map(Into::into).collect(),
        }
    }

    /// Désactive une règle (chaînable).
    pub fn disable(mut self, rule_id: impl Into<String>) -> Self {
        self.disabled.insert(rule_id.into());
        self
    }

    /// Vrai si la règle d'identifiant `rule_id` est active.
    pub fn is_enabled(&self, rule_id: &str) -> bool {
        !self.disabled.contains(rule_id)
    }
}

/// Point d'entrée principal du correcteur.
///
/// Un `Checker` est immuable une fois construit et `Send + Sync` : il peut être
/// partagé entre threads (par exemple via `app.manage()` côté Tauri) et
/// réutilisé pour vérifier autant de textes que nécessaire.
pub struct Checker {
    rules: Vec<Box<dyn Rule>>,
    speller: SpellChecker,
}

impl Checker {
    /// Construit un correcteur avec l'ensemble des règles actives par défaut et
    /// le correcteur orthographique embarqué.
    pub fn new() -> Self {
        Checker {
            rules: rules::all_rules(),
            speller: SpellChecker::new(),
        }
    }

    /// Donne accès au correcteur orthographique sous-jacent.
    pub fn speller(&self) -> &SpellChecker {
        &self.speller
    }

    /// Vérifie un texte et retourne les suggestions, triées par position.
    ///
    /// Toutes les règles (et le correcteur orthographique) sont actives. Pour
    /// en désactiver certaines, voir [`Checker::check_with`].
    pub fn check(&self, text: &str) -> Vec<Suggestion> {
        self.check_with(text, &CheckOptions::default())
    }

    /// Vérifie un texte en n'appliquant que les règles **activées** par
    /// `options` (toutes par défaut). Les suggestions sont triées par position.
    ///
    /// Une règle est identifiée par son `rule_id` (cf. [`Checker::rule_catalog`]
    /// pour la liste complète) ; le correcteur orthographique répond à
    /// l'identifiant [`SPELLING_RULE_ID`]. Permet à une application hôte (plugin
    /// Tauri…) de laisser l'utilisateur activer/désactiver chaque famille.
    pub fn check_with(&self, text: &str, options: &CheckOptions) -> Vec<Suggestion> {
        let tokens = tokenizer::tokenize(text);
        // Désambiguïsation POS (CRF) : une étiquette unique par token, alignée
        // sur `tokens`. Calculée une seule fois et offerte à chaque règle.
        let mut tags = pos::tag(&tokens);
        // Analyse en dépendances : renseigne head + relation de chaque token,
        // offrant aux règles la structure syntaxique de la phrase.
        dep::parse(&tokens, &mut tags);
        let mut suggestions = Vec::new();

        // Appliquer les règles grammaticales et typographiques activées.
        for rule in &self.rules {
            if options.is_enabled(rule.id()) {
                suggestions.extend(rule.check_tagged(&tokens, &tags));
            }
        }

        // Appliquer le correcteur orthographique (Dicollecte) s'il est activé.
        if options.is_enabled(SPELLING_RULE_ID) {
            suggestions.extend(self.spell_check(&tokens));
        }

        // Trier par position de départ, puis par identifiant de règle pour un
        // ordre stable et reproductible.
        suggestions.sort_by(|a, b| {
            a.span
                .start
                .cmp(&b.span.start)
                .then_with(|| a.rule_id.cmp(b.rule_id))
        });
        suggestions
    }

    /// Catalogue de toutes les règles disponibles (identifiant + nom lisible),
    /// correcteur orthographique inclus. Destiné aux interfaces qui proposent
    /// d'activer/désactiver chaque règle (cf. [`CheckOptions`]).
    pub fn rule_catalog() -> Vec<RuleInfo> {
        let mut catalog: Vec<RuleInfo> = rules::all_rules()
            .iter()
            .map(|r| RuleInfo {
                id: r.id(),
                name: r.name(),
            })
            .collect();
        catalog.push(RuleInfo {
            id: SPELLING_RULE_ID,
            name: "Orthographe",
        });
        catalog
    }

    /// Étiquette morphosyntaxiquement un texte (désambiguïsation POS par CRF).
    ///
    /// Renvoie une étiquette par token, **alignée** sur la tokenisation de
    /// `text`. Utile au débogage, aux tests, et au futur câblage des règles sur
    /// une catégorie unique.
    pub fn tag(&self, text: &str) -> Vec<Tagged> {
        let tokens = tokenizer::tokenize(text);
        pos::tag(&tokens)
    }

    /// Signale les mots absents du dictionnaire et propose des corrections.
    fn spell_check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for token in tokens {
            if token.kind != TokenKind::Word || !is_spellable(&token.text) {
                continue;
            }
            if self.speller.contains(&token.text) {
                continue;
            }
            suggestions.push(Suggestion {
                span: token.span,
                message: format!("Mot inconnu : « {} ».", token.text),
                replacements: self.speller.suggest(&token.text, SPELLING_SUGGESTIONS),
                rule_id: SPELLING_RULE_ID,
            });
        }
        suggestions
    }
}

/// Heuristiques limitant les faux positifs du correcteur orthographique : on ne
/// vérifie que des mots « ordinaires ».
fn is_spellable(word: &str) -> bool {
    let chars: Vec<char> = word.chars().collect();
    // Trop court pour être jugé fiablement.
    if chars.len() < 2 {
        return false;
    }
    // Composés à trait d'union ou apostrophe interne : laissés de côté pour le
    // POC (le tokenizer gère déjà les élisions séparément).
    if word.contains('-') || word.contains('\'') || word.contains('\u{2019}') {
        return false;
    }
    // Acronymes tout en majuscules (SNCF, RATP…).
    if chars.iter().all(|c| !c.is_lowercase()) {
        return false;
    }
    // Doit être entièrement alphabétique.
    chars.iter().all(|c| c.is_alphabetic())
}

impl Default for Checker {
    fn default() -> Self {
        Checker::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_detects_duplicate() {
        let checker = Checker::new();
        let suggestions = checker.check("il il mange");
        assert!(suggestions.iter().any(|s| s.rule_id == "duplicate_word"));
    }

    #[test]
    fn checker_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Checker>();
    }

    #[test]
    fn empty_text_yields_no_panic() {
        let checker = Checker::new();
        assert!(checker.check("").is_empty());
    }

    #[test]
    fn span_helpers() {
        let s = Span::new(2, 5);
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());
        assert!(Span::new(4, 4).is_empty());
    }

    #[test]
    fn disabling_a_rule_suppresses_its_suggestions() {
        let checker = Checker::new();
        let options = CheckOptions::new().disable("duplicate_word");
        let suggestions = checker.check_with("il il mange", &options);
        assert!(suggestions.iter().all(|s| s.rule_id != "duplicate_word"));
    }

    #[test]
    fn disabling_spelling_suppresses_unknown_words() {
        let checker = Checker::new();
        let options = CheckOptions::with_disabled([SPELLING_RULE_ID]);
        let suggestions = checker.check_with("xyzqwk", &options);
        assert!(suggestions.iter().all(|s| s.rule_id != SPELLING_RULE_ID));
    }

    #[test]
    fn rule_catalog_lists_known_rules() {
        let catalog = Checker::rule_catalog();
        assert!(catalog.iter().any(|r| r.id == SPELLING_RULE_ID));
        // Pas de doublon d'identifiant.
        let mut ids: Vec<&str> = catalog.iter().map(|r| r.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
