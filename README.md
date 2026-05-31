# Hugo — Correcteur grammatical français, rapide et embarquable

[![CI](https://github.com/theophiledonato/hugo/actions/workflows/ci.yml/badge.svg)](https://github.com/theophiledonato/hugo/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20%2F%20Apache--2.0-blue.svg)](#licence)

**Hugo** est un correcteur orthographique et grammatical français écrit en Rust,
conçu pour tourner **en local**, sans réseau ni LLM. Il combine un tagger
morphosyntaxique léger (FST sur le Lefff) avec un moteur de règles grammaticales
écrites à la main. Empreinte cible : ~14 MB sur disque, ~16 MB en RAM, <5 ms par
phrase. Nommé en hommage à Victor Hugo, en écho au projet [Harper](https://writewithharper.com/).

## Statut

🚧 **Travail en cours — Phase 1 (fondations).**

Fonctionnel aujourd'hui :

- ✅ Tokenizer français robuste (élisions, traits d'union, nombres, ponctuation, Unicode)
- ✅ Moteur de règles (`trait Rule` + agrégation)
- ✅ Règle « mot doublé » (`il il` → `il`)
- ✅ Règle « majuscule après ponctuation terminale »
- ✅ Plugin Tauri v2 (`check_text`), stubs C FFI et WASM compilables

À venir :

- ⏳ Compilation du Lefff en FST et de Dicollecte en DAWG (`tools/`)
- ⏳ Correcteur orthographique (Damerau-Levenshtein)
- ⏳ Règles d'accord (déterminant–nom, sujet–verbe) et homophones (phase 2)
- ⏳ CRF de désambiguïsation POS (phase 4)

Voir [`ROADMAP.md`](ROADMAP.md) pour le détail des phases.

## Architecture

Workspace Cargo organisé en crates et outils :

```
hugo/
├── crates/
│   ├── hugo-core/     Bibliothèque centrale : tokenizer, morpho, règles, correcteur
│   ├── hugo-ffi/      Bindings C (staticlib/cdylib) — iOS, macOS, Android
│   ├── hugo-wasm/     Bindings WebAssembly — JS/TS (paquet npm hugo-wasm)
│   └── hugo-tauri/    Plugin Tauri v2 (commande check_text)
└── tools/
    ├── compile-lefff/ Lefff (TSV) → lefff.fst
    └── compile-dict/  Dicollecte (.dic/.aff) → dicollecte.dawg
```

Le pipeline de `hugo-core` : `texte → tokenizer → (morpho) → règles → suggestions`.

## Utilisation

Ajouter `hugo-core` à un projet Rust :

```toml
[dependencies]
hugo-core = { git = "https://github.com/theophiledonato/hugo" }
```

```rust
use hugo_core::Checker;

let checker = Checker::new();
for suggestion in checker.check("il il va a Paris. il rentre") {
    println!(
        "[{}..{}] {} → {:?}",
        suggestion.span.start,
        suggestion.span.end,
        suggestion.message,
        suggestion.replacements,
    );
}
```

### Intégration Tauri v2

```rust
tauri::Builder::default()
    .plugin(hugo_tauri::init())
    // ...
```

```js
import { invoke } from "@tauri-apps/api/core";
const suggestions = await invoke("plugin:hugo|check_text", { text });
```

## Développement

```bash
cargo test --workspace        # tests
cargo clippy --workspace -- -D warnings
cargo fmt --all               # formatage
cargo build -p hugo-wasm --target wasm32-unknown-unknown   # cible web
```

## Données et licences

| Ressource          | Licence       | Usage                         |
|--------------------|---------------|-------------------------------|
| Lefff              | LGPL-LR       | Morphologie                   |
| Dicollecte fr_FR   | MPL 2.0       | Orthographe                   |
| UD French-GSD      | CC BY-SA 4.0  | Entraînement CRF (phase 4)    |
| LanguageTool fr    | LGPL 2.1      | Inspiration (réécriture Rust) |

## Licence

Distribué sous double licence, au choix :

- MIT ([`LICENSE-MIT`](LICENSE-MIT))
- Apache 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

Sauf mention contraire, toute contribution soumise pour inclusion est réputée
doublement licenciée comme ci-dessus, sans condition supplémentaire.
# Hugo
