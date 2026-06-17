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
  - [ ] ou/où : nécessite un étiquetage contrefactuel (phase 4+)
  - [x] ce/se : "il ce lève" → "se" (ce→se, heuristique) **et** "se chat" → "ce"
    (se→ce, via les étiquettes POS du CRF — `Rule::check_tagged`)
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
> hors phase 2 les homophones qui exigent un contexte POS riche. Depuis la
> **phase 4** (CRF), le sens **se→ce** est traité ; ou/où et sont→son attendent
> un étiquetage contrefactuel (l'étiqueteur « rattrape » l'erreur dans la phrase
> fautive).

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

- [x] **CRF (Conditional Random Field)** — désambiguïsation POS
  - [x] Entraînement sur Universal Dependencies French-GSD (`tools/train-crf` :
    CoNLL-U → features partagées → forward-backward + L-BFGS maison, pur Rust)
  - [x] Modèle compact embarqué (`assets/pos.crf`, **2,6 Mo**, poids quantifiés i16)
  - [x] Intégration dans le pipeline entre morpho lookup et moteur de règles
    (`pos::tag` dans `Checker::check`, exposé aux règles via `Rule::check_tagged`)
  - [x] Précision POS : **97,8 % (dev) / 97,6 % (test)** — cible >97 % atteinte
  - [x] *Suivi* : réécriture des règles pour consommer la catégorie unique —
    fait pour `agreement`/`epithet`, puis `conjugation` et `attribute`
    (cf. « Robustesse de l'identification du sujet », phase 6)

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

**Débloqué.** Toutes ces règles consomment désormais le POS via
`Rule::check_tagged` : se→ce et son↔sont (`rules::homophones`, étiquetage
contrefactuel) ; robustesse aux homographes de l'accord déterminant–nom et
épithète ; participe passé avec « avoir » (auxiliaire vs. verbe plein) et
pronominal ; subjonctif ; accords spéciaux. Restent ou/où (pas de signal
séparable) et la concordance des temps.
- [~] **Accord du participe passé**
  - [x] Avec être : "elle est parti" → "elle est partie" (`rules::attribute` +
    `morpho::participle`), y compris sujets nominaux ("les invités sont venu" →
    "venus")
  - [~] Avec avoir + COD antéposé (`rules::past_participle`, consomme le POS) :
    « je les ai vu » → « vus »/« vues » (les deux genres proposés pour *les*),
    « il la a vu » → « vue ». Limité aux clitiques objets non ambigus *les*/*la*
    (me/te/nous/vous, ambigus sujet/objet, et *l'* indéterminé, sont écartés).
  - [x] Verbes pronominaux (`rules::pronominal_participle`, consomme le POS) :
    « elle s'est levé » → « levée », « ils se sont trompé » → « trompés ».
    Garde du **COD postposé** : « elle s'est lavé les mains » reste invariable.
    `rules::attribute` délègue désormais ces constructions à la règle dédiée.
- [x] **Accord de l'adjectif épithète** (`rules::epithet`)
  - [x] Adjectifs antéposés et postposés : "les chats noir" → "noirs",
    "les petit chats" → "petits"
  - [x] Robustesse aux homographes nom/adjectif (« douce », « principales »…) :
    correction conjointe dans les règles déterminant–nom et sujet–verbe
  - [x] Robustesse POS (`check_tagged`) : la tête nominale doit être étiquetée
    nom — corrige le faux positif « est » (auxiliaire vs. nom « l'Est »)
  - [ ] Correction conjointe déterminant + adjectif ("une grande maisons" →
    "de grandes maisons") — partiellement (adjectif et déterminant corrigés
    séparément)
- [~] **Accord déterminant–nom** (`rules::agreement`)
  - [x] Robustesse POS (`check_tagged`) : tête nominale via les étiquettes (sauts
    d'adjectifs antéposés) ; garde anti pronom objet (« je les ferme » n'est plus
    pris pour un déterminant–nom)
- [~] **Subjonctif** (`rules::subjunctive`, consomme le POS)
  - [x] Après un ensemble fermé non ambigu : bien que, pour que, afin que,
    avant que, sans que, pourvu que, quoique, (à) condition que, de peur/crainte
    que, quoi que — « bien qu'il est » → « bien qu'il soit », « pour que tu
    viens » → « viennes ». Ne touche pas les formes où indicatif = subjonctif
    (« bien qu'il mange »).
  - [x] Sujets **nominaux** : « bien que les enfants sont » → « soient »
  - [ ] Conjonctions ambiguës (de sorte que…) et concordance des temps (imparfait
    → subjonctif imparfait) — écartées (correction de temps ambiguë)
- [~] **Accords spéciaux** (`rules::special_agreement`, consomme le POS)
  - [x] Couleurs **issues de noms**, invariables : « des gants marrons » →
    « marron », « des yeux noisettes » → « noisette » (postposées à un nom ;
    rose/mauve/pourpre… exclues car elles s'accordent)
  - [x] **même** : « les même livres » → « mêmes » (adverbe « même les… » exclu) ;
    **quelque** : « quelque livres » → « quelques » (« quelque chose » exclu)
  - [x] *tout* : "toute les jours" → "tous les jours" (`rules::quantifier`)
  - [ ] Adjectifs de couleur **composés** : "des robes bleu ciel" (deux mots)
  - [ ] Trait d'union dans les nombres composés (post-réforme 1990)
- [ ] **Moteur de règles déclaratif** → déplacé en **phase 6** (capitalisation sur
  Grammalecte/LT sans porter leur code ni leurs données).

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

### Phase 6 — Moteur de règles déclaratif & capitalisation sur Grammalecte/LT (mois 4+)

**Objectif** : faire croître la couverture grammaticale (« se conformer le plus
possible au français ») en réutilisant le *savoir* accumulé par Grammalecte et
LanguageTool, sans hériter de leurs contraintes.

**Décision stratégique.** Cible inchangée : correcteur **pur-Rust, embarquable,
rapide** (pas de serveur). Grammalecte (GPL v3, Python, lent) et LanguageTool
(LGPL 2.1, JVM) **ne sont pas portables tels quels** : leurs ~1 500 / ~700+
règles sont **couplées à leur moteur** (Python embarqué — `morph/select/define`
qui mutent la morpho — pour Grammalecte ; XML + classes Java pour LT). On ne porte
donc **ni leur code ni leurs données** (listes curées et phrases de test = œuvres
sous copyleft). On réutilise leur **savoir** — la *cartographie* des phénomènes du
français et de leurs exceptions, qui relève des faits, non protégeables — en le
réimplémentant dans **notre** moteur, avec **nos** listes et **notre** corpus.

**Principe anti-fragilité (emprunté à Grammalecte).** La morphologie + le CRF font
le tri ; les exceptions sont des **données explicites** ; **aucune règle sans son
corpus de test** (leurs 14 160 `TEST:` sont leur vrai trésor — on adopte le
format, pas le contenu).

**Moteur cible** (`crates/hugo-core/src/rules/confusion.rs` ou un module `engine`) :
- Une règle = **motif de tokens** (littéral / catégorie POS / condition morpho)
  + **antipatterns** (listes d'exceptions) + **cible & suggestion** (littéral ou
  forme engendrée par `morpho::decline/conjugate/participle`).
- S'exécute sur les tokens **et** les tags CRF déjà disponibles ; matcheurs
  compilés (rapide, embarquable). Bien plus efficace que Python + regex sur texte.
- Règles + corpus stockés en données versionnées (`corpus/*.md`).

**Tranches verticales** (par famille, chacune livrée avec son corpus reconstitué) :
- [x] **Tranche 1 — a/à** (`rules::confusion`, consomme le POS) : règle générale
  `NOM + a + NOM → à` + liste d'idiomes *avoir* ([`AVOIR_IDIOMS`], reconstituée) ;
  `a + infinitif → à` ; `verbe + a → à` (veto POS sur l'homographe « y » de
  « il y a ») ; locutions figées. Direction à→a en remontant au-delà des
  clitiques objets (`y / l' / les / nous / ne …`) : pronom sujet fort
  (`il/elle/on → a`, `ils/elles → ont`), sujet nominal/relatif `qui` gardé par un
  participe passé suivant. Remplace le veto fragile de `rules::homophones`.
  L'homographe nom/verbe mal étiqueté par le CRF (« moulin a poivre ») est
  rattrapé en consultant les **lectures possibles** du lexique (nom sans participe
  passé), à la Grammalecte, plutôt que l'unique tag forcé.
  Corpus : [`corpus/confusion-a-a.md`](corpus/confusion-a-a.md) — **19/20** cas
  fautifs captés, **0 faux positif**. Seul gap restant : `a + déterminant + nom`
  (« tarte a la rhubarbe »), ambiguïté **structurelle** — « avoir + det + nom »
  est productif et correct (« il a la grippe », « le gâteau a la forme d'un
  cœur ») ; aucune liste d'exceptions ne sépare les deux (même nom des deux
  côtés). Résoluble seulement par une heuristique **syntaxique** (la proposition a
  déjà son verbe → le nom est un COD, pas un sujet), à tenter ultérieurement.
- [x] **Tranche 2 — ce/se, c'est/s'est** (`rules::confusion::ce_se`, consomme le
  POS) : **ce→se** (sujet pronom + ce + verbe conjugué, « ne » sauté) ; **se→ce**
  (se devant un nom/adjectif tagué, ou un relatif que/qui/dont) — reprend et
  enrichit le traitement historiquement porté par `rules::homophones`, désormais
  retiré de cette dernière (comme la tranche 1 pour a/à) ; **c'→s'** (sujet de 3ᵉ
  personne `il/elle/on`/`qui`/nom, sans virgule, + « c'est »/« c'était » + participe
  passé → « s' est ») ; **s'→c'** (« s'est » non suivi d'un participe → « c'est »,
  adverbes sautés). Corpus :
  [`corpus/confusion-ce-se.md`](corpus/confusion-ce-se.md). Gaps assumés :
  **ces/ses** (deux déterminants également valides — ambiguïté structurelle, comme
  a+det+nom) ; **sais/sait** (déjà traités par l'accord sujet–verbe) ; homographes
  nom/verbe mal tagués après « se » (« se livre ») et participes hors lexique.
- [x] **Tranche 3 — ou/où, la/là/l'a, leur/leurs, peu/peut/peux** (une famille par
  module dans `rules::confusion`, chacune consommant le POS et livrée avec son
  corpus reconstitué) :
  - **ou/où** (`ou_ou::OuConfusion`, [`corpus/confusion-ou-ou.md`](corpus/confusion-ou-ou.md)) :
    seule direction **ou→où** a un signal séparable — antécédent de lieu/temps
    (liste fermée, ou `là`/`ici`/`partout`) + « ou » + **pronom sujet** →
    relative → « où » (« le jour ou je suis né » → « où »). **où→ou** reste un
    gap **structurel** (« où » relatif suit indifféremment un déterminant ou un
    sujet, comme l'alternative).
  - **la/là/l'a** (`la_la::LaConfusion`, [`corpus/confusion-la-la.md`](corpus/confusion-la-la.md)) :
    **là→la** devant un nom non explicitement masculin (« là maison » → « la ») ;
    **la→l'a** sujet 3ᵉ pers. + « la » + participe passé (« il la mangé » →
    « l'a »). Gaps : la→là adverbial (homographie pronom objet), là→la masculin
    (« le », article manquant).
  - **leur/leurs** (`leur_leurs::LeurConfusion`, [`corpus/confusion-leur-leurs.md`](corpus/confusion-leur-leurs.md)) :
    accord du possessif (« leur livres » → « leurs », « leurs maison » → « leur »)
    et pronom invariable devant un verbe (« je leurs parle » → « leur »). Gap :
    noms non marqués/invariables en nombre au lexique.
  - **peu/peut/peux** (`peu_peut::PeuConfusion`, [`corpus/confusion-peu-peut.md`](corpus/confusion-peu-peut.md)) :
    **peu→peut/peux** (sujet + « peu » + infinitif, forme selon la personne) ;
    **peut/peux→peu** (quantifieur précédent, ou *avoir* + … + « de »). La
    confusion de **personne** peux↔peut est laissée à l'accord sujet–verbe.
- [x] **Tranche 4 — quel(s)/quelle(s)/qu'elle(s), quand/quant, sans/s'en** (une
  famille par module dans `rules::confusion`, chacune consommant le POS et livrée
  avec son corpus reconstitué) :
  - **quel/qu'elle** (`quel_quelle::QuelConfusion`, [`corpus/confusion-quel-quelle.md`](corpus/confusion-quel-quelle.md)) :
    **qu'elle(s)→quel(le)(s)** (« qu'elle heure » → « quelle », fusion des deux
    jetons, accord avec le nom/adjectif qui suit — un pronom n'est jamais suivi
    d'un groupe nominal) ; **quelle(s)→qu'elle(s)** (forme féminine + verbe
    conjugué non *être*/*avoir*, clitiques sautés). Comme à la Grammalecte, la tête
    nominale est reconnue par les **lectures du lexique** (le CRF étiquette « heure »
    verbe sous l'effet de l'erreur). Gaps : « quelle est/a été… » (ambiguïté
    **structurelle** interrogatif ↔ subordonnée) ; masculin « quel→qu'il » (autre

