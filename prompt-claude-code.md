# Prompt Claude Code — Initialisation du projet Hugo

Copie ce prompt intégralement dans Claude Code pour initialiser le projet.

---

## Contexte

Tu vas initialiser **Hugo**, un correcteur orthographique et grammatical français en Rust, conçu pour tourner en local embarqué dans des applications (Tauri, iOS, WASM). Le nom rend hommage à Victor Hugo et fait écho au projet Harper (correcteur anglais en Rust).

Hugo n'utilise pas de LLM ni de réseau. Il combine un tagger morphosyntaxique léger (FST sur le Lefff) avec un moteur de règles grammaticales hand-crafted en Rust pur. L'empreinte cible est ~14 MB sur disque, ~16 MB en RAM, <5 ms par phrase.

## Ce que tu dois faire

### 1. Créer le workspace Cargo

Crée un workspace Cargo avec la structure suivante :

```
hugo/
├── Cargo.toml                  ← workspace members
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .gitignore
├── crates/
│   ├── hugo-core/              ← bibliothèque principale, logique pure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          ← types publics : Checker, Suggestion, Span, re-exports
│   │       ├── tokenizer.rs    ← tokenizer français
│   │       ├── morpho.rs       ← structures Token, Morph, lookup FST (stub)
│   │       ├── rules/
│   │       │   ├── mod.rs      ← trait Rule + engine
│   │       │   ├── agreement.rs   ← stub accord det-nom
│   │       │   ├── conjugation.rs ← stub accord sujet-verbe
│   │       │   └── homophones.rs  ← stub homophones a/à, ce/se
│   │       └── spelling.rs     ← correcteur orthographique (stub)
│   │
│   ├── hugo-ffi/               ← bindings C (cdylib)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          ← extern "C" stubs
│   │
│   ├── hugo-wasm/              ← cible wasm32-unknown-unknown
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          ← #[wasm_bindgen] stubs
│   │
│   └── hugo-tauri/             ← plugin Tauri v2
│       ├── Cargo.toml
│       └── src/lib.rs          ← #[tauri::command] + plugin init
│
└── tools/
    ├── compile-lefff/          ← futur outil de compilation Lefff → FST
    │   ├── Cargo.toml
    │   └── src/main.rs         ← placeholder
    └── compile-dict/           ← futur outil Dicollecte → DAWG
        ├── Cargo.toml
        └── src/main.rs         ← placeholder
```

### 2. Types fondamentaux dans `hugo-core`

Dans `lib.rs`, définis ces types publics :

```rust
/// Position dans le texte source (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Une suggestion de correction
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Position dans le texte source
    pub span: Span,
    /// Message lisible expliquant l'erreur
    pub message: String,
    /// Corrections possibles, triées par pertinence
    pub replacements: Vec<String>,
    /// Identifiant de la règle qui a produit cette suggestion
    pub rule_id: &'static str,
}

/// Point d'entrée principal — vérifie un texte et retourne les suggestions
pub struct Checker {
    // pour l'instant vide, sera rempli avec le FST, DAWG, et les règles
}

impl Checker {
    pub fn new() -> Self {
        Checker {}
    }

    pub fn check(&self, text: &str) -> Vec<Suggestion> {
        let tokens = tokenizer::tokenize(text);
        let mut suggestions = Vec::new();

        // Appliquer les règles grammaticales
        for rule in rules::all_rules() {
            suggestions.extend(rule.check(&tokens));
        }

        // TODO: appliquer le correcteur orthographique

        // Trier par position
        suggestions.sort_by_key(|s| s.span.start);
        suggestions
    }
}
```

### 3. Tokenizer français (`tokenizer.rs`)

Implémente un vrai tokenizer français qui gère :

- La segmentation sur les espaces et la ponctuation
- Les **élisions françaises** : l', d', j', n', m', t', s', c', qu', jusqu', lorsqu', puisqu', quelqu' — le `'` (apostrophe droite) ET `'` (apostrophe typographique U+2019) doivent être gérés
- Les **tirets** dans les inversions (dit-il, peut-être, est-ce)
- La ponctuation qui reste attachée au mot précédent (virgule, point, etc.)

Chaque token doit porter son `Span` (position dans le texte original) pour que les suggestions pointent au bon endroit.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub span: Span,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Punctuation,
    Whitespace,
    Elision,    // l', d', qu'...
    Number,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    // ...
}
```

Le tokenizer doit passer ces tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_sentence() {
        let tokens = tokenize("Le chat dort.");
        let words: Vec<&str> = tokens.iter()
            .filter(|t| t.kind == TokenKind::Word)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(words, vec!["Le", "chat", "dort"]);
    }

    #[test]
    fn test_elision() {
        let tokens = tokenize("L'homme arrive.");
        let words: Vec<&str> = tokens.iter()
            .filter(|t| matches!(t.kind, TokenKind::Word | TokenKind::Elision))
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(words, vec!["L'", "homme", "arrive"]);
    }

    #[test]
    fn test_typographic_apostrophe() {
        let tokens = tokenize("l\u{2019}homme");
        let elision = tokens.iter().find(|t| t.kind == TokenKind::Elision);
        assert!(elision.is_some());
    }

    #[test]
    fn test_inversion() {
        let tokens = tokenize("Peut-être dit-il vrai.");
        let words: Vec<&str> = tokens.iter()
            .filter(|t| t.kind == TokenKind::Word)
            .map(|t| t.text.as_str())
            .collect();
        // Le tokenizer doit préserver les mots composés avec tiret
        // ou les séparer — au choix, mais les spans doivent être corrects
        assert!(words.contains(&"dit") || words.contains(&"dit-il"));
    }

    #[test]
    fn test_spans_are_correct() {
        let input = "Le chat dort.";
        let tokens = tokenize(input);
        for token in &tokens {
            assert_eq!(&input[token.span.start..token.span.end], token.text);
        }
    }

    #[test]
    fn test_numbers() {
        let tokens = tokenize("Il a 42 ans.");
        let num = tokens.iter().find(|t| t.kind == TokenKind::Number);
        assert!(num.is_some());
        assert_eq!(num.unwrap().text, "42");
    }
}
```

