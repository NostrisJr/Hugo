fn main() {
    let checker = hugo_core::Checker::new();
    for text in &[
        "Les livres que j'ai lu",
        "Les lettres qu'il a écrit",
        "Les décisions qu'on a prit",
        "La lettre a été écris",
    ] {
        let sug = checker.check(text);
        println!("{text:?} → {:?}", sug.iter().map(|s| (&s.rule_id, &s.replacements)).collect::<Vec<_>>());
    }
}
