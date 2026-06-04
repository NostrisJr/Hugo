/*
 * Démonstrateur C de l'API Hugo.
 *
 * Compilation (depuis la racine du dépôt) :
 *
 *   cargo build -p hugo-ffi --release
 *   clang crates/hugo-ffi/examples/c_demo.c \
 *         -I crates/hugo-ffi/include \
 *         -L target/release -lhugo_ffi \
 *         -framework CoreFoundation -framework Security \
 *         -o /tmp/hugo_demo
 *   /tmp/hugo_demo
 */

#include <stdio.h>
#include "hugo.h"

int main(void) {
  HugoChecker *checker = hugo_checker_new();
  if (!checker) {
    fprintf(stderr, "échec de création du correcteur\n");
    return 1;
  }

  const char *text = "il il va a Paris";
  HugoResults results = hugo_checker_check(checker, text);

  printf("Texte : \"%s\"\n%zu suggestion(s)\n\n", text, results.len);
  for (size_t i = 0; i < results.len; i++) {
    HugoSuggestion *s = &results.suggestions[i];
    printf("[%zu..%zu] %s  (%s)\n", s->start, s->end, s->message, s->rule_id);
    for (size_t j = 0; j < s->replacements_len; j++) {
      printf("    -> %s\n", s->replacements[j]);
    }
  }

  hugo_free_results(results);
  hugo_checker_free(checker);
  return 0;
}
