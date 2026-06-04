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
  par automate de Levenshtein, reclassées par distance de Damerau-Levenshtein
  **et fréquence lexicale** (Lexique383, corpus livres)
- ✅ **Morphologie** : 125 k formes (catégorie, genre, nombre, lemme) issues de
  Lexique383, compilées en FST + blob (~2,5 Mo) embarqués
- ✅ **Accord déterminant–nom** (genre et nombre) : « un belle table » → « une »,
  « les chat » → « le », « le chats » → « les »
- ✅ **Accord sujet–verbe** (sujets pronoms) : « ils mange » → « mangent »,
  « tu mange » → « manges » ; correction engendrée par conjugaison du lemme
- ✅ **Désambiguïsation POS (CRF)** : étiqueteur morphosyntaxique (CRF à chaîne
  linéaire entraîné sur UD French-GSD, **~97,8 % d'exactitude**, modèle **2,6 Mo**
  embarqué) ; tranche les homographes (« elle **ferme** la **porte** » →
  VERBE / NOM) et distingue auxiliaire et verbe plein (« **sont** partis » → AUX
  + VERBE). Branché dans le pipeline ; étiquettes offertes aux règles
  (`Rule::check_tagged`).
- ✅ Plugin Tauri v2 (`check_text`), bindings C FFI et WASM fonctionnels

Règles **adossées au CRF** (consomment les étiquettes POS via `Rule::check_tagged`) :

- ✅ **se → ce** : « se petit chat » → « ce » — un « se » réflexif est toujours
  préverbal (`rules::homophones`).
- ✅ **son ↔ sont** par **étiquetage contrefactuel** : on compare le score POS des
  deux graphies (« mes parents son venus » → « sont », « il caresse sont chat » →
  « son »). *ou/où* reste hors de portée (pas de signal séparable).
- ✅ **Participe passé avec « avoir » + COD antéposé** (`rules::past_participle`) :
  « je les ai vu » → « vus »/« vues », « il la a vu » → « vue ».
- ✅ **Participe passé pronominal** (`rules::pronominal_participle`) : « elle s'est
  levé » → « levée » ; garde du COD postposé (« elle s'est lavé les mains »).
- ✅ **Subjonctif après conjonction** (`rules::subjunctive`) : « bien qu'il est » →
  « soit », « pour que tu viens » → « viennes » (sujets pronominaux **et** nominaux).
- ✅ **Accords spéciaux** (`rules::special_agreement`) : couleurs-noms invariables
  (« des gants marrons » → « marron »), même (« les même livres » → « mêmes »),
  quelque (« quelque livres » → « quelques »).
- ✅ **Robustesse POS des accords** : déterminant–nom et épithète identifient la
  tête nominale par étiquetage (plus de faux positifs « je les ferme », « est »).

À venir :

- ⏳ ou/où (pas de signal séparable) ; concordance des temps au subjonctif ;
  couleurs composées (« bleu ciel ») ; nombres composés ; port des règles
  LanguageTool

Démo : `cargo run --example check -- "Ils mange un belle table. Tu mange une pome."`

Voir [`ROADMAP.md`](ROADMAP.md) pour le détail des phases.

## Architecture

Workspace Cargo organisé en crates et outils :

```
hugo/
├── crates/
│   ├── hugo-core/     Bibliothèque centrale : tokenizer, morpho, POS (CRF), règles, correcteur
│   ├── hugo-ffi/      Bindings C (staticlib/cdylib) — iOS, macOS, Android
│   ├── hugo-wasm/     Bindings WebAssembly — JS/TS (paquet npm hugo-wasm)
│   └── hugo-tauri/    Plugin Tauri v2 (commande check_text)
└── tools/
    ├── compile-morpho/ Lexique383 (TSV) → morpho.fst + morpho.bin
    ├── compile-dict/   Dicollecte (.dic/.aff Hunspell) → dicollecte.fst
    └── train-crf/      UD French-GSD (CoNLL-U) → pos.crf (modèle POS)
```

Le pipeline de `hugo-core` : `texte → tokenizer → morpho → POS (CRF) → règles + orthographe → suggestions`.

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

### Régénérer le lexique morphologique

Les assets `morpho.fst`/`morpho.bin` sont dérivés de Lexique383 (CC BY-SA 4.0) :

```sh
curl -sSL -o Lexique383.tsv http://www.lexique.org/databases/Lexique383/Lexique383.tsv
cargo run -p compile-morpho --release -- Lexique383.tsv crates/hugo-core/assets/morpho
```

### Régénérer le modèle POS (CRF)

L'asset embarqué `crates/hugo-core/assets/pos.crf` est un CRF d'étiquetage
morphosyntaxique entraîné sur **Universal Dependencies French-GSD** (CC BY-SA 4.0).
Pour le reconstruire (voir [`tools/train-crf`](tools/train-crf/)) :

```sh
# 1. Récupérer les corpus CoNLL-U (train / dev / test)
base=https://raw.githubusercontent.com/UniversalDependencies/UD_French-GSD/master
for split in train dev test; do
  curl -sSL -o fr-$split.conllu $base/fr_gsd-ud-$split.conllu
done

# 2. Entraîner et écrire l'asset embarqué (affiche l'exactitude dev/test)
cargo run -p train-crf --release -- fr-train.conllu fr-dev.conllu fr-test.conllu \
  crates/hugo-core/assets/pos.crf
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
const suggestions = await invoke("plugin:hugo-tauri|check_text", { text });
```

> N'oubliez pas d'accorder la permission `hugo-tauri:allow-check-text` dans votre
> capability. **Guide complet** : [`docs/tauri-integration.md`](docs/tauri-integration.md)
> (dépendance, ACL, types TypeScript, dépannage).

### Intégration native (C, C++, Swift)

```bash
cargo build -p hugo-ffi --release   # → libhugo_ffi.a / .dylib + include/hugo.h
```

```c
HugoChecker *c = hugo_checker_new();
HugoResults  r = hugo_checker_check(c, "il va a Paris");
// ... lire r.suggestions[0..r.len] ...
hugo_free_results(r);
hugo_checker_free(c);
```

> **Guide complet** : [`docs/c-ffi-integration.md`](docs/c-ffi-integration.md)
> (API C, wrapper Swift `Hugo.swift`, XCFramework, JNI, dépannage).

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
| Lexique383         | CC BY-SA 4.0  | Morphologie (genre, nombre…)  |
| Dicollecte fr_FR   | MPL 2.0       | Orthographe                   |
| UD French-GSD      | CC BY-SA 4.0  | Entraînement CRF (phase 4)    |
| LanguageTool fr    | LGPL 2.1      | Inspiration (réécriture Rust) |

Voir [`NOTICE.md`](NOTICE.md) pour les attributions complètes des données embarquées.

## Licence

Distribué sous double licence, au choix :

- MIT ([`LICENSE-MIT`](LICENSE-MIT))
- Apache 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))

Sauf mention contraire, toute contribution soumise pour inclusion est réputée
doublement licenciée comme ci-dessus, sans condition supplémentaire.

Le dictionnaire orthographique embarqué est dérivé de Dicollecte (MPL 2.0) ;
voir [`NOTICE.md`](NOTICE.md) pour l'attribution.
