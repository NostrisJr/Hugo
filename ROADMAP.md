# Hugo — Feuille de route

> Correcteur orthographique et grammatical français en Rust, embarquable en local.  
> Nommé en hommage à Victor Hugo et en écho au projet Harper.

---

## Objectif final

Livrer une bibliothèque Rust autonome capable de :

- **Corriger l'orthographe** française via un DAWG compilé depuis Dicollecte (~400 000 formes)
- **Corriger la grammaire** via un moteur de règles opérant sur des tokens annotés morphologiquement
- **Tourner en local** sans réseau, sans LLM, avec ~14 MB sur disque et <5 ms par phrase
- **S'intégrer** dans n'importe quelle app via C FFI (Swift/Kotlin), WASM (JS/TS), plugin Tauri v2, ou PyO3 (Python)

---

## Contraintes techniques

| Paramètre | Cible |
|---|---|
| Langage | Rust, édition 2021 |
| Empreinte disque | ~14 MB (FST + DAWG + CRF + binaire) |
| Empreinte RAM | ~16 MB (mmap possible) |
| Latence | <5 ms par phrase sur CPU mobile |
| Données morpho | Lefff (~110k entrées, LGPL-LR) |
| Données ortho | Dicollecte fr_FR (~400k formes, MPL 2.0) |
| Données POS | Universal Dependencies French-GSD (CC BY-SA 4.0) |
| Licence projet | MIT / Apache-2.0 dual |

---

## Phases

### Phase 1 — Fondations (semaines 1–2)

**Objectif** : correcteur orthographique pur fonctionnel + structure du projet.

- [ ] Workspace Cargo + CI GitHub Actions
- [ ] `README.md` avec description, architecture, licence
- [ ] **Tokenizer français**
  - [ ] Segmentation espaces + ponctuation
  - [ ] Élisions (l', d', j', qu'… — apostrophe droite + typographique U+2019)
  - [ ] Tirets dans inversions (dit-il, peut-être)
  - [ ] Spans corrects (byte offsets dans le texte source)
  - [ ] Tests : phrases simples, élisions, nombres, Unicode, chaînes vides
- [ ] **Types publics** : `Span`, `Suggestion`, `Checker`, `Token`, `TokenKind`
- [ ] **Trait `Rule`** + moteur d'agrégation des suggestions
- [ ] **Première règle** : doublon de mot (il il → il)
- [ ] **Deuxième règle** : majuscule après ponctuation terminale
- [ ] **Plugin Tauri v2** : structure + commande `check_text` fonctionnelle
- [ ] **Stubs FFI et WASM** : compilent sans logique
- [ ] Compiler le Lefff en FST avec la crate `fst`
  - [ ] Outil `tools/compile-lefff` : parsing du TSV Lefff
  - [ ] Génération de `lefff.fst` (~8 MB)
  - [ ] Lookup morphologique : `Token` → `Vec<Morph>` (catégorie, genre, nombre, lemme)
- [ ] Compiler Dicollecte en DAWG
  - [ ] Outil `tools/compile-dict` : parsing .dic/.aff Hunspell
  - [ ] Génération de `dicollecte.dawg` (~3 MB)
  - [ ] Suggestions par distance de Damerau-Levenshtein
  - [ ] Tri des suggestions par fréquence lexicale
- [ ] Tests d'intégration orthographe (corpus de 500 mots de référence, cible >95%)

---

### Phase 2 — Grammaire de base (semaines 3–5)

**Objectif** : 5 règles d'accord fondamentales fonctionnelles grâce à la morphologie du Lefff.

- [x] **Moteur de règles** : fenêtre glissante par phrase (`lexical_sentences`) sur tokens annotés
- [x] **Accord déterminant–nom** (`rules::agreement`)
  - [x] Genre : "un belle table" → "une belle table"
  - [x] Nombre : "les chat" → "les chats"
  - [x] Déterminants contractés : "du table" → "de la", "aux chat" → "au"
- [x] **Accord sujet–verbe** (`rules::conjugation`)
  - [x] Personne + nombre, sujets pronominaux : "ils mange" → "ils mangent"
  - [x] Sujets nominaux : "les chats mange" → "les chats mangent"
  - [x] Gestion des inversions : "mange-t-il" ne déclenche pas
  - [x] Garde anti complément d'objet / groupe prépositionnel
  - [x] Sujets coordonnés : "Pierre et Marie mange" → "mangent" (personne = priorité 1>2>3 : "toi et moi" → nous)
- [x] **Accord adjectif attribut** (`rules::attribute`)
  - [x] "elle est content" → "elle est contente" (via `morpho::decline`)
  - [x] Copules être, sembler, paraître, devenir, rester, demeurer
  - [x] Sujets pronominaux (il/elle/ils/elles) et nominaux à genre connu
