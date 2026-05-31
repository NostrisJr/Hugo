//! Tests d'intégration du correcteur orthographique sur le dictionnaire réel.

use hugo_core::{Checker, SpellChecker, SPELLING_RULE_ID};

/// Échantillon de mots français courants : aucun ne doit être signalé.
const CORRECT_WORDS: &[&str] = &[
    "bonjour",
    "maison",
    "maisons",
    "chat",
    "chats",
    "chien",
    "manger",
    "mange",
    "mangent",
    "rapidement",
    "ordinateur",
    "téléphone",
    "français",
    "française",
    "beaucoup",
    "toujours",
    "pourquoi",
    "intelligence",
    "développement",
    "extraordinaire",
    "magnifique",
    "évidemment",
    "naturellement",
    "gouvernement",
    "université",
];

#[test]
fn no_false_positives_on_common_words() {
    let sc = SpellChecker::new();
    let unknown: Vec<&str> = CORRECT_WORDS
        .iter()
        .copied()
        .filter(|w| !sc.contains(w))
        .collect();
    assert!(
        unknown.is_empty(),
        "mots corrects rejetés à tort : {unknown:?}"
    );
}

#[test]
fn correct_sentence_yields_no_spelling_suggestion() {
    let checker = Checker::new();
    let text = "Le chat noir mange une souris dans la grande maison blanche.";
    let spelling: Vec<_> = checker
        .check(text)
        .into_iter()
        .filter(|s| s.rule_id == SPELLING_RULE_ID)
        .collect();
    assert!(
        spelling.is_empty(),
        "faux positifs orthographiques : {spelling:?}"
    );
}

#[test]
fn typical_typos_get_right_top_suggestion() {
    let sc = SpellChecker::new();
    // (faute, correction attendue dans le top 3)
    let cases = [
        ("bonjor", "bonjour"),
        ("téléphne", "téléphone"),
        ("dévloppement", "développement"),
        ("ordinateru", "ordinateur"),
    ];
    for (typo, expected) in cases {
        let sugg = sc.suggest(typo, 3);
        assert!(
            sugg.iter().any(|s| s == expected),
            "« {typo} » → {sugg:?}, attendait « {expected} » dans le top 3"
        );
    }
}

#[test]
fn unknown_word_is_flagged_with_suggestions() {
    let checker = Checker::new();
    let suggestions = checker.check("Voici une maisn.");
    let spell = suggestions
        .iter()
        .find(|s| s.rule_id == SPELLING_RULE_ID)
        .expect("le mot inconnu devrait être signalé");
    assert!(
        spell.replacements.iter().any(|r| r == "maison"),
        "suggestions = {:?}",
        spell.replacements
    );
}
