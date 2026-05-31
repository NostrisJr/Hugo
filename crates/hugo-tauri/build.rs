//! Génère les métadonnées et permissions du plugin Tauri.

const COMMANDS: &[&str] = &["check_text"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
