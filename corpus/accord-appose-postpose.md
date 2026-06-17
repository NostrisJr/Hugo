# Accord de l'apposition détachée avec sujet postposé

Règle `detached_appositive` — Phase 12.2

---

## Principe

Un adjectif ou participe passé encadré par des virgules (ou en tête de phrase
avant une virgule) s'accorde avec le **sujet de la phrase**, même si ce sujet
est **postposé** (inversion stylistique : sujet après le verbe).

Motifs couverts :

1. `[,] ADJ/PP … [,] VERBE SUJET` — apposé entre deux virgules
2. `ADJ/PP … [,] VERBE SUJET` — apposé en tête de phrase

---

## Cas fautifs → déclenchement attendu

### Motif 1 — apposé entre deux virgules

| Phrase fautive | Correction | Explication |
|---|---|---|
| `Au bord du lac, endormi sous les arbres, patientaient les soldats.` | `endormis` | soldats = Masc Pl |
| `Dans la montagne, blessé par la chute, parvinrent les alpinistes.` | `blessés` | alpinistes = Masc Pl |
| `Sur la place, épuisé par la chaleur, s'assirent les pèlerins.` | `épuisés` | pèlerins = Masc Pl |
| `Au sommet, heureuse d'arriver, couraient les randonneurs.` | `heureux` | randonneurs = Masc Pl |
| `Sous la pluie, tranquille malgré l'orage, attendaient les filles.` | `tranquilles` | filles = Fém Pl |

### Motif 2 — apposé en tête de phrase

| Phrase fautive | Correction | Explication |
|---|---|---|
| `Épuisé par le voyage, s'allongea la voyageuse.` | `Épuisée` | voyageuse = Fém Sg |
| `Fatigué par la route, partirent les soldates.` | `Fatigués` | soldates = Masc Pl (masculin neutre) |
| `Heureuse de partir, coururent les soldats.` | `Heureux` | soldats = Masc Pl |
| `Blessé à l'épaule, continua la joueuse.` | `Blessée` | joueuse = Fém Sg |

---

## Cas corrects → silence attendu

| Phrase correcte | Explication |
|---|---|
| `Au bord du lac, endormis sous les arbres, patientaient les soldats.` | déjà accordé Masc Pl |
| `Épuisée par le voyage, s'allongea la voyageuse.` | déjà accordé Fém Sg |
| `Épuisés par la route, s'arrêtèrent les soldats.` | déjà accordé Masc Pl |
| `Tranquilles malgré l'orage, attendaient les filles.` | déjà accordé Fém Pl |

---

## Non-déclenchement attendu

| Phrase | Raison de non-déclenchement |
|---|---|
| `Le soldat, épuisé par la route, s'arrêta.` | sujet « soldat » avant le verbe (ordre direct) |
| `Épuisé, il s'arrêta.` | sujet « il » avant le verbe (non inversé) |
| `Pierre, général de l'armée, repartit le matin.` | « général » est un nom, pas un adj (POS = NOUN) |
| `La ville, belle et calme, dormait.` | sujet « ville » avant le verbe (ordre direct) |
| `Épuisé par la route, fatigué.` | pas de verbe fini après la virgule |
| `Elle est contente, heureuse même.` | pas de sujet postposé nominal après la virgule |

---

## Gaps assumés

- **Sujet épicène ambigu** (`les enfants`, `les élèves`) : si le genre est
  indéterminé **et** contradictoire dans le lexique, la règle ne déclenche pas
  (précision > rappel). Pour les épicènes sans genre enregistré, le masculin
  grammatical est utilisé par convention.
- **Sujet pronominal postposé** (`dit-il`) : les inversions verbales avec pronom
  clitique sont traitées par `rules::trait_union`, pas par cette règle.
- **Apposé en position médiane** (ni entre virgules ni en tête) : hors du motif,
  laissé à la règle `attribute`.
- **Accord du participe passé avec auxiliaire** : délégué à `rules::attribute`
  et `rules::past_participle`.
