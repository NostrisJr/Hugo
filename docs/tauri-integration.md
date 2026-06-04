# Intégrer Hugo dans une application Tauri v2

Ce guide explique comment ajouter le correcteur orthographique et grammatical
**Hugo** à une application [Tauri v2](https://v2.tauri.app/) via le plugin
`hugo-tauri`. Le correcteur tourne **entièrement en local** (pas de réseau, pas
de LLM) : le dictionnaire et le lexique morphologique sont embarqués dans le
binaire.

> **Versions** — Ce guide vise Tauri **v2**. Le plugin ne fonctionne pas avec
> Tauri v1.

---

## 1. Vue d'ensemble

Le plugin expose une seule commande, `check_text`, qui prend une chaîne et
renvoie la liste des suggestions de correction. Côté Rust, un unique
[`Checker`] est construit au démarrage et partagé via l'état de l'application
(`app.manage`), si bien que les dictionnaires ne sont chargés qu'une fois.

```
┌────────────┐   invoke("plugin:hugo-tauri|check_text")   ┌──────────────┐
│  Front-end │ ──────────────────────────────────────────▶│  Plugin Rust │
│  (JS/TS)   │ ◀──────────────────────────────────────────│  check_text  │
└────────────┘            Vec<JsSuggestion>                └──────┬───────┘
                                                                  │
                                                          hugo_core::Checker
                                                       (dictionnaires embarqués)
```

---

## 2. Ajouter la dépendance Rust

Dans le `Cargo.toml` du crate Tauri de votre application (typiquement
`src-tauri/Cargo.toml`) :

```toml
[dependencies]
hugo-tauri = { git = "https://github.com/theophiledonato/hugo" }
# ou, en monorepo / chemin local :
# hugo-tauri = { path = "../../hugo/crates/hugo-tauri" }
```

`hugo-tauri` tire `hugo-core` transitivement : vous n'avez pas besoin de
l'ajouter séparément, sauf pour utiliser directement le `Checker` côté Rust.

---

## 3. Enregistrer le plugin

Dans la construction de votre application Tauri (souvent `src-tauri/src/lib.rs`
ou `main.rs`) :

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(hugo_tauri::init()) // ← enregistre la commande check_text
        // … vos autres plugins / handlers …
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Tauri");
}
```

`hugo_tauri::init()` :

- enregistre la commande `check_text` ;
- construit un `Checker` et le place dans l'état partagé via `app.manage(...)`
  pendant la phase `setup` du plugin.

---

## 4. Autoriser la commande (ACL / capabilities)

Tauri v2 refuse par défaut tout appel de commande de plugin tant qu'une
**permission** n'a pas été accordée dans une *capability*. Ajoutez la permission
par défaut du plugin à votre capability principale (par ex.
`src-tauri/capabilities/default.json`) :

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability par défaut",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "hugo-tauri:allow-check-text"
  ]
}
```

| Permission | Effet |
|---|---|
| `hugo-tauri:default` | Jeu par défaut ; inclut `allow-check-text`. |
| `hugo-tauri:allow-check-text` | Autorise la commande `check_text`. |
| `hugo-tauri:deny-check-text` | Refuse explicitement la commande. |

> **⚠️ Cohérence des noms** — L'espace de noms des permissions (`hugo-tauri:`)
> provient du **nom de crate** du plugin, tandis que la chaîne d'`invoke`
> (`plugin:hugo-tauri|check_text`) utilise le **nom d'exécution** passé à
> `Builder::new(...)`. Les deux valent `hugo-tauri` et **doivent rester
> identiques** : si vous forkez le plugin et renommez l'un, renommez l'autre,
> sinon l'ACL bloquera silencieusement l'appel (`command … not allowed`).

---

## 5. Appeler depuis le front-end

```js
import { invoke } from "@tauri-apps/api/core";

const suggestions = await invoke("plugin:hugo-tauri|check_text", {
  text: "il il va a Paris",
});
console.log(suggestions);
// [
//   { start: 0,  end: 5,  message: "Répétition du mot « il ».",            replacements: ["il"], ruleId: "duplicate_word" },
//   { start: 9,  end: 10, message: "Confusion d'homophones : …",          replacements: ["à"],  ruleId: "homophone" }
// ]
```

### Types TypeScript

Le plugin ne génère pas (encore) de `.d.ts`. En attendant, déclarez le type de
retour vous-même — il reflète `JsSuggestion` côté Rust :

