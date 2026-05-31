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
- ✅ **Correcteur orthographique** : 449 k formes Dicollecte compilées en FST (581 Ko)
  **embarqué** dans la bibliothèque ; détection des mots inconnus et suggestions
  par automate de Levenshtein reclassées en Damerau-Levenshtein
- ✅ Plugin Tauri v2 (`check_text`), bindings C FFI et WASM fonctionnels

À venir :

- ⏳ Compilation du Lefff en FST (morphologie) — `tools/compile-lefff`
- ⏳ Règles d'accord (déterminant–nom, sujet–verbe) et homophones (phase 2)
- ⏳ Fréquences lexicales pour le tri des suggestions
- ⏳ CRF de désambiguïsation POS (phase 4)

Démo : `cargo run --example check -- "les maison son belle. il dort dans dans."`

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
    ├── compile-lefff/ Lefff (TSV) → lefff.fst (à venir)
    └── compile-dict/  Dicollecte (.dic/.aff Hunspell) → dicollecte.fst
```

Le pipeline de `hugo-core` : `texte → tokenizer → (morpho) → règles + orthographe → suggestions`.

### Régénérer le dictionnaire orthographique

Le FST embarqué (`crates/hugo-core/assets/dicollecte.fst`) est dérivé du
dictionnaire Hunspell français de Dicollecte (MPL 2.0). Pour le reconstruire :

```sh
# 1. Récupérer le dictionnaire source (MPL 2.0)
curl -sSL -o fr.aff https://raw.githubusercontent.com/LibreOffice/dictionaries/master/fr_FR/fr.aff
curl -sSL -o fr.dic https://raw.githubusercontent.com/LibreOffice/dictionaries/master/fr_FR/fr.dic

# 2. Développer les affixes et compiler le FST
cargo run -p compile-dict --release -- fr.dic fr.aff crates/hugo-core/assets/dicollecte.fst
```

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

Le dictionnaire orthographique embarqué est dérivé de Dicollecte (MPL 2.0) ;
voir [`NOTICE.md`](NOTICE.md) pour l'attribution.