quand_quant::QuandConfusion`, [`corpus/confusion-quand-quant.md`](corpus/confusion-quand-quant.md)) :
    **quand→quant** (« quand à/au/aux » → « quant », virgule intercalée exclue) et
    **quant→quand** (« quant » non suivi de « à/au/aux », son seul emploi licite).
    Gap : **qu'en** (« qu'en penses-tu » ↔ « quand penses-tu »), pas de signal
    séparable — « quant » fautif est toujours rendu « quand ».
  - **sans/s'en** (`sans_sen::SansConfusion`, [`corpus/confusion-sans-sen.md`](corpus/confusion-sans-sen.md)) :
    **sans→s'en** (sujet 3ᵉ pers. + « sans » + verbe conjugué, « ne » sauté — la
    préposition ne s'insère pas là) ; **s'en→sans** (« s' » + « en » + tête nominale,
    fusion des deux jetons ; lectures du lexique pour écarter les homographes
    nom/verbe « s'en aller »). Gaps : **c'en** (« c'en est trop », trop rare) ;
    nom homographe d'un verbe après « s'en » (précision > rappel).
- [x] **Tranche 5 — terminaisons homophones -er/-é/-ez** (`terminaisons::TerminaisonsConfusion`,
  [`corpus/confusion-terminaisons.md`](corpus/confusion-terminaisons.md)) : pour un
  verbe du 1ᵉʳ groupe, infinitif (« manger »), participe masc. sg. (« mangé ») et
  2ᵉ pers. pl. (« mangez ») sont homophones (/e/). On tranche par le **gouverneur**
  (clitiques/adverbes/« ne » sautés) : **-é** après auxiliaire `avoir`/`être`
  (accord avec le sujet pour `être`) ; **-er** après préposition infinitive
  (`à/de/pour/sans`, garde sur « rien **de** changé ») ou semi-auxiliaire conjugué
  (liste [`SEMI_MODALS`]) ; **-ez** après un « vous » sujet en tête de proposition.
  Les homographes (nom « fer/clé/nez/côté », participes des 2ᵉ/3ᵉ groupes
  « parti/fait ») sont écartés par les lectures du lexique (lemme en `-er` exigé).
  Gaps assumés : **-ai/-ais** (futur ↔ conditionnel, ambiguïté **sémantique**) ;
  **-ais/-ait** (personne — déjà couvert par l'accord sujet–verbe) ; « vous » sujet
  hors tête de proposition ; accord du participe avec COD antéposé (laissé à
  `past_participle`).
- [x] **Robustesse de l'identification du sujet (consommation du POS)** —
  *chantier prioritaire* regroupant une famille de **faux positifs** où un
  complément voisin est pris pour le sujet (ou un nom homographe pour un verbe).
  Racine commune : `conjugation` et `attribute` raisonnent encore sur la
  morphologie brute (`Rule::check`, **sans** les tags CRF) et leur recherche du
  sujet est purement locale. Cas recensés (cf. [`PROBLEMES.md`](PROBLEMES.md),
  reproduits via `examples/check`) :
  - **nom homographe lu comme verbe** : « des points/barres verticales » →
    `barres` (étiqueté `NOUN` par le CRF) déclenché comme verbe. Correctif : le
    verbe candidat doit être étiqueté `VERB`/`AUX`.
  - **COD d'un infinitif pris pour sujet** : « …redéployer des postes
    compromis » → « des postes » (objet de l'infinitif `redéployer`) pris pour
    sujet, puis `compromis` accordé. La garde `is_finite_verb(prev)` ignore les
    infinitifs ; correctif : refuser un GN sujet précédé d'un token `VERB`/`AUX`.
    (À noter : `compromis` est ici **mal** étiqueté `VERB` par le CRF — la garde
    côté verbe ne suffit pas, d'où l'importance de la garde côté sujet.)
  - **complément en de+les / de+le** : « la démultiplication des usages digitaux
    alla… » → « des usages » (complément partitif de « démultiplication ») pris
    pour sujet. Correctif : « des »/« du » précédés d'un **nom** = complément,
    pas un nouveau sujet.
  - **objet de relative** : « le problème qui nous a été posé » → « nous »
    (clitique objet) pris pour sujet. Correctif : un pronom précédé d'un relatif
    `qui/que/dont/où` est objet ; le sujet est l'antécédent (« problème »).
  - **objet de groupe prépositionnel (attribut)** : « L'outil … dans une
    nouvelle ère fut nommé » → « ère » (objet de « dans ») pris pour sujet de
    `nommé`. Correctif : en remontant vers le sujet, refuser un nom introduit par
    une préposition (objet de PP) ; à défaut de sujet sûr dans la fenêtre, ne
    rien émettre (**précision > rappel**).
  - **proposition participiale** (bug historique) : « les filles fatiguant leur
    père sont fatigantes » → « père » pris pour sujet. **Réglé** par la même
    garde (`attribute::is_governed_left`) : un nom gouverné à gauche par un verbe
    — typiquement un participe présent — n'est pas le sujet de la copule.

  **Fait** : `conjugation` et `attribute` portés sur `check_tagged` (`check`
  conservé en repli), avec gardes POS dédiées (`verb_candidate_ok`,
  `detect_subject(.., tags)` côté sujet–verbe ; `find_subject` +
  `is_governed_left` côté attribut) et **corpus de non-régression** dans
  `tests/grammar.rs`. Les 6 symptômes ci-dessus sont neutralisés. A débloqué
  l'item « Suivi » de la phase 4.
- [x] **Capitalisation : « … » non terminal** — « (perte, vol, virus…) perdu »
  réclamait à tort une majuscule. **Réglé** dans `capitalization` : (a) suivi de
  la **profondeur de parenthèses** — une ponctuation terminale dans une
  parenthèse ne ferme pas la phrase porteuse ; (b) **heuristique de continuation**
  — les points de suspension sont un terminateur **ambigu** (`Pending::Ellipsis`),
  distinct des terminateurs fermes `.`/`!`/`?` (`Pending::Hard`) : suivis d'une
  minuscule, ils signalent une suspension intra-phrase (« il hésita… puis
  partit ») et n'imposent pas de majuscule (précision > rappel). Corpus de
  non-régression dans `capitalization`. *Item adjacent, hors du chantier
  « identification du sujet ».*
- [x] **sur / sûr** — « elle est sur le côté » → suggestion erronée « sure » (la
  préposition « sur », aussi adjectif « sur » = acide au lexique, traitée comme
  attribut à accorder). **Réglé** par une garde POS côté **attribut** dans
  `attribute` (`is_attribute_tag`) : on refuse un attribut candidat étiqueté
  `ADP`/`DET`/`PRON`/… par le CRF. Les attributs réels, même mal étiquetés `VERB`
  (« content », « parti », « allé »), restent acceptés ; corpus de non-régression
  dans `attribute` (`pos_path_preposition_attribute_is_ignored`) et `tests/grammar.rs`.

**Sources d'inspiration du corpus** (toujours reformulées, jamais copiées) :
Projet Voltaire, Question Orthographe, Banque de dépannage linguistique,
listes de difficultés du français. Les cas de Grammalecte/LT servent de
**checklist de phénomènes** uniquement.

---

## Cartographie Grammalecte → écarts de Hugo

> Lecture des familles de règles exposées par Grammalecte (`gc_lang/fr`,
> options de la barre de configuration) et état de couverture côté Hugo. Sert
> de boussole pour les phases 7+. Rappel méthodologique : on réutilise la
> **carte des phénomènes** (faits du français, non protégeables), jamais le
> code, les listes curées ni les phrases `TEST:` de Grammalecte.

| Famille Grammalecte | Couverture Hugo | Reste à faire |
|---|---|---|
| **Conjugaison** (accord sujet–verbe) | ✅ `conjugation` | sujets `qui` relatif, clivées « c'est … qui/que », « la plupart/beaucoup de » + verbe |
| **Accords GN** (det/adj/nom) | ✅ `agreement`, `epithet`, `attribute` | numéraux (vingt/cent), demi/nu/mi, adj. verbal vs participe présent, couleurs composées, correction conjointe det+adj |
| **Accords sujets coordonnés** (genre mixte) | ✅ `attribute` phase 12-préparation | adjectif détaché + sujet postposé (phase 12) |
| **Participe passé** (être/avoir/pronominal) | ✅ (3 règles) | COD antéposé au-delà de *les/la*, « en » partitif |
| **Subjonctif** | 🟡 `subjunctive` | conjonctions ambiguës, concordance des temps |
| **Confusions / homophones** | 🟡 13 familles (phase 6 + `homophones`) | et/est, ces/ses, ni/n'y, si/s'y, sa/ça, ma/m'a, dans/d'en, près/prêt, plutôt, du/dû, notre/nôtre, dont/donc, etc. (phase 9) |
| **Confusions fin de verbe** (-er/-é/-ez) | ✅ `terminaisons` | -ai/-ais (sémantique), assumé |
| **Élisions** | ❌ | le/ce/si/que + voyelle, jusqu', lorsqu'… (phase 8) |
| **Typographie** (apostrophe, guillemets, tirets, …) | 🟡 `typography` (apostrophe, suspension, doublons, guillemets, ligatures) | tirets (incise/dialogue), signes (`×`, `©`) |
| **Espaces** (insécables, surnuméraires, manquants) | 🟡 `typography` (`typo_space`, `typo_nbsp`) | insertion d'insécables manquantes, espace après `. ! ? :` |
| **Nombres** (séparateurs, ordinaux, romains) | 🟡 `typography` (`typo_ordinal`) | séparateur de milliers, romains, `%`/`°`/`€` |
| **Majuscules** | 🟡 `capitalization` (début de phrase) | sigles, après deux-points, gentilés |
| **Traits d'union et soudures** | ❌ | nombres composés, impératif + pronom, inversions (phase 8/10) |
| **Pléonasmes** | ❌ | phase 11 |
| **Négations** (« ne » manquant) | ❌ | phase 11 (registre) |
| **Style** (répétitions, anglicismes, lourdeurs) | ❌ | phase 11, optionnel/désactivable |

---

### Phase 7 — Typographie, ponctuation, espaces, nombres

**Objectif** : reproduire le module le plus utilisé de Grammalecte — entièrement
**déterministe**, sans CRF ni morphologie, à très haute précision. Travaille sur
les tokens et le texte brut ; chaque sous-règle est **activable/désactivable**
(les conventions typographiques varient selon les contextes).

> **État (phase 7 — cœur livré)** : module `rules::typography`, 8 règles
> déterministes (`typo_*`), 34 tests unitaires + corpus `corpus/typo-*.md`.
> Elles n'opèrent que sur les jetons d'**espace**/**ponctuation**/**nombre** —
> ignorés par les règles grammaticales — donc sans interférence. Restent en
> attente (signalés ci-dessous) tirets, signes, milliers et romains.
>
> **Activation/désactivation** (demande explicite, transverse à toutes les
> règles, pas seulement la typo) : le `Checker` reste **immuable et partagé** ;
> on filtre **par appel** via [`CheckOptions`] (liste de `rule_id` désactivés,
> défaut = tout actif) passée à `Checker::check_with`. `Checker::rule_catalog()`
> fournit `(id, nom)` de **toutes** les règles (orthographe incluse) pour
> alimenter une UI. Côté plugin Tauri : `check_text` accepte `disabledRules` et
> la commande `list_rules` expose le catalogue.

- [x] **Apostrophe typographique** (`typo_apostrophe`) : `'` droite → `'`
      (U+2019) en position d'élision (`l'`, `qu'`…) et apostrophe interne
      (`aujourd'hui`). Garde : lettre des deux côtés (épargne `5'`, guillemet
      simple).
- [x] **Points de suspension** (`typo_ellipsis`) : `...` (3+ points contigus)
      → `…`. `..` laissé au doublon.
- [x] **Espaces insécables** (`typo_nbsp`) : **conversion** d'une espace
      ordinaire existante en fine insécable avant `;` `!` `?`, insécable avant
      `:` `»` et après `«`. On n'**insère** pas une espace absente (garde URL
      `http://`, heures `12:30`, code).
