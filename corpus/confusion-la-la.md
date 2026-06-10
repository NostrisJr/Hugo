# Corpus — confusion « la » / « là » / « l'a »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 3** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **la** = article ou pronom féminin (« **la** table », « il **la** voit ») ;
> - **là** = adverbe de lieu (« reste **là** », « ce jour-**là** ») ;
> - **l'a** = « l' » (pronom élidé) + « a » (*avoir*) : « il **l'a** vu ».
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## là → la (adverbe écrit pour l'article)

> « là » devant un **nom** (non explicitement masculin — « là » n'est homophone
> que de l'article **féminin** « la ») occupe la place de l'article.

- FAUX: Là maison au bout du chemin est à vendre.            → la
- FAUX: Là montagne se dresse au loin.                        → la
- FAUX: Regarde là voiture qui passe.                         → la

## la → l'a (article/pronom écrit pour « l' » + auxiliaire)

> Sujet de 3ᵉ personne (`il/elle/on`, relatif `qui`, ou un nom) + « la » +
> **participe passé** (adverbes sautés) → pronom élidé + auxiliaire « l'a ».

- FAUX: Il la mangé sans rien laisser.                        → l'a
- FAUX: Elle la vu partir hier soir.                          → l'a
- FAUX: On la cassé en le déplaçant.                          → l'a
- FAUX: Le chat la attrapé d'un bond.                         → l'a
- FAUX: Il la déjà oublié.                                    → l'a (adverbe « déjà » sauté)

## OK — ne rien signaler (antipatterns)

### article / pronom « la » corrects
- OK: La maison est belle au printemps.
- OK: Il la voit chaque jour à la même heure.   (pronom objet + verbe conjugué)
- OK: Elle la regarde sans dire un mot.
- OK: Je la mange tout de suite.                 (1ʳᵉ pers. : « je l'ai », pas « l'a »)

### « là » adverbe correct, « l'a » déjà correct
- OK: Reste là encore un moment.
- OK: Il l'a vu hier soir.

## Limites assumées (gaps)

### la → là **adverbial** (homographie article / pronom objet)
> « viens la » → « viens là », « ce jour la » → « ce jour-là » : « la » y est
> souvent ambigu avec le pronom objet (« regarde-la »). On ne traite pas cette
> direction.
- OK: Viens la, on t'attend.   (fautif mais non corrigé — gap assumé)

### là → la devant un nom **masculin** (correction « le »)
> « là garçon » réclamerait « le garçon », non « la » : ce n'est pas une confusion
> là/la (homophones) mais un article manquant. Hors champ.
