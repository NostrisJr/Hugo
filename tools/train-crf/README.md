# train-crf

Entraîne le **CRF d'étiquetage morphosyntaxique (POS)** de Hugo et produit
l'asset binaire `crates/hugo-core/assets/pos.crf` embarqué par le runtime
([`hugo_core::pos`]). Outil hors livrable (`publish = false`) ; sa seule
dépendance est `hugo-core`, dont il réutilise l'extraction de traits et le
sérialiseur — garantissant que les features d'entraînement et de décodage sont
identiques.

## Données

**Universal Dependencies French-GSD** (CoNLL-U, CC BY-SA 4.0). Récupération :

```sh
base=https://raw.githubusercontent.com/UniversalDependencies/UD_French-GSD/master
for split in train dev test; do
  curl -sSL -o fr-$split.conllu $base/fr_gsd-ud-$split.conllu
done
```

## Usage

```sh
cargo run -p train-crf --release -- \
  fr-train.conllu fr-dev.conllu fr-test.conllu \
  ../../crates/hugo-core/assets/pos.crf
```

Le troisième argument peut être `-` pour omettre l'évaluation sur le test. La
commande affiche la décroissance de la perte, l'exactitude POS dev/test et la
taille du modèle écrit.

## Méthode

- **Étiquettes** : jeu UPOS (17 étiquettes, `hugo_core::pos::Upos`).
- **Traits** : `pos::observation_attributes` — forme, préfixes/suffixes 1–4,
  casse/chiffres/ponctuation, **sac des catégories candidates du lexique**, le
  tout sur une fenêtre ±2 ; transitions étiquette→étiquette. Les attributs vus
  moins de `MIN_COUNT` (3) fois sont élagués.
- **Modèle** : CRF à chaîne linéaire. Maximum de vraisemblance pénalisé (L2),
  gradient exact par **forward-backward** en espace log, optimisation **L-BFGS**
  avec recherche linéaire d'Armijo (implémentés à la main, sans dépendance).
- **Sortie** : poids quantifiés en `i16`, attributs indexés par une `fst::Map`,
  via `pos::serialize_model`.

Résultats de référence (paramètres par défaut) : ~97,8 % dev / ~97,6 % test,
modèle ~2,6 Mo.
