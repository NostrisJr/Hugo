# Corpus — confusion « quel(s)/quelle(s) » / « qu'elle(s) »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 4** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **quel/quelle/quels/quelles** = adjectif interrogatif/exclamatif, **accordé
>   avec un nom** (« **quelle** heure ? », « **quels** beaux jours ! ») ;
> - **qu'elle(s)** = « que/qu' » + pronom « elle(s) », **suivi d'un verbe**
>   (« je crois **qu'elle** vient », « pour **qu'elles** partent »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## qu'elle(s) → quel(le)(s) (élision écrite pour l'adjectif)

> « elle(s) » (pronom) n'est jamais suivi d'un **nom** ni d'un **adjectif** :
> « qu'elle » + groupe nominal est l'adjectif « quel », accordé avec ce nom
> (genre du nom, nombre d'elle/elles). On consulte les **lectures du lexique** —
> et non l'unique tag CRF, souvent leurré par l'erreur (« qu'elle heure » fait
> étiqueter « heure » verbe).

- FAUX: Je me demande qu'elle heure il est.          → quelle
- FAUX: Qu'elle joie de te revoir enfin !            → quelle
- FAUX: Qu'elles belles fleurs dans ce jardin !      → quelles
- FAUX: Qu'elle homme courageux il était !           → quel (nom masculin)
- FAUX: Qu'elles beaux jardins nous avons visités !  → quels (nom masculin pluriel)

## quelle(s) → qu'elle(s) (adjectif écrit pour l'élision)

> « quelle »/« quelles » (formes féminines, homophones de « qu'elle(s) ») suivi
> d'un **verbe conjugué** (clitiques « ne/se/… » sautés) qui n'est **pas** une
> forme d'*être*/*avoir* : l'adjectif interrogatif appelle un nom, pas un verbe.

- FAUX: Je crois quelle vient demain.                → qu'elle
- FAUX: Il faut quelles partent vite.                → qu'elles
- FAUX: Je pense quelle ne vient pas ce soir.        → qu'elle (« ne » sauté)
- FAUX: Je sais quelle se trompe souvent.            → qu'elle (« se » sauté)

## OK — ne rien signaler (antipatterns)

### « quel(le)(s) » adjectif correct
- OK: Quelle heure est-il maintenant ?
- OK: Quelle belle maison vous avez !
- OK: Quel livre lis-tu en ce moment ?

### « qu'elle(s) » élision correcte
- OK: Je crois qu'elle vient ce soir.
- OK: Il faut tout faire pour qu'elles partent à temps.

### idiome « tel quel / telle quelle »
- OK: Il le prend tel quel sans rien changer.
- OK: Elle le rend telle quelle après l'avoir lu.

## Limites assumées (gaps)

### « quelle est / a été… » (être/avoir) — ambiguïté **structurelle**
> « quelle est la réponse » (interrogatif) et « je crois qu'elle est venue »
> (subordonnée) partagent « quelle est » ; « quelle a été ta surprise »
> (interrogatif) et « qu'elle a été malade » (subordonnée) partagent « quelle a
> été ». Aucun signal ne les sépare : on écarte les formes d'*être*/*avoir*.
- OK: Quelle est la réponse à cette question ?   (interrogatif, non corrigé)
- OK: Quelle a été ta plus grande surprise ?     (interrogatif, non corrigé)
- OK: Quelle fut sa joie en nous voyant !        (exclamatif, être sans participe)

### formes masculines « quel/quels » → « qu'il(s) »
> « quel vient » → « qu'il vient » relève d'une **autre famille** (qu'il(s)),
> hors de cette tranche, et n'est pas traité ici.
