# Corpus — nombres (abréviations ordinales)

> Phrases **originales** rédigées pour ce projet. Spécification *et* tests de
> non-régression de la règle `typo_ordinal` (phase 7).
>
> Abréviations ordinales mal formées accolées à un nombre → forme correcte.
> Seul le **suffixe** est réécrit ; il doit suivre immédiatement un nombre.
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.

## Ordinaux mal formés

- FAUX: la 1ère fois.              → 1re
- FAUX: le 2ème jour.             → 2e
- FAUX: le 2ième jour.           → 2e
- FAUX: les 3èmes places.        → 3es
- FAUX: la 1ière édition.        → 1re

## OK  (abréviations correctes ou non ordinales)

- OK: le 1er jour.               (1er correct)
- OK: la 1re fois.              (1re correct)
- OK: la 2e place, les 2es.    (2e/2es corrects)
- OK: la 2nde guerre.          (2nde admis pour « seconde »)
- OK: une ère nouvelle.        (« ère » mot autonome, sans nombre devant)

## Gaps assumés

- Séparateur de **milliers** (`10000` → `10 000` insécable) : non traité
  (ambiguïté avec les identifiants, codes postaux, années).
- Nombres **romains**, exposants `%`/`°`/`€` : non traités (passe ultérieure).
- `2nd`/`2nde` : considérés **admis** (abréviation de « second(e) »), non
  corrigés en `2d`.
