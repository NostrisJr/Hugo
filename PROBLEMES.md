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


