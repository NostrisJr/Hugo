# Corpus — confusion « peu » / « peut » / « peux »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 3** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **peu** = adverbe de quantité (remplaçable par « beaucoup » : « un **peu** »,
>   « **peu** de gens ») ;
> - **peut** / **peux** = verbe *pouvoir* (remplaçable par « pouvait »/« pouvais » :
>   « il **peut** », « je/tu **peux** »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## peu → peut/peux (adverbe écrit pour le verbe)

> Pronom sujet (`je/tu/il/elle/on`) + « peu » + **infinitif** (négation sautée).
> La forme suit la personne : `je/tu` → « peux », `il/elle/on` → « peut ».

- FAUX: Il peu marcher pendant des heures.                    → peut
- FAUX: Elle peu venir dès demain matin.                      → peut
- FAUX: On peu partir quand on veut.                          → peut
- FAUX: Je peu venir si tu insistes.                          → peux
- FAUX: Tu peu partir maintenant.                             → peux
- FAUX: Il ne peu pas venir ce soir.                          → peut (négation sautée)

## peut/peux → peu (verbe écrit pour l'adverbe)

> Précédé d'un quantifieur (`un`, `très`, `trop`, `si`, `assez`…), ou d'un *avoir*
> immédiatement suivi de « de ».

- FAUX: Un peut de sel suffit largement.                      → peu
- FAUX: Il y a très peut de monde aujourd'hui.               → peu
- FAUX: Trop peut de gens le savent.                          → peu
- FAUX: Il a peut de temps devant lui.                        → peu (avoir + … + « de »)
- FAUX: Elle a peut de patience ce matin.                     → peu

## OK — ne rien signaler (antipatterns)

- OK: Il peut marcher pendant des heures.        (verbe correct)
- OK: Je peux venir si tu insistes.              (verbe correct)
- OK: Il y a peu de monde aujourd'hui.           (adverbe correct)
- OK: Un peu de sel suffit.                       (adverbe correct)
- OK: Il a peu de temps.                          (adverbe correct)
- OK: Il peut de nouveau jouer du piano.          (« peut » + « de nouveau »)
- OK: Tout peut arriver.                          (« tout » écarté de la liste)

## Limites assumées (gaps)

### confusion de **personne** peux ↔ peut
> « je peut », « il peux » sont des défauts d'**accord en personne** du verbe
> *pouvoir*, captés par l'accord sujet–verbe (`rules::conjugation`), pas par le
> moteur de confusions — comme sais/sait en tranche 2.
- (cf. corpus d'accord sujet–verbe)
