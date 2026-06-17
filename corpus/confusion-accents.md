# Corpus — confusions de paires accentuées (du/dû, sur/sûr, notre/nôtre…)

> Phrases originales. Phase 9 — [`crate::rules::confusion::accents`].

## du → dû (participe passé de devoir, devant « à »)

- FAUX: c'est du à la chaleur.           → dû
- FAUX: le retard est du à la grève.     → dû
- FAUX: cela est du à un manque de temps. → dû

## du correct (article contracté ou partitif)

- OK: je bois du café.
- OK: il a du talent.
- OK: du pain et du beurre.

## sur → sûr (adjectif, après copule)

- FAUX: il est sur que tu viendras.      → sûr
- FAUX: elle est sur de sa réponse.      → sûr (mais précision limitée)
- FAUX: en es-tu sur ?                   → sûr

## sur correct (préposition)

- OK: il est sur la table.
- OK: la nappe est sur la table.
- OK: mets-le sur l'étagère.

## notre → nôtre / votre → vôtre (pronoms possessifs après article)

- FAUX: c'est le notre.                  → nôtre
- FAUX: c'est la notre.                  → nôtre
- FAUX: c'est le votre.                  → vôtre
- FAUX: les notres sont partis.          → nôtres

## notre / votre corrects (déterminants possessifs)

- OK: notre maison est belle.
- OK: votre voiture est là.
- OK: c'est notre problème.

## mur → mûr (adjectif après copule)

- FAUX: ce fruit est mur.                → mûr
- FAUX: il est mur pour ce poste.        → mûr

## mur correct (nom)

- OK: il repeint le mur.
- OK: un mur en pierre.

## Limites assumées (gaps)

### cru/crû, nu/nû
> Très rares ; signal trop proche du nom/verbe. Non traités.

### sur → sûr devant préposition
> « il est sûr de lui » — après copule + préposition, la détection est incertaine
> (« il est sur le toit » est le cas normal). Limité aux fins de phrase et aux
> contextes clairs.
