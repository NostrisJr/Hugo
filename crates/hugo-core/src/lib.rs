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

pub mod morpho;
pub mod pos;
pub mod rules;
pub mod spelling;
pub mod tokenizer;

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
    pub fn check(&self, text: &str) -> Vec<Suggestion> {
        let tokens = tokenizer::tokenize(text);
        // Désambiguïsation POS (CRF) : une étiquette unique par token, alignée
        // sur `tokens`. Calculée une seule fois et offerte à chaque règle.
        let tags = pos::tag(&tokens);
        let mut suggestions = Vec::new();

        // Appliquer les règles grammaticales.
        for rule in &self.rules {
            suggestions.extend(rule.check_tagged(&tokens, &tags));
        }

        // Appliquer le correcteur orthographique (Dicollecte).
        suggestions.extend(self.spell_check(&tokens));

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
}