- [x] **Espaces surnuméraires** (`typo_space`) : doubles espaces, espace avant
      `,` `.` `)`, espace après `(`.
- [~] **Espaces manquants** (`typo_space`) : après `,` `;` (garde sur les
      décimales `3,14`). `: . ! ?` **non traités** (homographes URL/abréviation,
      précision > rappel).
- [x] **Guillemets** (`typo_quotes`) : paire `"…"` → `« … »` (avec insécables,
      espaces intérieures absorbées). Gaps : imbriqués `“ ”`, guillemet isolé.
- [ ] **Tirets** : tiret d'incise `—` (cadratin) / `–` (demi-cadratin) vs trait
      d'union `-` ; tiret de dialogue en début de ligne. *(différé)*
- [x] **Ligatures** (`typo_ligature`) : `coeur` → `cœur`, `oeuf` → `œuf`… via
      **liste curée** (garde par construction sur `coexister`, `moelle`,
      `goéland`…) ; casse respectée (`CŒUR`). Gaps : `æ`, composés à trait
      d'union.
- [x] **Doublons de ponctuation** (`typo_punct_doubling`) : `!!`, `??`, `,,`,
      `;;`, `..` → simple (garde sur `!?`, `?!` ; `…` exclu).
- [~] **Nombres** (`typo_ordinal`) : ordinaux mal formés `1ère`→`1re`,
      `2ème`/`2ième`→`2e`, pluriels (`2es`) ; `2nd(e)` admis. **Différés** :
      séparateur de milliers, nombres romains, `%`/`°`/`€`.
