//! Plugin Tauri v2 pour Hugo.
//!
//! Enregistre la commande `check_text` et gère un unique [`Checker`] partagé
//! via l'état de l'application.
//!
//! ```rust,ignore
//! tauri::Builder::default()
//!     .plugin(hugo_tauri::init())
//!     .run(tauri::generate_context!())
//!     .expect("erreur au démarrage de Tauri");
//! ```
//!
//! Côté front :
//!
//! ```js
//! import { invoke } from "@tauri-apps/api/core";
//! const suggestions = await invoke("plugin:hugo-tauri|check_text", { text: "il il mange" });
//! ```
//!
//! Le nom d'exécution du plugin (`hugo-tauri`) coïncide volontairement avec le
//! nom de crate, car c'est ce dernier que `tauri-plugin` utilise comme espace
//! de noms des permissions ACL (`hugo-tauri:allow-check-text`). Les deux doivent
//! être identiques pour que la commande soit autorisée.

use hugo_core::Checker;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

/// Une suggestion sérialisée pour le front-end.
#[derive(serde::Serialize)]
pub struct JsSuggestion {
    /// Offset d'octet de début dans le texte source.
    pub start: usize,
    /// Offset d'octet de fin (exclu).
    pub end: usize,
    /// Message explicatif.
    pub message: String,
    /// Corrections proposées, triées par pertinence.
    pub replacements: Vec<String>,
    /// Identifiant de la règle.
    #[serde(rename = "ruleId")]
    pub rule_id: String,
}

/// Commande Tauri : vérifie `text` et renvoie les suggestions.
///
/// `async` afin que Tauri l'exécute sur le pool de threads de l'async runtime
/// et non sur le thread principal : le calcul (potentiellement coûteux sur un
/// gros texte) ne bloque pas l'UI de l'application hôte.
#[tauri::command]
async fn check_text(
    text: String,
    state: tauri::State<'_, Checker>,
) -> Result<Vec<JsSuggestion>, ()> {
    Ok(state
        .check(&text)
        .into_iter()
        .map(|s| JsSuggestion {
            start: s.span.start,
            end: s.span.end,
            message: s.message,
            replacements: s.replacements,
            rule_id: s.rule_id.to_string(),
        })
        .collect())
}

/// Initialise le plugin Hugo.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("hugo-tauri")
        .invoke_handler(tauri::generate_handler![check_text])
        .setup(|app, _api| {
            app.manage(Checker::new());
            Ok(())
        })
        .build()
}