- [x] **Homophones grammaticaux — fréquents** (`rules::homophones`, heuristiques haute précision)
  - [x] a/à : "il va a Paris" → "il va à Paris" (et "il à faim" → "a")
  - [ ] ou/où : nécessite la désambiguïsation POS (phase 4)
  - [x] ce/se : "il ce lève" → "il se lève" (sens ce→se uniquement)
  - [x] mes/mais : "mes je ne sais pas" → "mais je ne sais pas"
  - [x] son/sont : "ils son partis" → "ils sont partis"
  - [x] on/ont : "ils on mangé" → "ils ont mangé"
- [x] **Suite de tests grammaticaux** : tests unitaires par règle + corpus
  annoté d'intégration (`tests/grammar.rs`, cas correct / incorrect / correction).
- [x] Benchmark de performance : contrôle dans `tests/grammar.rs`
  (`performance_is_within_budget`), bien en deçà de 5 ms par phrase de ~20 mots.

> **État (phase 2 — terminée)** : les cinq familles de règles
> d'accord/homophonie sont fonctionnelles et testées (101 tests unitaires + 3
> tests d'intégration, `cargo clippy` propre). Seuls restent volontairement
> hors phase 2 les homophones qui exigent un contexte POS riche (ou/où, sens
> se→ce, sont→son), repoussés à la **phase 4** avec le CRF.

---

### Phase 3 — Intégrations (semaines 6–8)

**Objectif** : livrable utilisable par d'autres applications.

- [~] **Plugin Tauri v2 complet**
  - [x] Commande `check_text` avec sérialisation `JsSuggestion`
  - [x] `Checker` initialisé une seule fois via `app.manage()`
  - [x] Guide d'intégration ([`docs/tauri-integration.md`](docs/tauri-integration.md), avec types TS fournis)
  - [ ] Types TypeScript **générés** (`hugo.d.ts`) automatiquement
  - [ ] Exemple d'intégration dans une app Tauri + React
- [x] **C FFI**
  - [x] `hugo_checker_new()`, `hugo_checker_check()`, `hugo_free_results()`
  - [x] Header C ([`crates/hugo-ffi/include/hugo.h`](crates/hugo-ffi/include/hugo.h)) + `cbindgen.toml` de régénération
  - [x] Build en `staticlib`/`cdylib` (vérifié par un test C linké, voir `examples/c_demo.c`)
  - [x] Package XCFramework pour Swift ([`scripts/build-xcframework.sh`](scripts/build-xcframework.sh))
  - [x] Wrapper Swift ([`crates/hugo-ffi/swift/Hugo.swift`](crates/hugo-ffi/swift/Hugo.swift)) avec API idiomatique
- [ ] **WASM**
  - [x] Bindings `wasm-bindgen` fonctionnels (`HugoChecker`, `check`)
  - [ ] Build avec `wasm-pack` + package npm `hugo-wasm` (types TypeScript)
  - [ ] Exemple d'utilisation dans une app web
- [~] **Documentation**
  - [x] rustdoc sur les types et fonctions publics
  - [x] Guides d'intégration ([Tauri](docs/tauri-integration.md), [C FFI](docs/c-ffi-integration.md))
  - [x] Exemples : `crates/hugo-core/examples/check.rs`, `crates/hugo-ffi/examples/c_demo.c`
  - [ ] Exemple WASM web

---

### Phase 4 — Grammaire avancée (mois 3+)

**Objectif** : CRF pour la désambiguïsation POS + règles nécessitant un contexte syntaxique précis.

- [ ] **CRF (Conditional Random Field)** — désambiguïsation POS
  - [ ] Entraînement sur Universal Dependencies French-GSD
  - [ ] Modèle compact (~2 MB)
  - [ ] Intégration dans le pipeline entre morpho lookup et moteur de règles
  - [ ] Précision POS cible : >97%

#### Plan d'implémentation du CRF

**Pourquoi.** Aujourd'hui chaque règle lève l'ambiguïté à la main (gardes sur le
voisinage, lemme unique, listes fermées). Cela plafonne la couverture : « son »
n'est corrigé qu'après `ils/elles`, « ou/où » et « se/ce » sont hors de portée,
et l'accord nominal s'arrête au premier homographe. Un **étiqueteur POS** qui
assigne à chaque token **une** catégorie (sa probabilité maximale dans le
contexte) remplacerait ces heuristiques par une désambiguïsation globale.

**Architecture cible** (un nouveau crate-outil + un module runtime) :