- [ ] **Signes** : `x` → `×`, `(c)` → `©`, `n°` → `№` (optionnel). *(différé)*
- [x] **Corpus** : `corpus/typo-*.md` (apostrophe, ponctuation, espaces,
      guillemets, ligatures, nombres) — cas correct/incorrect/correction et
      **cas de non-déclenchement** (URLs, code, décimales).

---

### Phase 8 — Élisions et contractions obligatoires

**Objectif** : règles morphologiques déterministes manquantes, à fort rappel
(Grammalecte les classe « élisions »). S'appuie sur `morpho` (mot suivant
commence par une voyelle ou un *h* muet) + une **liste curée originale des *h*
aspirés** (héros, hibou, haricot, hêtre… → pas d'élision).

> **État (phase 8 — livrée)** : module [`rules::elision`] (`ElisionRule`,
> id `elision`). Détecte les élisions manquantes (`le arbre` → `l'arbre`,
> `de eau` → `d'eau`, `je ai` → `j'ai`, `si il` → `s'il`, `que il` → `qu'il`,
> `ce arbre` → `cet arbre`, `lorsque il` → `lorsqu'il`…) et les élisions
> fautives devant h aspiré (`l'héros` → `le héros`). Liste [`H_ASPIRES`]
> originale (≈ 60 lemmes). Corpus : [`corpus/elision.md`].

- [x] **Article/pronom + voyelle** : `le arbre` → `l'arbre`, `la école` →
      `l'école`, `je ai` → `j'ai`, `me a` → `m'a`, `te a`, `se est`, `ne est` →
      `n'est`, `de eau` → `d'eau`.
- [x] **ce → cet** devant voyelle/h muet : `ce arbre` → `cet arbre`, `ce homme`
      → `cet homme` ; garde *h* aspiré (`ce héros` reste `ce`).
- [x] **si + il/ils → s'il/s'ils** (jamais devant elle : `si elle` reste `si`).
- [x] **que → qu'** : `que il` → `qu'il`, `que elle`, `que on`, `que un`.
- [x] **Conjonctions** : `lorsque/puisque/quoique + il/elle/on/un/en` → `lorsqu'`,
      `puisqu'`, `quoiqu'`.
- [x] **Élision fautive inverse** : `l'héros` → `le héros` (h aspiré).
- [x] **Corpus** : [`corpus/elision.md`] + liste [`H_ASPIRES`] intégrée.
- [ ] *Gaps assumés* : `jusque → jusqu'`, `presque → presqu'île` (très ponctuels),
      `s'elle → si elle` (rare).

---

### Phase 9 — Confusions et homophones restants

**Objectif** : compléter la longue traîne des confusions de Grammalecte, sur le
modèle des tranches 1–5 (une famille = un module `rules::confusion`, consomme le
POS, livré avec son corpus). **Précision > rappel** : on ne traite que les
directions à signal séparable, on documente les gaps structurels.

> **État (phase 9 — livrée)** : 7 nouveaux modules de confusion livrés avec
> leurs corpus. Gaps structurels documentés.

- [x] **et/est** (`confusion::et_est`, [`corpus/confusion-et-est.md`]) :
      sujet singulier 3ᵉ pers. + `et` + attribut/PP → `est`.
- [ ] **ces/ses** : **gap structurel** assumé (deux déterminants valides, signal non séparable).
- [x] **ni/n'y**, **si/s'y** (`confusion::ni_ny`, [`corpus/confusion-ni-ny.md`]) :
      pronom sujet + `ni/si` + verbe conjugué → `n'y`/`s'y`/`m'y`/`t'y`.
- [x] **sa/ça**, **ma/m'a**, **ta/t'a** (`confusion::sa_ca`, [`corpus/confusion-sa-ca.md`]).
- [x] **dans/d'en** (`confusion::dans_den`, [`corpus/confusion-dans-den.md`]) :
      `dans` + infinitif → `d'en`.
- [x] **près/prêt** (`confusion::pres_pret`, [`corpus/confusion-pres-pret.md`]) :
      copule + `près` + `à` + infinitif → `prêt(e/s)`.
- [x] **plutôt/plus tôt**, **d'avantage/davantage** (`confusion::plutot`,
      [`corpus/confusion-plutot.md`]).
