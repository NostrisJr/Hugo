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
- [ ] Tranche 4 — quel(s)/quelle(s)/qu'elle(s), quand/quant/qu'en, sans/s'en/c'en
- [ ] Tranche 5 — terminaisons homophones (-er/-é/-ez, -ai/-ais/-ait)
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
- [ ] **Capitalisation : « … » non terminal** — « (perte, vol, virus…) perdu »
  réclame à tort une majuscule sur « perdu » : les points de suspension dans une
  parenthèse (et, plus largement, suivis d'une minuscule) ne ferment pas la
  phrase. Correctif côté tokenizer/`capitalization` (suivi de la profondeur de
  parenthèses + heuristique de continuation). *Item adjacent, hors du chantier
  « identification du sujet ».*
- [ ] **sur / sûr** — « sur le côté » → suggestion erronée « sûre » (la
  préposition « sur », aussi adjectif « sur » = acide au lexique, traitée comme
  attribut à accorder). Non reproduit isolément ; à rejouer sur le paragraphe
  complet, vraisemblablement réglé par la même garde POS (n'accorder qu'un
  attribut étiqueté `ADJ`). *Item adjacent.*

**Sources d'inspiration du corpus** (toujours reformulées, jamais copiées) :
Projet Voltaire, Question Orthographe, Banque de dépannage linguistique,
listes de difficultés du français. Les cas de Grammalecte/LT servent de
**checklist de phénomènes** uniquement.

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
