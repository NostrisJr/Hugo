# Corpus — confusion « quand » / « quant »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 4** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **quand** = conjonction/adverbe de **temps** (« **quand** il pleut »,
>   « **quand** pars-tu ? ») ; remplaçable par « lorsque ».
> - **quant** n'existe **que** dans « **quant à / au / aux** » (« **quant à**
>   moi ») ; remplaçable par « en ce qui concerne ».
> - **qu'en** = « que/qu' » + « en » (« **qu'en** penses-tu ? »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## quand → quant (« quand à » écrit pour la locution)

> « quand » immédiatement suivi de « à »/« au »/« aux » est « quant » : la
> conjonction de temps n'introduit jamais ce complément (signal **séparable**,
> « quand à/au/aux » étant toujours fautif).

- FAUX: Quand à moi, je préfère rester.            → quant
- FAUX: Quand au reste, on verra plus tard.        → quant
- FAUX: Quand aux enfants, ils dorment déjà.       → quant

## quant → quand (locution écrite pour la conjonction)

> « quant » qui **n'est pas** suivi de « à »/« au »/« aux » est mal orthographié :
> « quant à » est son seul emploi licite. On corrige vers « quand ».

- FAUX: Quant il pleut, je reste à la maison.      → quand
- FAUX: Je rentrerai quant tu partiras.            → quand
- FAUX: Quant viendras-tu nous voir ?              → quand

## OK — ne rien signaler (antipatterns)

### « quand » conjonction/adverbe correct
- OK: Quand il pleut, je lis un livre.
- OK: Quand pars-tu en vacances ?
- OK: Dis-moi quand tu seras prêt.

### « quant à » locution correcte
- OK: Quant à moi, je préfère le thé.
- OK: Quant au dîner, tout est prêt.
- OK: Quant aux résultats, ils sont bons.

### virgule intercalée (pas de rattachement « quand » → « à »)
- OK: Quand, à Paris, il arrivera, préviens-moi.

## Limites assumées (gaps)

### qu'en (« qu'en penses-tu » ↔ « quand penses-tu »)
> « qu'en » et « quand » sont tous deux suivis d'un verbe : aucun signal ne les
> sépare. On ne traite pas cette confusion, et « quant » fautif est corrigé en
> « quand » (le cas le plus fréquent), jamais en « qu'en ».