- [x] **du/dû**, **sur/sûr**, **notre/nôtre**, **votre/vôtre**, **mur/mûr**
      (`confusion::accents`, [`corpus/confusion-accents.md`]).
- [ ] *Gaps assumés* : **ces/ses**, **quelque/quel que**, **quoique/quoi que**,
      **parce que/par ce que**, **tant/temps/t'en**, **dont/donc**, **peut-être/peut être**,
      **différent/différend**, **censé/sensé**, **voie/voix** — signal non séparable
      ou nécessitant une analyse sémantique. Documentés pour phase 11.

---

### Phase 10 — Accords avancés et traits d'union

**Objectif** : refermer les écarts d'accord notés en phases 4/6 et couvrir les
accords numéraux/composés de Grammalecte.

> **État (phase 10 — livrée)** : 5 nouveaux modules livrés avec leurs corpus.
> Gaps couleurs composées et correction conjointe det+adj documentés.

- [x] **Numéraux** (`rules::numeraux`, [`corpus/accord-avance-numeraux.md`]) :
      `quatre-vingts-un` → `quatre-vingt-un` (composé) ; `deux cent euros` →
      `deux cents` (fin de numéral) ; `deux cents trois` → `deux cent` (avant unité).
- [x] **demi/nu/mi/semi** invariables antéposés (`rules::demi`,
      [`corpus/accord-avance-demi.md`]) : `une demie-heure` → `demi-heure` ;
      `nue-pieds` → `nu-pieds`.
