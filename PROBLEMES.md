[PARTIEL — « barres » (#1c) RÉSOLU : `conjugation` consomme le POS et le verbe
candidat doit être étiqueté VERB/AUX, donc l'homographe nom « barres » ne
déclenche plus. « sûre » (#1a, sur/sûr) RÉSOLU : `attribute` refuse un attribut
candidat étiqueté ADP/DET/PRON… (`is_attribute_tag`), donc la préposition « sur »
(« elle est sur le côté ») ne devient plus « sure ». RESTE : « il y a » (#1b) ne
se reproduit pas isolément — à rejouer sur le paragraphe complet.]

ERREURES INVISIBLES POUR LE CORRECTEUR:
[PARTIEL — apposition détachée] “Au bord du lac, endormi à l’ombre des arbres, patientaient les enfants” → devrait être “endormis”. La règle `detached_appositive` (phase 12.2) gère maintenant ce motif. GAP RÉSIDUEL : “enfant” est un nom épicène (genre non enregistré dans le lexique) → le genre par défaut masculin est utilisé → correction “endormis” proposée si le CRF détecte bien “patientaient” comme verbe fini. Pour des noms à genre connu (soldats, voyageuses…), la règle fonctionne pleinement.

“Jeanne à si peu de temps devant elle. Le soir approche et elle à faim. Jeanne est pressé de retrouvé sa famille. “ -> la première erreur de à/a n’estpas attrappée, Jeanne ne semble pas être pris comme le sujet, n’est pas compris comme féminin (certes difficile pour les noms propres. peut-être inclure une liste de noms courants ?). “devant elle” est pris comme une inversion alors que ça ne devrait pas

“Sera-il des nôtres ce soir ? Se dîner sera-il le notre ?” -> ne propose pas le t d’élition pour l’inversion “sera-t-il”. l’explication du pronom possessif est étrange. Le pronom possessif prend un accent circonflexe, et le fait qu’il y ait un article définit avant ou non ne change rien (si ce n’est qu’il n’est pas correct d’employer ce pronom possessif sans article). Et ne voit pas la différence ce/se
[PARTIEL (t euphonique) — `trait_union` : (a) pour deux mots séparés « sera il » → « sera-t-il » ; (b) pour un token déjà lié « Sera-il » → « Sera-t-il » (verbe en voyelle + pronom en voyelle → insertion du *t*). Reste : explication du pronom possessif à améliorer (gap style).]

Il lui dit : “Je ne pense pas que Marc puisses avoir raison” -> cela demande un trai d’union entre dit et Je, sans voir que : “ est un signe de début de nouvelle phrase. par ailleurs, toujours le soucis de Marc qui n’est pas pris comme le sujet, donc ça rate l’erreur d’accord
[PARTIEL (faux positif trait d’union) — `trait_union` : une ponctuation ou guillemet entre le verbe et le pronom bloque la détection d’inversion → plus de suggestion erronée « dit-Je ». Reste : Marc non pris comme sujet (gap identification sujet).]

“Qui’il se taises !” -> rate l’élision du i (qu’il) et l’accord de “taise”

[PARTIEL — repli suffixe 1ᵉʳ groupe]
"Nous implémentes la phase 12." → RÉSOLU : repli par suffixe détecte -es (2sg) ≠ nous (1pl) → « implémentons ».
"Je conjugueriez avec facilité." → RÉSOLU : repli par suffixe détecte -eriez (2pl cond.) ≠ je (1sg) → « conjuguerais ».
"Implémentes la phase 12." → RÉSOLU : `imperatif::ImperatifGroupe1` — règle positionnelle (tête de phrase, suffixe -es, infinitif connu du Lefff, pas de « tu »/verbe fini suivant, pas de virgule) ; indépendante du CRF et du sujet.


[RÉSOLU] Ils ont posé des problèmes aux équipes en charge des nouveaux arrivants -> voiit équipes comme un verbe, et propose donc un trait d’union pour l’inversion. FIX : trait_union impératif gate CRF (NOUN → skip) + exclusion infinitifs.
[RÉSOLU] La synthèse de ces éléments en différents scénarios -> voit différent comme un participe présent. FIX : adjectif_verbal garde "en" gérondif vs préposition (accord adj-nom suivant + CRF next-word).
[RÉSOLU] Compréhension du problème et état de l’art -> veut mettre un verbe à la palce de et. FIX : et_est garde SP (nom précédé de du/de/au/aux = pas sujet).
[RÉSOLU] Il fallu vite en comprendre la complexité. -> le classique problème d’inversion. FIX : trait_union exclusion infinitifs dans looks_like_imperative.
[RÉSOLU] périphériques, applications tierces, processus métiers, contraintes légales, contraintes opérationnelles… -> veut accorder. FIX : epithet check comma_between entre adj et nom candidat.
[RÉSOLU] je ne veux pas la règle de la typo de ... FIX : EllipsisRule retirée de all_rules().
[RÉSOLU] Si un SI peut sembler imbriqué, perclus de dépendances circulaires et d’attaches, différentes grandes catégories se dégagent -> erreur sur peut et perclus. FIX : peu_peut garde CRF NOUN sur quantifieur "si" ; detached_appositive garde SCONJ avant c1.
[RÉSOLU] Par ailleurs, les accès ne sont plus les mêmes -> étrange demande de trait d’union sur plus les. FIX : trait_union impératif gate CRF (ADV → skip).
[RÉSOLU] L’identité est encore moins visible et précède en partie à la sécurité -> problème de trait d’union. FIX : trait_union garde "en" préposition (suivi d’un nom = SP, pas pronom).
[PARTIEL] Cela a un double but : contrôler le shadow IT et être en mesure -> encore une proposition de trait d’union. FIX trait_union : contrôler est un infinitif → exclu. GAP résiduel : "shadow" inconnu (anglais) ; "mesure" pris pour verbe par conjugation (faux positif dans "être en mesure").
[RÉSOLU] Deux solutions visant à automatiser la migration ont été citées. -> rate l'accord de citées. FIX : ANALYSE EN DÉPENDANCES (nouveau parser arc-eager maison). passive_participle lit le sujet via `nsubj:pass` (« solutions »), plus par la position « aux−1 » (« migration »). La phrase correcte reste silencieuse ; un vrai désaccord est corrigé contre le bon sujet éloigné.
[RÉSOLU (capacité)] La nuit était totalement tombée, et le jeune homme avançait toujours -> ne comprend pas le sujet. Le parser donne maintenant un sujet PAR proposition : « nuit » nsubj:pass→tombée, « homme » nsubj→avançait. (Câblé dans passive_participle ; migration de conjugation/attribute encore à faire pour exploiter pleinement.)


4. epithet.rs — "applications tierces**,** processus" [RÉSOLU PAR L'ARBRE]

  L'ancien fix comma_between (heuristique de virgule) a été RETIRÉ du chemin de
  production. epithet::check_tagged lit désormais l'arc `amod` du parser : « tierces »
  est rattaché à « applications » par la structure, par-delà la virgule — sans
  heuristique de surface. Gardes anti-ambiguïté « N de N ADJ » : abstention si un nom
  s'intercale, ou si l'adjectif s'accorde avec un nom ancêtre.


Leurs amis étaient deux fois plus grands qu'eux.
Qui s’intéresse à ce qui se passe sous la terre quand on voit le ciel ?
Tout cela les amusait beaucoup, mais ce qu’ils préféraient, c’était la baignoire.
Mais dès que la machine fut construite, prête, puis écrasée au sol, l’homme l’accusa d’avoir mal fait son travail.

[VETO ARBRE — conjugation/attribute] L'accord sujet–verbe et l'accord d'attribut lisent
désormais le VRAI sujet dans l'arbre (nsubj ; pour une copule, le nsubj du prédicat) afin de
S'ABSTENIR quand le verbe/attribut est déjà accordé. Faux positifs corpus UD test : SVA 13→8,
attribut 7→4 (inversions « écrit l'ONG », compléments « du N », appositions). Zéro régression.
NOTE : un « override » (détecter positivement le sujet via nsubj) a été testé puis RETIRÉ — à
89 % UAS il triplait les faux positifs. L'arbre sert de veto (précision), pas de détecteur (recall) ;
le recall sur sujets masqués relève d'un meilleur parser (beam/oracle), pas d'un patch.
[RÉSOLU] Les FP « autres règles » sur les 4 phrases ci-dessus sont corrigés (444 tests verts) :
- detached_appositive « Tout cela les amusait beaucoup, » → « Tous » : FIX motif 2 — une apposition
  détachée est non-finie ; un verbe fini avant la première virgule = proposition complète, on s'abstient.
- detached_appositive « construite, prête, puis écrasée…, l'homme » → « prêt » : FIX try_direct — le sujet
  préposé d'une apposition doit être contigu ; une virgule entre l'apposé et le sujet signale une autre
  proposition (« homme » est le sujet de « accusa », pas de « prête » qui relève du prédicat de « machine »).
- terminaison « …, puis écrasée au sol » → « écraser » : FIX is_infinitive_governor — un gouverneur
  semi-auxiliaire doit être étiqueté VERB/AUX par le CRF ; « puis » (CCONJ, homographe rare de « pouvoir »)
  ne gouverne plus d'infinitif.
- ce_se « ce qui se passe » → « ce » : FIX — le repli nominal (homographe nom d'un mot tagué VERB) ne
  s'applique qu'aux mots SANS lecture verbale finie (« dîner ») ; « passe »/« porte »/« marche » conjugués
  restent des réfléchis. + « qui » relatif ajouté aux sujets licenciant le réfléchi.