# Corpus — élisions et contractions obligatoires

> Phrases originales rédigées pour ce projet. Spécification *et* tests de
> non-régression de la **phase 8** ([`crate::rules::elision`]).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## le / la → l' (article défini devant voyelle ou h muet)

- FAUX: le arbre est grand.              → l'arbre
- FAUX: la école est fermée.             → l'école
- FAUX: le homme est là.                 → l'homme
- FAUX: le hôpital est loin.             → l'hôpital
- FAUX: le enfant dort.                  → l'enfant
- OK: le héros de la nation.             (h aspiré — pas d'élision)
- OK: le hibou chasse la nuit.           (h aspiré)
- OK: le haricot est vert.               (h aspiré)
- OK: l'arbre est grand.                 (déjà élidé)

## de → d' (préposition devant voyelle ou h muet)

- FAUX: un verre de eau fraîche.         → d'eau
- FAUX: il parle de argent.              → d'argent
- FAUX: il vient de Italie.              → d'Italie
- FAUX: il manque de humilité.           → d'humilité
- OK: d'eau fraîche.                     (déjà élidé)
- OK: de hasard.                         (h aspiré)

## je → j' (pronom sujet devant voyelle)

- FAUX: je ai faim.                      → j'ai
- FAUX: je arrive demain.                → j'arrive
- FAUX: je espère le voir.               → j'espère
- OK: j'arrive demain.                   (déjà élidé)

## me / te / se / ne → m' / t' / s' / n'

- FAUX: il me a dit bonjour.             → m'a
- FAUX: il te a appelé hier.             → t'a
- FAUX: il se est levé tôt.              → s'est
- FAUX: je ne ai pas mangé.              → n'ai
- OK: il m'a dit bonjour.               (déjà élidé)
- OK: il s'est levé tôt.                (déjà élidé)

## que → qu' (conjonction / relatif devant voyelle)

- FAUX: il dit que il viendra.           → qu'il
- FAUX: que on sache bien.               → qu'on
- FAUX: il faut que elle parte.          → qu'elle
- OK: qu'il viendra.                     (déjà élidé)

## si + il/ils → s'il/s'ils

- FAUX: si il vient demain.              → s'il
- FAUX: si ils partent ce soir.          → s'ils
- OK: s'il vient demain.                 (déjà élidé)
- OK: si elle vient (pas d'élision devant « elle »)

## ce + voyelle/h-muet → cet (déterminant démonstratif)

- FAUX: ce arbre est beau.               → cet
- FAUX: ce homme est généreux.           → cet
- FAUX: ce enfant est sage.              → cet
- OK: cet arbre est beau.               (déjà correct)
- OK: ce héros est courageux.           (h aspiré — pas de cet)

## lorsque / puisque / quoique → forme élidée

- FAUX: lorsque il est parti.            → lorsqu'il
- FAUX: puisque il insiste.              → puisqu'il
- OK: lorsqu'il est parti.              (déjà élidé)

## Élision fautive devant h aspiré

- FAUX: l'héros de la nation.            → le héros
- FAUX: l'hibou chassait.                → le hibou
- OK: le héros est courageux.           (correct sans élision)
- OK: l'homme est là.                   (h muet — élision correcte)

## Limites assumées (gaps)

### h aspiré non présent dans la liste
> Certains mots à h aspiré peu fréquents peuvent ne pas figurer dans la liste
> [`H_ASPIRES`] et déclencher une fausse élision. La liste est conservatrice
> (seuls les h aspirés courants) : précision > rappel.

### Élision dans des contextes syntaxiques complexes
> `de + NOM_PROPRE_COMMENÇANT_PAR_VOYELLE` : « il vient de Oran » → « d'Oran »
> est correct mais peut créer des ambiguïtés avec des noms communs (déjà traité).