- [x] **Adjectif verbal vs participe présent** (`rules::adjectif_verbal`,
      [`corpus/accord-avance-adjectif-verbal.md`]) : `fatiguant`/`fatigant`,
      `négligeant`/`négligent`, `provoquant`/`provocant`, `convainquant`/`convaincant`…
      (9 paires). Tranché par la copule (attribut → forme adjectivale) et par le
      gérondif `en` (participe présent → forme verbale).
- [ ] **Couleurs composées** (gap phase 4) : `des robes bleu ciel` (invariable) —
      pattern deux-mots difficile à distinguer des épithètes normales. Différé phase 11.
- [ ] **Correction conjointe det+adj** (gap phase 4) : `une grande maisons` →
      `de grandes maisons` — nécessite coordination entre deux règles (agreement +
      epithet). Différé phase 11.
- [x] **Accords par locution** (`rules::locutions`, [`corpus/accord-avance-locutions.md`]) :
      `la plupart/beaucoup/peu des N` + verbe singulier → pluriel ;
      `des plus/moins + adj singulier` → pluriel.
- [x] **Traits d'union impératif/inversion** (`rules::trait_union`,
      [`corpus/accord-avance-trait-union.md`]) : `donne moi` → `donne-moi`,
      `est ce que` → `est-ce que`, `dit il` → `dit-il`, `vas y` → `vas-y`.

