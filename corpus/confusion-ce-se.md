# Corpus — confusions « ce »/« se » et « c'est »/« s'est »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique, etc. — aucune phrase
> copiée d'un corpus tiers). Spécification *et* tests de non-régression de la
> **tranche 2** du moteur de confusions (phase 6).
>
> Tests mémo (Projet Voltaire) :
> - **se** (réfléchi) se remplace par « me/te » en changeant de personne
>   (« il **se** lave » → « je **me** lave ») ; **ce** est le démonstratif
>   (« **ce** livre », « **ce** que »).
> - **s'est** = « se + est », **toujours suivi d'un participe passé**
>   (« il **s'est** levé ») ; **c'est** = « cela est » (« **c'est** beau »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## ce → se  (réfléchi écrit en démonstratif)

### sujet (pronom) + ce + verbe conjugué
- FAUX: Le matin, il ce lève toujours très tôt.            → se
- FAUX: Elle ce demande si la pluie va cesser.             → se
- FAUX: Avec le temps, on ce trompe moins souvent.         → se

### négation « ne » intercalée
- FAUX: Malgré le bruit, il ne ce réveille pas.            → se

## se → ce  (démonstratif écrit en réfléchi)

### se + nom / adjectif (étiquette CRF)
- FAUX: Se chien aboie toute la nuit.                        → ce
- FAUX: Il a déjà lu se petit roman policier.               → ce
- FAUX: Range se vieux carton au grenier.                   → ce

### se + relatif que / qui / dont
- FAUX: Je ne comprends pas se que tu racontes.             → ce
- FAUX: Voilà se qui s'est réellement passé.                → ce

## c' → s'  (« c'est » écrit pour le réfléchi « s'est »)

> Sujet de 3ᵉ personne (sans virgule) + « c'est » + **participe passé**.

- FAUX: Ce matin, il c'est trompé de chemin.               → s'
- FAUX: La situation, elle c'est aggravée d'un coup.        → s' (« elle c'est aggravée »)
- FAUX: En tombant, le vase c'est cassé en mille morceaux.  → s'
- FAUX: Le voisin qui c'est trompé de porte est reparti.    → s' (sujet relatif « qui »)
- FAUX: Hier soir, il c'était endormi devant la télé.       → s'

## s' → c'  (« s'est » écrit pour le démonstratif « c'est »)

> « s'est » non suivi d'un participe passé (adverbes sautés) = « c'est ».

- FAUX: S'est vraiment une magnifique journée.              → c'
- FAUX: Pour lui, s'est un beau jour de fête.               → c'
- FAUX: Franchement, s'est dommage de partir si tôt.        → c'

## OK — ne rien signaler (antipatterns)

### « se » réfléchi correct (homographes verbe/nom compris)
- OK: Chaque matin, il se lève à six heures.
- OK: Le dimanche, il se livre à la lecture pendant des heures.

### « ce » démonstratif correct (devant un nom, après un verbe)
- OK: Ce livre est passionnant du début à la fin.
- OK: Je relis souvent ce vieux roman.
- OK: Ce sont mes amis d'enfance.  (« ce sont » : clitique non élidé, hors champ)

### « s'est » correct (réfléchi + participe)
- OK: Après l'effort, il s'est enfin reposé.
- OK: La porte s'est ouverte sans un bruit.
- OK: Elle s'est bien amusée à la fête.  (adverbe « bien » sauté)

### « c'est » correct (démonstratif)
- OK: C'est une très bonne idée.
- OK: Et c'est tant mieux pour tout le monde.
- OK: Je pense que c'est parfaitement juste.
- OK: Lui, c'est différent : il a de l'expérience.  (virgule : pas de sujet rattaché)

## Limites assumées (gaps)

### Homographes nom/verbe après « se » (couverture)
> Comme pour « moulin a poivre » en tranche 1, un homographe nom/verbe que le CRF
> étiquette **verbe** échappe à se→ce : « Se livre est lourd » (« livre » lu comme
> verbe réfléchi) n'est pas corrigé. Les noms non ambigus (« se chien », « se
> roman ») le sont. Rattrapage syntaxique possible plus tard (« se » en tête de
> phrase ne peut être réfléchi).

### Participes hors lexique (couverture)
> La direction c'→s' exige un participe passé reconnu : un participe absent du
> Lefff ou sans trait de genre/nombre (« plaint »…) n'est pas détecté.

### ces / ses — ambiguïté **structurelle** (hors de portée)
> « ces » (démonstratif) et « ses » (possessif) sont deux déterminants également
> valides devant un nom ; aucune règle grammaticale ne les sépare. On ne signale
> donc rien, dans un sens comme dans l'autre.
- OK: Ces livres sont neufs.
- OK: Ses amis sont venus le voir.
- OK: Il range ses affaires dans ces tiroirs.

### sais / sait — déjà traités par l'accord sujet–verbe
> La confusion sais/sait est un défaut d'accord en personne du verbe *savoir* ;
> elle est captée par `rules::conjugation` (« il sais » → « sait », « je sait » →
> « sais »), pas par le moteur de confusions.
- (cf. corpus d'accord sujet–verbe)
