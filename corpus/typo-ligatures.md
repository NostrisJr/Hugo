# Corpus — ligatures œ

> Phrases **originales** rédigées pour ce projet. Spécification *et* tests de
> non-régression de la règle `typo_ligature` (phase 7).
>
> `oe` → `œ` dans une **liste curée** de mots où la ligature est obligatoire.
> L'approche par liste fermée écarte par construction les mots où `oe` n'est
> **pas** une ligature.
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.

## oe → œ

- FAUX: mon coeur bat fort.        → cœur
- FAUX: un oeuf à la coque.        → œuf
- FAUX: ma soeur aînée.            → sœur
- FAUX: un voeu pieux.             → vœu
- FAUX: un noeud marin.            → nœud
- FAUX: une belle manoeuvre.       → manœuvre
- FAUX: Coeur (en tête de phrase). → Cœur (casse respectée)
- FAUX: COEUR (tout en capitales). → CŒUR

## OK  (ne rien signaler — « oe » n'est pas une ligature)

- OK: il faut coexister.           (coexister)
- OK: la moelle épinière.          (moelle)
- OK: le coefficient directeur.    (coefficient)
- OK: un goéland sur la jetée.     (goéland — « oé », pas « oe »)
- OK: une poêle à frire.           (poêle — « oê »)

## Gaps assumés

- Mots **composés** à trait d'union/apostrophe (`chef-d'oeuvre`) : non traités
  (tokenisés comme un seul mot non listé).
- Ligature `æ` (`et caetera`, `ex aequo`) : non traitée (corpus distinct
  ultérieur).
- La liste curée est volontairement **conservatrice** ; à enrichir au fil de
  l'eau plutôt que d'inférer par motif (risque de faux positifs).
