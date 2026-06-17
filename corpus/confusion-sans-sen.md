# Corpus — confusion « sans » / « s'en »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 4** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **sans** = préposition de privation (« **sans** toi », « **sans** manger ») ;
> - **s'en** = « se » (réfléchi) + « en », **préverbal** (« il **s'en** va »,
>   « elle **s'en** souvient ») ;
> - **c'en** = « ce » + « en », quasi exclusivement « **c'en** est… » (rare).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## sans → s'en (préposition écrite pour le pronominal)

> La préposition « sans » ne s'insère jamais entre un **sujet de 3ᵉ personne**
> (`il/elle/on/ils/elles`, relatif `qui`, ou un nom) et un **verbe conjugué**
> (« ne » sauté) : c'est le pronominal « s'en ».

- FAUX: Il sans va sans dire au revoir.            → s'en
- FAUX: Elle sans souvient encore aujourd'hui.     → s'en
- FAUX: Ils sans vont dès demain matin.            → s'en
- FAUX: Il ne sans souvient pas du tout.           → s'en (« ne » sauté)
- FAUX: Le voleur sans alla aussitôt.              → s'en (sujet nominal)

## s'en → sans (pronominal écrit pour la préposition)

> « s'en » (élision « s' » + « en ») suivi d'une **tête nominale** (nom/adjectif,
> et non un verbe) est impossible : « en » pronom réclame un verbe. On fusionne
> les deux jetons en « sans ». On consulte les **lectures du lexique** pour écarter
> les homographes nom/verbe (« il s'en va », « de s'en aller »).

- FAUX: Il a réussi s'en effort apparent.          → sans
- FAUX: Il agit s'en scrupule aucun.               → sans
- FAUX: Un repas s'en gros sel ni épices.          → sans (adjectif « gros »)

## OK — ne rien signaler (antipatterns)

### « s'en » pronominal correct
- OK: Il s'en va sans rien dire.
- OK: Elle s'en souvient parfaitement.
- OK: Il décide de s'en aller tout de suite.   (« s'en » + infinitif)

### « sans » préposition correcte
- OK: Il part sans toi ce matin.
- OK: Il réussit sans effort.
- OK: Manger sans pain ne lui plaît pas.
- OK: Sans le savoir, il a gagné.

## Limites assumées (gaps)

### c'en (« c'en est trop »)
> « c'en » (= « ce » + « en ») est presque toujours « c'en est… » : trop rare et
> trop proche de « s'en est » pour un signal fiable. Non traité.

### homographes nom/verbe après « s'en »
> Un nom qui est aussi forme verbale (« bruit » = nom et 3ᵉ pers. de « bruire »)
> est écarté par prudence (précision > rappel) : « partie s'en bruit » n'est pas
> corrigé.
