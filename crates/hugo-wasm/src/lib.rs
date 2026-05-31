//! Bindings WebAssembly pour Hugo.
//!
//! Empaqueté avec `wasm-pack` (phase 3) sous le nom npm `hugo-wasm`. Expose une
//! classe [`HugoChecker`] et une fonction utilitaire [`check`].
//!
//! ```js
//! import init, { HugoChecker } from "hugo-wasm";
//! await init();
//! const checker = new HugoChecker();
//! const suggestions = checker.check("il il mange");
//! ```

use hugo_core::{Checker, Suggestion};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Une suggestion sérialisable vers un objet JavaScript.
#[derive(Serialize)]
pub struct JsSuggestion {
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub replacements: Vec<String>,
    #[serde(rename = "ruleId")]
    pub rule_id: String,
}

impl From<Suggestion> for JsSuggestion {
    fn from(s: Suggestion) -> Self {
        JsSuggestion {
            start: s.span.start,
            end: s.span.end,
            message: s.message,
            replacements: s.replacements,
            rule_id: s.rule_id.to_string(),
        }
    }
}

/// Initialise les hooks de panique pour de meilleurs messages d'erreur dans la
/// console du navigateur. Appelé automatiquement au chargement du module.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

/// Correcteur exposé à JavaScript.
#[wasm_bindgen]
pub struct HugoChecker {
    inner: Checker,
}

#[wasm_bindgen]
impl HugoChecker {
    /// Crée un nouveau correcteur.
    #[wasm_bindgen(constructor)]
    pub fn new() -> HugoChecker {
        HugoChecker {
            inner: Checker::new(),
        }
    }

    /// Vérifie un texte et retourne un tableau d'objets suggestion.
    pub fn check(&self, text: &str) -> Result<JsValue, JsValue> {
        let suggestions: Vec<JsSuggestion> =
            self.inner.check(text).into_iter().map(Into::into).collect();
        serde_wasm_bindgen::to_value(&suggestions).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for HugoChecker {
    fn default() -> Self {
        HugoChecker::new()
    }
}

/// Fonction de commodité : vérifie un texte avec un correcteur jetable.
#[wasm_bindgen]
pub fn check(text: &str) -> Result<JsValue, JsValue> {
    HugoChecker::new().check(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_suggestion_conversion() {
        let suggestions = Checker::new().check("il il mange");
        let js: Vec<JsSuggestion> = suggestions.into_iter().map(Into::into).collect();
        assert!(js.iter().any(|s| s.rule_id == "duplicate_word"));
    }
}
