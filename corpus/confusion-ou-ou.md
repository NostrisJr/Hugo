# Corpus — confusion « ou » / « où »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 3** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) : **ou** (sans accent) = conjonction d'alternative
> (remplaçable par « ou bien ») ; **où** (avec accent) = lieu ou temps
> (« le jour **où** », « là **où** »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## ou → où (alternative écrite pour le relatif de lieu/temps)

> Signal séparable retenu : **antécédent de lieu/temps** (nom d'une liste fermée,
> ou adverbe `là`/`ici`/`partout`) + « ou » + **pronom sujet** → proposition
> relative → « où ».

- FAUX: Je me souviens du jour ou je suis arrivé ici.        → où
- FAUX: C'est le pays ou nous avons grandi.                  → où
- FAUX: Voici le moment ou tu dois choisir.                  → où
- FAUX: La ville ou il habite est très calme.                → où
- FAUX: Là ou je vis, il pleut souvent.                      → où
- FAUX: Partout ou il passe, on le remarque.                 → où

## OK — ne rien signaler (antipatterns)

### « ou » alternative correcte (suivi d'un déterminant / d'un autre groupe)
- OK: Tu préfères le jour ou la nuit ?
- OK: Veux-tu du thé ou du café ?
- OK: C'est vrai ou faux ?
- OK: Il vient ou il reste, à lui de voir.   (coordination de propositions)

### « où » déjà correct
- OK: Le jour où je suis né, il neigeait.
- OK: Le pays où les gens vivent heureux existe-t-il ?

## Limites assumées (gaps)

### où → ou : ambiguïté **structurelle** (hors de portée)
> « où » relatif/interrogatif peut être suivi d'un déterminant comme d'un sujet
> (« le pays où les gens vivent »), exactement comme l'alternative « ou ». Aucun
> signal ne sépare « le thé où le café » (fautif) de « le lieu où le drame a eu
> lieu » (correct). On ne traite donc pas cette direction.
- OK: Tu préfères le thé où le café ?   (fautif mais non corrigé — gap assumé)

### Antécédent hors liste / éloigné (couverture)
> L'antécédent de lieu/temps est une **liste fermée** (gage de précision) et doit
> être **immédiatement** avant « ou ». « le jour férié ou je me repose »
> (adjectif intercalé) ou un nom de lieu absent de la liste échappent à la règle.
