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

- [ ] **Moteur de règles** : fenêtre glissante sur tokens annotés
- [ ] **Accord déterminant–nom**
  - [ ] Genre : "un belle maison" → "une belle maison"
  - [ ] Nombre : "les chat" → "les chats"
  - [ ] Gestion des déterminants composés (de la, du, des)
- [ ] **Accord sujet–verbe**
  - [ ] Personne + nombre : "ils mange" → "ils mangent"
  - [ ] Gestion des inversions : "mange-t-il" ne doit pas déclencher
  - [ ] Sujets composés : "Pierre et Marie mangent"
- [ ] **Accord adjectif attribut**
  - [ ] "elle est content" → "elle est contente"
  - [ ] Gestion de être, sembler, paraître, devenir, rester
- [ ] **Homophones grammaticaux — fréquents**
  - [ ] a/à : "il va a Paris" → "il va à Paris"
  - [ ] ou/où : "je sais ou il est" → "je sais où il est"
  - [ ] ce/se : "il ce lève" → "il se lève"
  - [ ] mes/mais : "mes je ne sais pas" → "mais je ne sais pas"
  - [ ] son/sont : "il son parti" → "ils sont partis"
  - [ ] on/ont : "ils on mangé" → "ils ont mangé"
- [ ] **Suite de tests grammaticaux** : corpus de phrases annotées (correct / incorrect / correction attendue)
- [ ] Benchmark de performance : <5 ms par phrase de 20 mots

---

### Phase 3 — Intégrations (semaines 6–8)

**Objectif** : livrable utilisable par d'autres applications.

- [ ] **Plugin Tauri v2 complet**
  - [ ] Commande `check_text` avec sérialisation `JsSuggestion`
  - [ ] `Checker` initialisé une seule fois via `app.manage()`
  - [ ] Types TypeScript générés (`hugo.d.ts`)
  - [ ] Exemple d'intégration dans une app Tauri + React
- [ ] **C FFI**
  - [ ] `hugo_checker_new()`, `hugo_checker_check()`, `hugo_free_results()`
  - [ ] Header C généré par `cbindgen`
  - [ ] Build en `staticlib` pour iOS/macOS
  - [ ] Package XCFramework pour Swift
  - [ ] Wrapper Swift (`Hugo.swift`) avec API idiomatique
- [ ] **WASM**
  - [ ] Build avec `wasm-pack`
  - [ ] Package npm `hugo-wasm` avec types TypeScript
  - [ ] Exemple d'utilisation dans une app web
- [ ] **Documentation**
  - [ ] rustdoc sur tous les types et fonctions publics
  - [ ] Guide d'intégration dans le README
  - [ ] Exemples dans `examples/`

---

### Phase 4 — Grammaire avancée (mois 3+)

**Objectif** : CRF pour la désambiguïsation POS + règles nécessitant un contexte syntaxique précis.

- [ ] **CRF (Conditional Random Field)**
  - [ ] Entraînement sur Universal Dependencies French-GSD
  - [ ] Modèle compact (~2 MB)
  - [ ] Intégration dans le pipeline entre morpho lookup et moteur de règles
  - [ ] Précision POS cible : >97%
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
