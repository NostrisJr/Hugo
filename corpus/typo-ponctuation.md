# Corpus — ponctuation (suspension, doublons)

> Phrases **originales** rédigées pour ce projet. Spécification *et* tests de
> non-régression des règles `typo_ellipsis` et `typo_punct_doubling` (phase 7).
>
> Légende : `FAUX → correction` = doit être signalé ; `OK` = ne rien signaler.

## Points de suspension — `typo_ellipsis`

Trois points ou plus → caractère unique `…` (U+2026).

- FAUX: attends...                          → …
- FAUX: et puis.... voilà.                  → …
- OK:   attends…                            (déjà le caractère …)
- OK:   fin. Suite                          (point simple)
- OK:   a . b                               (points séparés par des espaces)

## Doublons de ponctuation — `typo_punct_doubling`

Signe identique redoublé → signe unique. `..` (deux points) = faute de frappe
d'un point (les trois points relèvent de la suspension).

- FAUX: quoi!!                              → !
- FAUX: vraiment??                          → ?
- FAUX: rouge,,vert                         → ,
- FAUX: fin..                               → .
- OK:   quoi !? vraiment ?!                 (signes différents, légitime)
- OK:   oui ! non ?                         (signes simples)
- OK:   attends...                          (suspension, pas un doublon)

## Gaps assumés

- `!?` / `?!` : combinaisons **légitimes**, jamais corrigées.
- Doublons mêlant des signes différents (`,.`, `;,`) : non traités (rares,
  parfois licites comme `).`).
