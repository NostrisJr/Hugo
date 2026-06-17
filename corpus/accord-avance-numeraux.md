# Corpus — accord des numéraux (vingt, cent)

> Phrases originales. Phase 10 — [`crate::rules::numeraux`].
>
> Règle : « vingt » et « cent » prennent un « s » quand ils sont multiples
> entiers et en fin de numéral.

## vingts → vingt (avant une autre unité)

- FAUX: quatre-vingts-un euros.          → quatre-vingt-un
- FAUX: quatre-vingts-deux ans.          → quatre-vingt-deux
- FAUX: quatre vingts un élèves.         → vingt (forme espace-séparée)

## vingt correct

- OK: quatre-vingts euros.               (80, fin de numéral → s correct)
- OK: quatre-vingt-un.                   (81, avant unité → sans s)
- OK: quatre-vingt-dix.                  (90)

## cent → cents (multiple, fin de numéral)

- FAUX: deux cent euros.                 → cents
- FAUX: cinq cent kilomètres.            → cents
- FAUX: trois cent étudiants.            → cents

## cents → cent (avant une unité)

- FAUX: deux cents trois personnes.      → cent
- FAUX: cinq cents vingt étudiants.      → cent

## cent correct

- OK: deux cents euros.                  (200 — s correct)
- OK: deux cent trois.                   (203 — sans s)
- OK: deux mille euros.                  (mille invariable)

## Limites assumées (gaps)

### Trait d'union systématique (réforme 1990)
> La réforme de 1990 recommande le trait d'union systématique dans les numéraux.
> Signalement partiel uniquement (cas composés).
