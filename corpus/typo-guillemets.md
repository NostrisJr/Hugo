# Corpus — guillemets français

> Phrases **originales** rédigées pour ce projet. Spécification *et* tests de
> non-régression de la règle `typo_quotes` (phase 7).
>
> Une **paire** de guillemets droits `"…"` devient `« … »` avec insécables
> (`«[insec]…[insec]»`, `[insec]` = U+00A0). Une espace ordinaire bordant un
> guillemet à l'intérieur est absorbée (pas de double espace).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.

## " … " → « … »

- FAUX: il dit "oui" enfin.        → «[insec]oui[insec]»
- FAUX: il dit " oui " enfin.      → «[insec]oui[insec]» (espaces absorbées)
- FAUX: le mot "liberté" résonne.  → «[insec]liberté[insec]»

## OK  (ne rien signaler)

- OK: une mesure de 5" environ.    (guillemet isolé : pouce, non apparié)
- OK: vide "" ici.                 (paire vide, aucun contenu)
- OK: il dit « oui » enfin.        (déjà des guillemets français)

## Gaps assumés

- Guillemets **imbriqués** `“ … ”` (anglais/typographiques courbes) : non
  convertis pour l'instant.
- Guillemet droit **isolé** (nombre impair) : laissé tel quel (précision >
  rappel : on ne devine pas où placer le guillemet manquant).
