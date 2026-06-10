# Corpus — confusion « leur » / « leurs »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression d'une famille de
> la **tranche 3** du moteur de confusions (phase 6).
>
> Mémo (Projet Voltaire) :
> - **leur** devant un **verbe** = pronom personnel (« à eux »), **invariable**
>   (« je **leur** parle ») ;
> - **leur** / **leurs** devant un **nom** = déterminant possessif, qui
>   **s'accorde** avec le nom (« **leur** livre », « **leurs** livres »).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## leur → leurs (possessif singulier écrit devant un nom pluriel)

- FAUX: Ils ont rangé leur affaires avant de partir.         → leurs
- FAUX: Leur livres sont restés sur la table.                 → leurs
- FAUX: Ils aiment leur grandes maisons de campagne.         → leurs (adjectif sauté)

## leurs → leur

### pronom personnel (devant un verbe, invariable)
- FAUX: Je leurs parle tous les jours.                        → leur
- FAUX: Il leurs donne un coup de main.                       → leur

### possessif pluriel écrit devant un nom singulier
- FAUX: Leurs maison est trop petite pour eux.                → leur
- FAUX: Ils adorent leurs enfant unique.                      → leur

## OK — ne rien signaler (antipatterns)

- OK: Leur livre préféré est épuisé.            (possessif singulier correct)
- OK: Leurs livres sont neufs.                  (possessif pluriel correct)
- OK: Je leur parle souvent.                    (pronom invariable correct)
- OK: Il leur donne un cadeau.                  (pronom invariable correct)
- OK: Leurs grandes maisons sont vendues.       (possessif pluriel + adjectif)
- OK: Leur grande maison est vendue.            (possessif singulier + adjectif)

## Limites assumées (gaps)

### noms non marqués / invariables en nombre au lexique
> « prix », « livre »… ne portent pas toujours de trait de nombre explicite (ou
> sont invariables) : on ne peut alors trancher le possessif singulier/pluriel
> sans risque de faux positif (« leurs prix » peut être un pluriel correct). On
> s'abstient.
- OK: Leurs prix sont élevés.   (« prix » non tranché — gap assumé)
