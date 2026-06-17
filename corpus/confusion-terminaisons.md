# Corpus — confusion des terminaisons « -er » / « -é » / « -ez »

> Phrases **originales** rédigées pour ce projet (inspiration : patrons décrits
> par Projet Voltaire, Banque de dépannage linguistique — aucune phrase copiée
> d'un corpus tiers). Spécification *et* tests de non-régression de la
> **tranche 5** du moteur de confusions (phase 6).
>
> Pour un verbe du **1ᵉʳ groupe** (infinitif en « -er »), l'infinitif
> (« manger »), le participe passé masculin singulier (« mangé ») et la 2ᵉ
> personne du pluriel (« mangez ») se **prononcent à l'identique** (/e/). C'est la
> faute de français la plus répandue.
>
> Mémo (Projet Voltaire) : remplacer le verbe suspect par un verbe du **3ᵉ
> groupe** —
> - si « **vendre** » convient, c'est l'**infinitif** (« -er ») : « il commence à
>   *vendre* » → « il commence à manger » ;
> - si « **vendu** » convient, c'est le **participe passé** (« -é ») : « il a
>   *vendu* » → « il a mangé ».
>
> On tranche par le **gouverneur** du verbe (à gauche, clitiques/adverbes/« ne »
> sautés). Les noms/adjectifs homographes (« fer », « clé », « nez », « côté ») et
> les verbes des autres groupes (« partir »/« parti », « faire »/« fait ») sont
> écartés par les lectures du lexique (lemme en « -er » exigé).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne doit rien signaler.

## … → -é (auxiliaire « avoir »/« être » + participe passé)

> Un **auxiliaire conjugué** gouverne un participe passé, jamais un infinitif.
> Avec « être », le participe **s'accorde** avec le sujet.

- FAUX: Il a manger une pomme entière.            → mangé
- FAUX: J'ai bien manger ce soir.                  → mangé (adverbe sauté)
- FAUX: Elle a chanter toute la nuit.              → chanté
- FAUX: Il ne l'a pas manger du tout.              → mangé (« ne … pas » + clitique sautés)
- FAUX: Elle est tomber dans l'escalier.           → tombée (accord, sujet « elle »)
- FAUX: Ils sont arriver hier soir.                → arrivés (accord, sujet « ils »)

## … → -er (préposition / semi-auxiliaire + infinitif)

> Une **préposition** infinitive (`à`, `de`, `pour`, `sans`) ou un
> **semi-auxiliaire** conjugué (`aller`, `vouloir`, `devoir`, `aimer`…) gouverne
> un infinitif.

- FAUX: Il commence à mangé son dessert.           → manger
- FAUX: Il décide de mangé maintenant.             → manger
- FAUX: C'est facile à réalisé.                     → réaliser
- FAUX: Il part sans mangé le matin.               → manger
- FAUX: Je veux mangé maintenant.                  → manger (semi-auxiliaire « vouloir »)
- FAUX: Il doit travaillé demain.                  → travailler (semi-auxiliaire « devoir »)
- FAUX: Nous allons mangé bientôt.                 → manger (semi-auxiliaire « aller »)

## … → -ez (sujet « vous » + 2ᵉ personne du pluriel)

> Un sujet « vous » en tête de proposition appelle la forme en « -ez ».

- FAUX: Vous manger trop de sucre.                 → mangez
- FAUX: Vous mangé trop vite.                       → mangez

## OK — ne rien signaler (antipatterns)

### Participe passé correct (auxiliaire)
- OK: Il a mangé une pomme.
- OK: Elle est tombée dans l'escalier.
- OK: Ils sont arrivés hier soir.

### Infinitif correct (préposition / semi-auxiliaire)
- OK: Il commence à manger.
- OK: Je veux manger ce soir.
- OK: Il va manger bientôt.
- OK: Il part sans manger le matin.

### « -ez » correct
- OK: Vous mangez trop de sucre.

### Homographes et pièges
- OK: Il veut vous voir demain.        (« vous » objet, pas sujet)
- OK: Il n'y a rien de changé.         (« de » + participe adjectival)
- OK: Le saumon fumé est délicieux.    (participe épithète, non gouverné)
- OK: Un travail à finir avant ce soir. (« à » + infinitif du 3ᵉ groupe)
- OK: Il a un déjeuner important.      (nom homographe après déterminant)
- OK: Il est venu hier.                (participe en -ir, hors champ)
- OK: Nous avons fait le travail.      (participe en -re, hors champ)

## Limites assumées (gaps)

### -ai / -ais (futur ↔ conditionnel)
> « je mangerai » (futur) et « je mangerais » (conditionnel) se prononcent
> presque à l'identique mais dépendent du **sens** (et souvent d'une subordonnée
> en « si … »). Aucun signal local séparable : non traité.

### -ais / -ait (personne)
> L'opposition « je/tu mange**ais** » ↔ « il mange**ait** » est une affaire
> d'**accord sujet–verbe**, déjà couverte par
> [`rules::conjugation`](../crates/hugo-core/src/rules/conjugation.rs). Hors champ
> de cette tranche.

### « vous » sujet hors tête de proposition
> « vous » n'est reconnu comme sujet qu'en tête de membre (rien à gauche, ou après
> une conjonction de coordination), pour ne pas confondre avec le « vous » objet
> (« il veut vous parler »). « Hier vous manger trop » n'est donc pas corrigé
> (précision > rappel).

### Accord du participe au-delà du masculin singulier (avoir + COD antéposé)
> Avec « avoir », la correction propose le participe masculin singulier
> (« mangé ») ; un éventuel **COD antéposé** (« les pommes qu'il a manger » →
> « mangées ») relève de
> [`rules::past_participle`](../crates/hugo-core/src/rules/past_participle.rs). La
> terminaison /e/ est corrigée, l'accord fin est laissé à cette règle.
