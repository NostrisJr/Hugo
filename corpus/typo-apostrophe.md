# Corpus — apostrophe typographique

> Phrases **originales** rédigées pour ce projet (inspiration : conventions
> typographiques françaises usuelles ; aucune phrase copiée d'un corpus tiers).
> Spécification *et* tests de non-régression de la règle `typo_apostrophe`
> (phase 7).
>
> Règle : l'apostrophe droite `'` (U+0027) devient typographique `'` (U+2019)
> en position d'élision (`l'`, `qu'`, `j'`…) et dans les apostrophes internes
> entourées de lettres (`aujourd'hui`).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.

## ' → '  (apostrophe droite saisie)

### Élisions
- FAUX: l'homme est arrivé tôt.            → l'
- FAUX: qu'il vienne demain.               → qu'
- FAUX: jusqu'ici tout va bien.            → jusqu'

### Apostrophes internes
- FAUX: aujourd'hui il fait beau.          → aujourd'hui
- FAUX: un vieux prud'homme.               → prud'homme
- FAUX: une presqu'île déserte.            → presqu'île

## OK  (ne rien signaler)

- OK: l'homme est arrivé tôt.              (déjà U+2019)
- OK: aujourd'hui il fait beau.            (déjà U+2019)
- OK: 'citation entre guillemets simples'  (apostrophe non bordée de lettres)

## Gaps assumés

- Détection des apostrophes en **code** ou citation littérale : non distinguées
  d'une élision si elles sont bordées de lettres (`it's` → `it's`). Rare en
  texte français ; précision jugée suffisante.
