# Intégrer Hugo via le C FFI (C, C++, Swift, Kotlin/JNI)

Le crate `hugo-ffi` expose une API C stable au-dessus de `hugo-core`. Elle
permet d'embarquer le correcteur dans des applications natives — Swift (iOS /
macOS), C/C++, ou Kotlin/Java via JNI. Tout tourne **en local** : dictionnaire
et lexique morphologique sont liés dans la bibliothèque.

- Header : [`crates/hugo-ffi/include/hugo.h`](../crates/hugo-ffi/include/hugo.h)
- Wrapper Swift : [`crates/hugo-ffi/swift/Hugo.swift`](../crates/hugo-ffi/swift/Hugo.swift)
- Démo C : [`crates/hugo-ffi/examples/c_demo.c`](../crates/hugo-ffi/examples/c_demo.c)

---

## 1. Construire la bibliothèque

`hugo-ffi` produit un `staticlib` (`.a`) et un `cdylib` (`.dylib`/`.so`) :

```bash
cargo build -p hugo-ffi --release
# → target/release/libhugo_ffi.a  (et libhugo_ffi.dylib)
```

---

## 2. API C

```c
HugoChecker *hugo_checker_new(void);
void         hugo_checker_free(HugoChecker *checker);
HugoResults  hugo_checker_check(const HugoChecker *checker, const char *text);
void         hugo_free_results(HugoResults results);
```

```c
typedef struct {
  size_t start;             /* offset d'OCTET de début (inclus)  */
  size_t end;               /* offset d'OCTET de fin   (exclu)   */
  char  *message;
  char **replacements;
  size_t replacements_len;
  char  *rule_id;
} HugoSuggestion;

typedef struct {
  HugoSuggestion *suggestions; /* NULL si len == 0 */
  size_t          len;
} HugoResults;
```

### Règles du cycle de vie

- chaque `HugoChecker *` issu de `hugo_checker_new()` se libère **une fois** avec
  `hugo_checker_free()` ;
- chaque `HugoResults` issu de `hugo_checker_check()` se libère **une fois** avec
  `hugo_free_results()` — cela libère aussi toutes les chaînes internes ; ne les
  libérez pas individuellement ;
- toutes les fonctions tolèrent `NULL` et les chaînes non-UTF-8 (résultat vide),
  et ne paniquent **jamais** à travers la frontière FFI ;
- `start`/`end` sont des offsets d'**octets** UTF-8, pas des index de caractères.

### Régénérer le header

Le header est versionné. Avec [`cbindgen`](https://github.com/mozilla/cbindgen) :

```bash
cbindgen --config crates/hugo-ffi/cbindgen.toml \
         --crate hugo-ffi \
         --output crates/hugo-ffi/include/hugo.h
```

---

## 3. Exemple C

```bash
cargo build -p hugo-ffi --release
clang crates/hugo-ffi/examples/c_demo.c \
      -I crates/hugo-ffi/include \
      -L target/release -lhugo_ffi \
      -framework CoreFoundation -framework Security \
      -o /tmp/hugo_demo
/tmp/hugo_demo
```

```
Texte : "il il va a Paris"
2 suggestion(s)

[3..5] Mot répété : « il ».  (duplicate_word)
[9..10] Confusion d'homophones : « a » devrait être « à ».  (homophone)
    -> à
```

> Sur macOS, le `staticlib` Rust requiert les frameworks système
> `CoreFoundation` et `Security` au moment de l'édition de liens (voir
> ci-dessus). Sous Linux, liez plutôt `-lpthread -ldl -lm`.

---

## 4. Swift / iOS / macOS

### a. Construire le XCFramework

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim \
  x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
./scripts/build-xcframework.sh
# → target/xcframework/Hugo.xcframework
```

Le script compile pour iOS (device), le simulateur (arm64 + x86_64) et macOS
(arm64 + x86_64), puis assemble le `.xcframework` avec le dossier `headers`
(qui contient `hugo.h` et `module.modulemap`, exposant le module `CHugo`).

### b. Intégrer dans Xcode / SwiftPM

1. Glissez `Hugo.xcframework` dans votre cible (ou référencez-le via
   `.binaryTarget` dans un `Package.swift`).
2. Ajoutez [`Hugo.swift`](../crates/hugo-ffi/swift/Hugo.swift) à votre cible.
3. Utilisez l'API Swift idiomatique :

```swift
let checker = HugoChecker()
let text = "il va a Paris"
for s in checker.check(text) {
    print(s.message, s.replacements)            // accès direct
    if let r = s.stringRange(in: text) {        // plage octets → String.Index
        print(text[r])
    }
}
```

`HugoChecker` libère ses ressources natives dans `deinit`. Il n'est pas
thread-safe : une instance par thread, ou sérialisez les appels.

---

## 5. Kotlin / Java (JNI) — esquisse

Le `cdylib` (`libhugo_ffi.so`) est chargeable depuis la JVM. Deux options :

- déclarer les fonctions `hugo_*` via **JNA**/**JNR-FFI** (pas de code natif
  supplémentaire à écrire) ;
- ou écrire une fine couche JNI exposant `String[] check(String)`.

L'API C étant stable et sans rappel (callbacks), JNA est le chemin le plus
court. (Wrapper Kotlin de référence : à venir — voir la feuille de route.)

---

## 6. Dépannage

| Symptôme | Cause probable |
|---|---|
| `Undefined symbols … _CFRelease` à l'édition de liens (macOS) | Frameworks manquants : ajoutez `-framework CoreFoundation -framework Security`. |
| `library 'hugo_ffi' not found` | Mauvais `-L` : pointez sur `target/release` (ou le triple cible). |
| `No such module 'CHugo'` (Swift) | XCFramework non construit/ajouté, ou `module.modulemap` absent du dossier `headers`. |
| Offsets décalés sur accents/emoji | `start`/`end` sont en **octets** UTF-8 (voir `stringRange(in:)`). |

---

## Voir aussi

- [`docs/tauri-integration.md`](tauri-integration.md) — intégration côté Tauri/JS.
- [`ROADMAP.md`](../ROADMAP.md) — phase 3 (intégrations) et suite.
