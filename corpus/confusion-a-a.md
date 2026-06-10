# Corpus — confusion « a » / « à »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique, etc. — aucune phrase
> copiée d'un corpus tiers). Sert de spécification *et* de tests de non-régression
> pour la tranche 1 du moteur de règles (phase 6).
>
> Test mémo (Projet Voltaire) : « a » est le verbe *avoir* si on peut le remplacer
> par « avait » ; sinon c'est la préposition « à ».
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## a → à  (« a » saisi, « à » attendu)

### NOM + a + NOM (complément du nom, matière, usage)
- FAUX: Range les tasses a café sur l'étagère du haut.        → à
- FAUX: Elle a acheté un moulin a poivre en bois d'olivier.   → à (2e « a »)
- FAUX: Mon grand-père collectionne les fers a repasser.      → à
- FAUX: On a servi une tarte a la rhubarbe en dessert.        → à
- FAUX: Il porte toujours ses bottes a clous en montagne.     → à

### NOM/locution a NOM-propre (destination, lieu)
- FAUX: Le train repart a Bordeaux dans dix minutes.          → à
- FAUX: Nous montons a pied a Montmartre demain matin.        → à (les deux « a »)

### a + infinitif régi par le mot précédent
- FAUX: Le ciel se couvre, il commence a pleuvoir.            → à
- FAUX: Cette énigme est difficile a résoudre.                → à
- FAUX: Tu n'as rien a craindre de lui.                       → à

### Locutions prépositionnelles
- FAUX: Le marché est fermé a cause de la tempête.            → à
- FAUX: A partir de lundi, l'atelier ouvre plus tôt.          → À
- FAUX: Assis face a l'océan, il ne disait rien.              → à
- FAUX: Petit a petit, le jardin a repris vie.                → à (1er « a »)

## à → a  (« à » saisi, « a » attendu)

### Clitique + à (le « à » occupe une place de verbe)
- FAUX: Il y à trop de bruit dans cette salle.                → a
- FAUX: Personne ne l'à prévenue du changement.               → a
- FAUX: On nous à promis une réponse avant vendredi.          → a
- FAUX: Qui à laissé la porte ouverte ?                       → a

### Sujet + à + participe/complément
- FAUX: La situation à beaucoup changé depuis hier.           → a
- FAUX: Mon voisin à enfin réparé sa clôture.                 → a

## OK — ne rien signaler (antipatterns)

### Idiomes « avoir + nom » (le nom suit, mais « a » = avoir)
- OK: Le chiot a faim depuis ce matin.
- OK: Tu as raison de te méfier.
- OK: Cette demande a peu de chances d'aboutir.
- OK: Le spectacle a lieu samedi soir.
- OK: Elle a besoin d'un peu de repos.
- OK: Ce témoignage a force de preuve.

### « il y a », « a » verbe ordinaire
- OK: Il y a du monde sur la place ce dimanche.
- OK: Le chat a renversé son bol d'eau.
- OK: Personne n'a rien remarqué.

### « à » préposition déjà correcte
- OK: Nous partons à Lyon en début d'après-midi.
- OK: C'est une vraie machine à laver le linge.
- OK: Il faut penser à tout, jusqu'au moindre détail.

### Minimal pairs (les deux graphies dans la même phrase)
- MIXTE: Celui qui marche a pied a souvent plus de mérite.
  → 1er « a » : à (marche **à** pied) ; 2e « a » : OK (il **a** du mérite)
- MIXTE: Elle pense a tout mais a peu de temps.
  → 1er « a » : à (pense **à** tout) ; 2e « a » : OK (elle **a** peu de temps)

## Liste d'idiomes « avoir + nom » (reconstituée, à compléter)

> Noms qui suivent légitimement « a » (verbe avoir) et bloquent la règle
> NOM+a+NOM. À enrichir au fil du corpus, **sans** copier de liste tierce.

accès, affaire, besoin, charge, confiance, conscience, cours, crainte, droit,
envie, faim, force, garde, hâte, honte, horreur, intérêt, lieu, mal, peine,
peur, pitié, raison, rapport, recours, soif, sommeil, tendance, tort, trait,
vocation
