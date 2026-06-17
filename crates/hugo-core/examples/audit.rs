//! Audit des **faux positifs** sur un corpus de texte **correct**.
//!
//! Lit un fichier CoNLL-U (lignes `# text = …`) ou un fichier texte (une phrase
//! par ligne), passe chaque phrase au [`Checker`] et agrège les suggestions par
//! règle. Le corpus étant correct, **toute** suggestion est un faux positif
//! candidat.
//!
//! ```sh
//! cargo run --release -p hugo-core --example audit -- /tmp/fr-test.conllu
//! cargo run --release -p hugo-core --example audit -- /tmp/fr-test.conllu --rule confusion_terminaison
//! ```

use std::collections::BTreeMap;

use hugo_core::Checker;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: audit <fichier> [--rule <id>] [--grammar]");
    let mut only_rule: Option<String> = None;
    let mut grammar_only = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--rule" => only_rule = args.next(),
            "--grammar" => grammar_only = true,
            _ => {}
        }
    }

    // Règles d'orthographe/capitalisation : bruit attendu sur du texte brut
    // (noms propres, mots étrangers). `--grammar` les masque.
    let non_grammar = ["spelling", "capitalization_after_period"];

    let raw = std::fs::read_to_string(&path).expect("lecture du corpus");
    let sentences: Vec<String> = if path.ends_with(".conllu") {
        raw.lines()
            .filter_map(|l| l.strip_prefix("# text = ").map(str::to_string))
            .collect()
    } else {
        raw.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    };

    let checker = Checker::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let mut flagged_sentences = 0usize;
    let mut examples: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for text in &sentences {
        let mut hit = false;
        for s in checker.check(text) {
            if grammar_only && non_grammar.contains(&s.rule_id) {
                continue;
            }
            if let Some(r) = &only_rule {
                if s.rule_id != *r {
                    continue;
                }
            }
            hit = true;
            *by_rule.entry(s.rule_id.to_string()).or_default() += 1;
            let frag = &text[s.span.start..s.span.end];
            let rep = s.replacements.first().map(String::as_str).unwrap_or("∅");
            let bucket = examples.entry(s.rule_id.to_string()).or_default();
            if bucket.len() < 25 {
                bucket.push(format!("  «{frag}» → {rep}   | {text}"));
            }
        }
        if hit {
            flagged_sentences += 1;
        }
    }

    let total: usize = by_rule.values().sum();
    println!("Corpus : {} phrases", sentences.len());
    println!(
        "Phrases signalées : {flagged_sentences} ({:.1} %)",
        100.0 * flagged_sentences as f64 / sentences.len().max(1) as f64
    );
    println!("Suggestions totales : {total}\n");
    println!("Par règle :");
    let mut rows: Vec<(&String, &usize)> = by_rule.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1));
    for (rule, n) in rows {
        println!("  {n:>4}  {rule}");
    }

    if let Some(r) = &only_rule {
        println!("\nExemples ({r}) :");
        if let Some(ex) = examples.get(r) {
            for e in ex {
                println!("{e}");
            }
        }
    }
}