---

### Phase 12 — Structure syntaxique et accords avec sujet postposé

**Objectif** : détecter les constructions où le **sujet est postposé** (inversion
stylistique) et accorder correctement les adjectifs détachés qui s'y rapportent.
Pré-requis : représentation des **propositions** (limites de clause) dans le pipeline.

#### 12.1 — Structure de propositions (`crates/hugo-core/src/clause.rs`)

> **État (phase 12.1 — livrée)** : module `clause` avec `Clause { start, end,
> verb_pos, subject_pos, inverted }` et `segment_clauses(tokens, tags) ->
> Vec<Clause>`. Détection O(n) par ponctuation + CRF. Inversion détectée via
> `find_subject_before` / `find_subject_after`. 4 tests unitaires verts.

- [x] `Clause` : struct { verb_pos, subject_pos: Option<usize>, start, end, inverted: bool }
- [x] `segment_clauses(tokens, tags) -> Vec<Clause>` : détection par ponctuation +
      CRF (VERB/AUX avec pronom sujet ou sujet nominal)
- [x] Inversion détectée quand le nom sujet est **après** le verbe fini
- [x] Tests unitaires sur phrases simples, inversées, multi-virgules

#### 12.2 — Accord de l'adjectif apposé avec sujet postposé

> **État (phase 12.2 — livrée)** : règle `rules::detached_appositive` (id
> `detached_appositive`). Deux motifs couverts : (1) apposé **entre deux
> virgules** puis verbe + sujet postposé ; (2) apposé **en tête de phrase**
> (avant la première virgule) puis verbe + sujet postposé. Accord via
> `morpho::decline` (adj) / `morpho::participle` (PP). 9 tests unitaires verts.
> Corpus : [`corpus/accord-appose-postpose.md`](corpus/accord-appose-postpose.md).
> Genre inconnu (épicènes) → masculin grammatical par défaut.

- [x] Nouvelle règle `rules::detached_appositive`
- [x] Détection du motif (deux variantes : enclosed + sentence-initial)
- [x] Accord en genre et nombre via `morpho::decline` / `morpho::participle`
- [x] Corpus : `corpus/accord-appose-postpose.md` (cas corrects, fautifs, non-déclenchement)
- [x] Garde anti-faux-positif : si le sujet est ambigu ou absent, ne rien émettre

#### 12.3 — Architecture de rendu asynchrone par priorité