```ts
export interface HugoSuggestion {
  /** Offset d'octet de début dans le texte source (inclus). */
  start: number;
  /** Offset d'octet de fin (exclu). */
  end: number;
  /** Message explicatif, en français. */
  message: string;
  /** Corrections proposées, triées de la plus à la moins pertinente. */
  replacements: string[];
  /** Identifiant stable de la règle ayant produit la suggestion. */
  ruleId: string;
}

import { invoke } from "@tauri-apps/api/core";

export function checkText(text: string): Promise<HugoSuggestion[]> {
  return invoke("plugin:hugo-tauri|check_text", { text });
}
```

> **Offsets en octets** — `start` / `end` sont des offsets d'**octets** UTF-8,
> pas des index de caractères JavaScript. Pour découper la chaîne côté JS sans
> surprise sur les accents/emoji, travaillez sur l'encodage UTF-8 (par ex. via
> `TextEncoder`/`TextDecoder`) plutôt que sur `String.prototype.slice`.

### Exemple : appliquer une correction

```ts
function applyFirst(text: string, s: HugoSuggestion): string {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  const bytes = enc.encode(text);
  const before = dec.decode(bytes.slice(0, s.start));
  const after = dec.decode(bytes.slice(s.end));
  return before + (s.replacements[0] ?? "") + after;
}
```

### Bonne pratique : débattre la frappe (debounce)

`check_text` est rapide (bien en deçà de 5 ms par phrase), mais inutile de
l'appeler à chaque touche. Débattez l'appel pour limiter les allers-retours :

```ts
let timer: number | undefined;
function onInput(text: string, render: (s: HugoSuggestion[]) => void) {
  clearTimeout(timer);
  timer = setTimeout(async () => render(await checkText(text)), 200);
}
```

---

## 6. Identifiants de règles (`ruleId`)

| `ruleId` | Catégorie | Exemple |
|---|---|---|
| `spelling` | Orthographe (mot inconnu) | « maisn » → « maison » |
| `duplicate_word` | Mot répété | « il il » → « il » |
| `capitalization_after_period` | Majuscule après ponctuation | « … . il » → « Il » |
| `determiner_noun_agreement` | Accord déterminant–nom | « un table » → « une » |
| `subject_verb_agreement` | Accord sujet–verbe | « les chats mange » → « mangent » |
| `attribute_adjective_agreement` | Accord de l'attribut | « elle est content » → « contente » |
| `homophone` | Homophones grammaticaux | « il va a Paris » → « à » |

Utilisez `ruleId` pour styliser différemment les soulignements (rouge pour
`spelling`, bleu pour la grammaire, etc.) ou pour filtrer les catégories que
vous souhaitez afficher.

---

## 7. Sans plugin : appeler le cœur directement

Si vous préférez déclarer vos propres commandes Tauri (ou si vous voulez plus
de contrôle sur la sérialisation), ajoutez `hugo-core` et gérez le `Checker`
vous-même :

```rust
use hugo_core::Checker;
use tauri::Manager;

#[tauri::command]
fn check(text: String, checker: tauri::State<'_, Checker>) -> Vec<(usize, usize, String)> {
    checker
        .check(&text)
        .into_iter()
        .map(|s| (s.span.start, s.span.end, s.replacements.join(" / ")))
        .collect()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Checker::new()); // une seule construction
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![check])
        .run(tauri::generate_context!())
        .unwrap();
}
```

`Checker` est `Send + Sync` et immuable après construction : il se partage sans
verrou entre fenêtres et threads.

---

## 8. Notes de performance et d'empreinte

- **Local et hors-ligne** : aucun appel réseau ; tout est embarqué.
- **Chargement paresseux** : le lexique morphologique et les index de
  conjugaison/déclinaison se construisent à la première utilisation. Le premier
  appel à `check_text` est donc un peu plus lent ; les suivants sont chauds.
  Pour masquer cette latence, déclenchez un appel « à blanc » au démarrage de la
  fenêtre.
- **Empreinte** : les assets (FST morphologique + DAWG orthographique) sont liés
  dans le binaire de l'application — voir la feuille de route pour les cibles de
  taille.

---

## 9. Dépannage

| Symptôme | Cause probable |
|---|---|
| `command check_text not allowed` (ou silence) | Permission absente de la capability, ou désaccord de noms (voir §4). |
| `plugin hugo-tauri not found` | `.plugin(hugo_tauri::init())` non enregistré, ou nom d'`invoke` erroné. |
| Offsets décalés sur les accents | `start`/`end` sont en **octets** UTF-8 (voir §5). |
| Premier appel lent | Chargement paresseux des index ; faites une chauffe au démarrage. |

---

## Voir aussi

- [`ROADMAP.md`](../ROADMAP.md) — phases du projet (intégrations, génération de
  types TypeScript, etc.).
- [`README.md`](../README.md) — vue d'ensemble et utilisation de `hugo-core`.

[`Checker`]: ../crates/hugo-core/src/lib.rs
