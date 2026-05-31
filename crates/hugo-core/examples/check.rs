//! Démo en ligne de commande du correcteur Hugo.
//!
//! ```sh
//! cargo run --example check -- "Le chat mange. il dort. les maison son belle."
//! ```
//! Sans argument, une phrase de démonstration est utilisée.

use hugo_core::Checker;

fn main() {
    let text = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let text = if text.trim().is_empty() {
        "Le chat mange. il dort dans dans la maisn avec une belle peluche.".to_string()
    } else {
        text
    };

    let checker = Checker::new();
    let suggestions = checker.check(&text);

    println!("Texte : {text}\n");
    if suggestions.is_empty() {
        println!("Aucune suggestion.");
        return;
    }

    for s in &suggestions {
        let fragment = &text[s.span.start..s.span.end];
        let reps = if s.replacements.is_empty() {
            "—".to_string()
        } else {
            s.replacements
                .iter()
                .map(|r| if r.is_empty() { "(supprimer)" } else { r })
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "[{:>3}..{:<3}] «{}»  {}\n            → {}  ({})",
            s.span.start, s.span.end, fragment, s.message, reps, s.rule_id
        );
    }
}
