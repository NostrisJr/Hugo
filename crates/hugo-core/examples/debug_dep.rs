//! Affiche l'arbre de dépendances d'une phrase (token, POS, head, relation).
//! Usage : `cargo run -p hugo-core --example debug_dep -- "ma phrase"`.

use hugo_core::dep;
use hugo_core::pos;
use hugo_core::tokenizer::{tokenize, TokenKind};

fn show(phrase: &str) {
    let tokens = tokenize(phrase);
    let mut tags = pos::tag(&tokens);
    dep::parse(&tokens, &mut tags);
    println!("\n=== {phrase} ===");
    for (i, t) in tokens.iter().enumerate() {
        if t.kind == TokenKind::Whitespace {
            continue;
        }
        let head = tags[i].head as usize;
        let head_txt = if dep::is_root(&tags, i) {
            "ROOT".to_string()
        } else {
            format!("{} ({})", tokens[head].text, head)
        };
        println!(
            "  {:>2} {:<14} {:<6} --{}--> {}",
            i,
            t.text,
            tags[i].upos.as_str(),
            tags[i].dep.as_str(),
            head_txt
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        show(&args.join(" "));
        return;
    }
    show("Les applications tierces, processus métiers, contraintes légales.");
    show("Compréhension du problème et état de l'art.");
    show("Deux solutions visant à automatiser la migration ont été citées.");
    show("La nuit était totalement tombée, et le jeune homme avançait toujours.");
    show("Le chat que j'ai vu dormait.");
}