> *Motivation* : les règles de la phase 12 (analyse de clause, adjectifs détachés)
> sont plus coûteuses que la typographie ou les confusions simples. Plutôt que de
> bloquer l'affichage jusqu'à la fin de toutes les règles, le correcteur peut
> **remonter les erreurs au fil de leur découverte**, par ordre de priorité :
>
> 1. **Orthographe** (DAWG lookup) — quelques µs par mot, priorité max
> 2. **Grammaire de base** (conjugation, agreement, attribute, confusion, typo) — < 5 ms
> 3. **Grammaire complexe** (adjectifs détachés, sujet postposé, style) — optionnel

**Architecture proposée :**

- `Checker::check_streaming(text, opts, sender: Sender<Suggestion>)` : exécute les
  passes dans l'ordre de priorité et envoie chaque `Suggestion` dès qu'elle est
  produite.
- Côté Tauri : commande `check_text_stream` retournant un **channel** Tauri v2
  (`tauri::ipc::Channel`) — l'UI applique les soulignements en temps réel sans
  attendre la passe « grammaire complexe ».
- Les règles sont groupées en trois **niveaux** (`RulePriority::Fast / Normal / Deep`)
  configurables via `CheckOptions`; les règles `Deep` peuvent être **annulées**
  si l'utilisateur retape avant la fin.
- Rétrocompatibilité : `Checker::check_with` reste synchrone (accumule toutes les
  suggestions des trois niveaux) ; seule la commande Tauri exposée change.

**Critère de déclenchement de cette phase :** à activer quand la latence totale des
règles dépasse 3 ms en mediane sur une phrase de 30 mots (à mesurer sur les règles
phase 12 réelles). Si la latence reste sous budget, le streaming peut être ajouté
comme amélioration UX pure sans urgence.

---

### Phase 11 — Négation, pléonasmes et style (module optionnel)

**Objectif** : couvrir le module « style » de Grammalecte — **désactivable par
défaut** (registre/goût), précision maximale, jamais bloquant.

- [ ] **« ne » manquant** (registre) : `je sais pas` → `je ne sais pas`,
      `il a rien dit` → `n'a rien` (signalé comme familier, non comme faute dure).
- [ ] **Pléonasmes** (liste curée originale) : `au jour d'aujourd'hui`,
      `voire même`, `comme par exemple`, `monter en haut`, `descendre en bas`,
      `prévoir à l'avance`, `s'avérer vrai/faux`, `car en effet`, `puis ensuite`.
- [ ] **Barbarismes / impropriétés** : `malgré que` → `bien que`, `pallier à` →
      `pallier`, `solutionner` → `résoudre`, `au final` → `en fin de compte`,
      `de par` (limité).
- [ ] **Répétitions** (style) : mot identique à courte distance — signalement
      doux, configurable.
- [ ] **Anglicismes** (optionnel, liste originale) : `supporter` (= accepter),
      `réaliser` (= comprendre), `digital` (= numérique)… — purement informatif.
- [ ] **Corpus** : `corpus/style-*.md` ; chaque sous-module derrière un flag
      d'activation distinct (l'API publique expose le niveau de sévérité).

---

## Données et licences

| Ressource | Licence | Usage |
|---|---|---|
| Lefff | LGPL-LR | Morphologie — libre commercial |
| Dicollecte fr_FR | MPL 2.0 | Orthographe — libre commercial |
| UD French-GSD | CC BY-SA 4.0 | Entraînement CRF (phase 4) |
| Grammalecte (gc_lang/fr) | GPL v3 | **Inspiration seule** — checklist de phénomènes ; aucun code, liste ni test copié (cf. phase 6) |
| LanguageTool rules/fr | LGPL 2.1 | **Inspiration seule** — idem ; savoir réutilisé, données reconstituées |
| Crate `fst` | MIT / Unlicense | FST et DAWG |
| Crate `wasm-bindgen` | MIT / Apache-2.0 | Bindings WASM |
| Tauri v2 | MIT / Apache-2.0 | Plugin natif |

---

## Notes

- Le nom `hugo` sur crates.io est à vérifier (peut-être pris par le générateur de sites statique Hugo en Go — dans ce cas, `hugo-fr` ou `hugo-spell`).
- Licence du projet : MIT / Apache-2.0 dual (standard Rust).
- Le CRF (phase 4) est la pièce la plus complexe. Les phases 1–3 fonctionnent avec la morphologie lexicale seule + heuristiques de contexte.
- **Complétion du genre** : Lexique383 laisse ~5 % des noms sans genre. La grande
  majorité est légitime (épicènes « un/une camarade » ; bi-genres « le/la tour,
  livre, poste »…), mais quelques dizaines sont de **vrais trous** mono-genre
  (« maison », « voiture », « main », « cours »…). Ils sont comblés par une liste
  curée **originale** ([`tools/compile-morpho/gender-overrides.tsv`](tools/compile-morpho/gender-overrides.tsv)),
  honorée par `compile-morpho` à la recompilation et appliquée à l'asset déjà
  compilé par [`tools/patch-morpho-gender`](tools/patch-morpho-gender) (la source
  Lexique383 n'étant pas vendue dans le dépôt). On n'écrase jamais un genre connu
  et on n'invente jamais celui des épicènes/bi-genres. Débloque l'accord en genre
  sur ces noms (« un maison » → « une maison »).
