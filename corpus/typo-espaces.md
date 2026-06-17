# Corpus — espaces (surnuméraires, manquants, insécables)

> Phrases **originales** rédigées pour ce projet. Spécification *et* tests de
> non-régression des règles `typo_space` et `typo_nbsp` (phase 7).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.
> (Les espaces insécables sont notées `[fine]` = U+202F, `[insec]` = U+00A0.)

## Espaces surnuméraires / manquants — `typo_space`

- FAUX: le  chat dort.            → une seule espace
- FAUX: le chat , lui.            → pas d'espace avant la virgule
- FAUX: fin ) du texte.          → pas d'espace avant la parenthèse fermante
- FAUX: ( a) entre parenthèses.  → pas d'espace après la parenthèse ouvrante
- FAUX: rouge,vert,bleu.         → espace manquante après la virgule
- OK:   il a payé 3,14 euros.    (virgule décimale, encadrée de chiffres)
- OK:   le chat dort.            (espacement correct)
- OK:   ligne\n  suite           (saut de ligne + indentation : non touché)

## Espaces insécables — `typo_nbsp`

On **convertit** une espace ordinaire existante ; on n'**insère** pas une
espace absente (pour épargner URL, heures, code).

- FAUX: vraiment ?               → [fine] avant « ? »
- FAUX: attention !              → [fine] avant « ! »
- FAUX: ceci ; cela              → [fine] avant « ; »
- FAUX: voici : ceci             → [insec] avant « : »
- FAUX: il dit « oui »           → [insec] après « « » et avant « » »
- OK:   http://site              (pas d'espace devant « : » → rien)
- OK:   rendez-vous à 12:30      (heure, pas d'espace → rien)
- OK:   vraiment[fine]?          (déjà une fine insécable)

## Gaps assumés

- **Insertion** d'une insécable manquante avant `: ; ! ?` (sans espace du tout) :
  non traitée — risque de faux positifs (URL, code `a:b`, `etc.`).
- Espace manquante après `. ! ? :` : non traitée (homographes URL/abréviations).
