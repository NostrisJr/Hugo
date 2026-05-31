# Avis et attributions

Le code source de Hugo est distribué sous double licence MIT / Apache-2.0
(voir `LICENSE-MIT` et `LICENSE-APACHE`).

## Données embarquées

### Dictionnaire orthographique français — Dicollecte

`crates/hugo-core/assets/dicollecte.fst` est un FST dérivé (par développement
des affixes) du dictionnaire orthographique français « classique » de
**Dicollecte**, par Olivier R.

- Licence : **Mozilla Public License 2.0** (MPL 2.0)
- Source : <https://grammalecte.net/> — distribué notamment via
  <https://github.com/LibreOffice/dictionaries/tree/master/fr_FR>
- Outil de génération : `tools/compile-dict` (voir `README.md`)

Conformément à la MPL 2.0, le fichier source (`fr.dic`/`fr.aff`) reste
disponible aux adresses ci-dessus et le présent avis accompagne la forme
dérivée. La MPL 2.0 s'applique fichier par fichier : elle couvre l'asset dérivé,
sans contaminer le reste du code source de Hugo.