### 4. Trait `Rule` et moteur (`rules/mod.rs`)

```rust
use crate::{Suggestion, tokenizer::Token};

pub trait Rule: Send + Sync {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion>;
    fn name(&self) -> &'static str;
    fn id(&self) -> &'static str;
}

/// Retourne toutes les règles actives
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(super::rules::agreement::DeterminerNounAgreement),
        Box::new(super::rules::conjugation::SubjectVerbAgreement),
        Box::new(super::rules::homophones::HomophoneRule),
    ]
}
```

Pour l'instant, les règles dans `agreement.rs`, `conjugation.rs`, et `homophones.rs` sont des stubs qui retournent `vec![]`. L'important est que la structure compile et que le trait soit bien défini.

### 5. Première règle fonctionnelle : doublon de mot

Dans `rules/`, ajoute un fichier `duplicates.rs` qui implémente une vraie règle fonctionnelle — la détection de mots doublés :

- "il il mange" → suggestion de supprimer le doublon
- "le le chat" → suggestion de supprimer le doublon
- Ignorer les mots d'un seul caractère
- Ignorer la casse (« Le le » est aussi un doublon)

Avec tests :

```rust
#[test]
fn test_duplicate_word() {
    let tokens = tokenize("il il mange");
    let rule = DuplicateWord;
    let suggestions = rule.check(&tokens);
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].rule_id, "duplicate_word");
}

#[test]
fn test_no_false_positive() {
    let tokens = tokenize("il mange il dort");
    let rule = DuplicateWord;
    let suggestions = rule.check(&tokens);
    assert_eq!(suggestions.len(), 0);
}
```

### 6. Deuxième règle fonctionnelle : majuscule après point

Dans `rules/`, ajoute `capitalization.rs` :

- "fin. il repart" → "fin. Il repart" (minuscule après point/point d'exclamation/point d'interrogation)
- Ne pas déclencher sur les abréviations courantes (M., Mme., etc.)

Avec tests.

### 7. Plugin Tauri (`hugo-tauri`)

Implémente un vrai plugin Tauri v2 fonctionnel :

```rust
use hugo_core::Checker;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

#[derive(serde::Serialize)]
pub struct JsSuggestion {
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub replacements: Vec<String>,
    pub rule_id: String,
}

#[tauri::command]
fn check_text(text: String, state: tauri::State<'_, Checker>) -> Vec<JsSuggestion> {
    state.check(&text)
        .into_iter()
        .map(|s| JsSuggestion {
            start: s.span.start,
            end: s.span.end,
            message: s.message,
            replacements: s.replacements,
            rule_id: s.rule_id.to_string(),
        })
        .collect()
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("hugo")
        .invoke_handler(tauri::generate_handler![check_text])
        .setup(|app, _api| {
            app.manage(Checker::new());
            Ok(())
        })
        .build()
}
```

Dépendances dans `Cargo.toml` :
- `tauri = { version = "2", features = ["build"] }`
- `serde = { version = "1", features = ["derive"] }`
- `hugo-core = { path = "../hugo-core" }`

### 8. Stubs FFI et WASM

`hugo-ffi/src/lib.rs` : expose au minimum `hugo_checker_new()`, `hugo_checker_check()`, `hugo_free_results()` en `extern "C"` — stubs qui compilent mais ne font rien de complexe.

`hugo-wasm/src/lib.rs` : expose un `#[wasm_bindgen]` `check(text: &str) -> JsValue` — stub compilable.

### 9. Configuration CI minimale

Ajoute `.github/workflows/ci.yml` :

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
```

### 10. README.md

Écris un `README.md` avec :

- Le nom et le tagline ("Hugo — Correcteur grammatical français, rapide et embarquable")
- Badge CI
- Description courte (3-4 lignes)
- Section "Statut" indiquant que c'est un travail en cours
- Section "Architecture" avec la structure des crates
- Section "Utilisation" avec un exemple Rust minimal
- Section "Licence" (dual MIT / Apache 2.0)

## Contraintes

- Tout doit compiler avec `cargo build --workspace` (sauf hugo-wasm qui nécessite la target wasm32)
- Tous les tests doivent passer avec `cargo test --workspace`
- Pas de `unwrap()` dans le code de bibliothèque (sauf les tests)
- Utilise `thiserror` pour les types d'erreur si nécessaire
- Documentation Rust (/// commentaires) sur tous les types et fonctions publics
- Édition Rust 2021
- Le tokenizer doit être robuste : pas de panic sur des entrées arbitraires (chaînes vides, Unicode exotique, émojis)