1. `tools/train-crf` (hors livrable, dépendances libres d'entraînement)
   - Lecture du CoNLL-U de UD French-GSD (train/dev/test).
   - Mapping des UPOS UD → notre `MorphCategory`.
   - **Features par token** (gabarit classique d'étiquetage) : forme brute,
     minuscule, préfixes/suffixes 1–4, casse/chiffres/ponctuation, et surtout le
     **sac de catégories possibles issu de `morpho::lookup`** (le CRF tranche
     entre les analyses que le lexique propose déjà) ; mêmes features décalées à
     ±1, ±2 ; features de transition (étiquette précédente).
   - Entraînement L-BFGS + régularisation L2 ; viser >97 % sur le dev.
   - Sérialisation des poids vers un format binaire compact (`assets/pos.crf`,
     ~2 MB), quantifié si nécessaire.

2. `crates/hugo-core/src/pos.rs` (runtime, embarqué)
   - Décodage **Viterbi** linéaire sur la séquence de tokens d'une phrase.
   - API : `pub fn tag(tokens: &[Token]) -> Vec<MorphCategory>` (ou un
     `Vec<Tagged>` portant catégorie + traits désambiguïsés).
   - Chargement paresseux du modèle via `OnceLock`, comme `morpho::instance()`.

3. **Intégration pipeline** (`Checker::check`)
   - Tokenize → `morpho::lookup` (analyses candidates) → **`pos::tag`**
     (désambiguïsation) → règles, qui consomment alors une catégorie unique au
     lieu d'inspecter `Vec<Morph>`.
   - Étape progressive : exposer un `&[Tagged]` aux règles sans casser l'API
     `Rule::check(&[Token])` (passer un contexte enrichi, ou stocker les tags
     dans une structure parallèle indexée par position).

**Critères budget.** Décodage Viterbi O(n · |états|²) ; avec ~12 catégories et
des phrases courtes, reste très en dessous du budget <5 ms. Vérifier l'empreinte
du modèle embarqué (cible ~2 MB) et le temps de chargement initial.

**Débloque ensuite.** ou/où, se/ce (sens manquant), sont/son, accord nominal
au-delà du premier homographe, et l'accord du participe passé (qui suppose de
distinguer auxiliaire vs. verbe plein).
- [ ] **Accord du participe passé**
  - [ ] Avec être : "elle est parti" → "elle est partie"
  - [ ] Avec avoir + COD antéposé : "je les ai vu" → "je les ai vus"
  - [ ] Verbes pronominaux : "elle s'est levé" → "elle s'est levée"
- [ ] **Accord de l'adjectif épithète**
  - [ ] "une grande maisons" → "de grandes maisons"
  - [ ] Adjectifs antéposés et postposés
- [ ] **Subjonctif**
  - [ ] Après bien que, pour que, afin que, quoique…
  - [ ] "bien qu'il est" → "bien qu'il soit"
- [ ] **Accords spéciaux**
  - [ ] Adjectifs de couleur composés (invariables) : "des robes bleu ciel"
  - [ ] tout/même/quelque : "toute les jours" → "tous les jours"
  - [ ] Trait d'union dans les nombres composés (post-réforme 1990)
- [ ] **Port des règles LanguageTool**
  - [ ] Inventaire des règles françaises XML de LanguageTool
  - [ ] Réécriture en Rust des règles les plus impactantes
  - [ ] Benchmark qualité vs LanguageTool en mode local

---

### Phase 5 — Qualité et polish (mois 4+)

**Objectif** : qualité production.

- [ ] Benchmark comparatif avec LanguageTool (précision, rappel, F1)
- [ ] Faux positifs : audit sur corpus de textes corrects (littérature, presse, code)
- [ ] Gestion des noms propres et acronymes (ne pas corriger)
- [ ] Dictionnaire utilisateur (mots personnalisés)
- [ ] Gestion de l'ancienne vs. nouvelle orthographe (1990)
- [ ] Performances : profiling, optimisation des chemins chauds
- [ ] Publication sur crates.io (`hugo-core`, `hugo-tauri`)
- [ ] Publication npm (`hugo-wasm`)
- [ ] Site web avec démo interactive

---

## Données et licences

| Ressource | Licence | Usage |
|---|---|---|
| Lefff | LGPL-LR | Morphologie — libre commercial |
| Dicollecte fr_FR | MPL 2.0 | Orthographe — libre commercial |
| UD French-GSD | CC BY-SA 4.0 | Entraînement CRF (phase 4) |
| LanguageTool rules/fr | LGPL 2.1 | Inspiration — réécriture intégrale en Rust |
| Crate `fst` | MIT / Unlicense | FST et DAWG |
| Crate `wasm-bindgen` | MIT / Apache-2.0 | Bindings WASM |
| Tauri v2 | MIT / Apache-2.0 | Plugin natif |

---

## Notes

- Le nom `hugo` sur crates.io est à vérifier (peut-être pris par le générateur de sites statique Hugo en Go — dans ce cas, `hugo-fr` ou `hugo-spell`).
- Licence du projet : MIT / Apache-2.0 dual (standard Rust).
- Le CRF (phase 4) est la pièce la plus complexe. Les phases 1–3 fonctionnent avec la morphologie lexicale seule + heuristiques de contexte.
