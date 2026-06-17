//! Génère les métadonnées et permissions du plugin Tauri.

const COMMANDS: &[&str] = &["check_text", "list_rules"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
