/*
 * hugo.h — API C du correcteur Hugo (crate hugo-ffi).
 *
 * Référence d'API versionnée. Régénérable avec cbindgen :
 *
 *     cbindgen --config crates/hugo-ffi/cbindgen.toml \
 *              --crate hugo-ffi \
 *              --output crates/hugo-ffi/include/hugo.h
 *
 * Le correcteur tourne entièrement en local : dictionnaire et lexique
 * morphologique sont embarqués dans la bibliothèque. Toutes les fonctions
 * tolèrent les pointeurs nuls et ne paniquent jamais à travers la frontière FFI.
 */

#ifndef HUGO_H
#define HUGO_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Poignée opaque vers un correcteur. */
typedef struct HugoChecker HugoChecker;

/* Une suggestion de correction.
 *
 * `start`/`end` sont des offsets d'OCTETS UTF-8 dans le texte source (plage
 * demi-ouverte `start..end`). Toutes les chaînes sont terminées par `\0` et
 * appartiennent au résultat : ne pas les libérer individuellement, utiliser
 * hugo_free_results() sur le HugoResults englobant. */
typedef struct HugoSuggestion {
  size_t start;
  size_t end;
  char *message;
  char **replacements;
  size_t replacements_len;
  char *rule_id;
} HugoSuggestion;

/* Tableau de suggestions. `suggestions` vaut NULL lorsque `len == 0`. */
typedef struct HugoResults {
  HugoSuggestion *suggestions;
  size_t len;
} HugoResults;

/* Crée un correcteur. À libérer avec hugo_checker_free(). */
HugoChecker *hugo_checker_new(void);

/* Libère un correcteur créé par hugo_checker_new(). Sans effet si NULL. */
void hugo_checker_free(HugoChecker *checker);

/* Vérifie `text` (chaîne C UTF-8) et renvoie les suggestions.
 *
 * Renvoie un résultat vide si `checker` ou `text` est NULL, ou si `text` n'est
 * pas de l'UTF-8 valide. Le résultat doit être libéré avec hugo_free_results(). */
HugoResults hugo_checker_check(const HugoChecker *checker, const char *text);

/* Libère un HugoResults et toutes les chaînes qu'il référence. */
void hugo_free_results(HugoResults results);

/* Libération générique d'un pointeur retourné par cette bibliothèque. */
void hugo_free(void *ptr);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HUGO_H */
