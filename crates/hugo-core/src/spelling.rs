//! Correcteur orthographique adossé à Dicollecte.
//!
//! Le dictionnaire (449 k formes françaises développées depuis le `.dic`/`.aff`
//! Hunspell de Dicollecte, MPL 2.0) est compilé en FST par
//! `tools/compile-dict` puis **embarqué** dans la bibliothèque via
//! [`include_bytes!`]. Aucune ressource externe n'est donc requise à
//! l'exécution.
//!
//! - [`SpellChecker::contains`] teste l'appartenance (insensible à la casse) ;
//! - [`SpellChecker::suggest`] propose des corrections par automate de
//!   Levenshtein sur le FST, reclassées par distance de Damerau-Levenshtein.

use fst::automaton::Levenshtein;
use fst::{IntoStreamer, Set, Streamer};

/// FST des formes françaises, embarqué à la compilation.
static DICT_FST: &[u8] = include_bytes!("../assets/dicollecte.fst");

/// Nombre maximal de candidats remontés du FST avant reclassement.
const MAX_CANDIDATES: usize = 256;

/// Correcteur orthographique adossé à un dictionnaire de formes françaises.
pub struct SpellChecker {
    set: Set<&'static [u8]>,
}

impl SpellChecker {
    /// Construit un correcteur à partir du dictionnaire embarqué.
    ///
    /// # Panics
    /// Panique uniquement si le FST embarqué est corrompu, ce qui ne peut
    /// arriver qu'en cas d'asset invalide au moment de la compilation.
    pub fn new() -> Self {
        Self::try_new().expect("FST orthographique embarqué invalide")
    }

    /// Variante faillible de [`SpellChecker::new`].
    pub fn try_new() -> Result<Self, fst::Error> {
        Ok(SpellChecker {
            set: Set::new(DICT_FST)?,
        })
    }

    /// Indique si une forme appartient au dictionnaire (insensible à la casse).
    pub fn contains(&self, word: &str) -> bool {
        if self.set.contains(word) {
            return true;
        }
        let lower = word.to_lowercase();
        lower != word && self.set.contains(&lower)
    }

    /// Propose jusqu'à `max` corrections pour une forme supposée erronée,
    /// triées de la plus à la moins pertinente.
    pub fn suggest(&self, word: &str, max: usize) -> Vec<String> {
        if max == 0 {
            return Vec::new();
        }
        let query = word.to_lowercase();
        let query_chars: Vec<char> = query.chars().collect();

        // Plus le mot est court, plus on restreint la distance (sinon le bruit
        // explose). 1 erreur pour <=4 lettres, 2 au-delà.
        let max_dist = if query_chars.len() <= 4 { 1 } else { 2 };

        let lev = match Levenshtein::new(&query, max_dist) {
            Ok(l) => l,
            // Mot trop long pour construire l'automate : pas de suggestion.
            Err(_) => return Vec::new(),
        };

        let mut candidates: Vec<String> = Vec::new();
        let mut stream = self.set.search(&lev).into_stream();
        while let Some(key) = stream.next() {
            candidates.push(String::from_utf8_lossy(key).into_owned());
            if candidates.len() >= MAX_CANDIDATES {
                break;
            }
        }

        // Reclassement par distance de Damerau-Levenshtein (qui valorise les
        // transpositions, fréquentes en frappe), puis : on défavorise les noms
        // propres (initiale majuscule) face à un mot tapé en minuscules, puis
        // proximité de longueur, puis ordre alphabétique.
        // NB : à distances égales, un classement par fréquence lexicale serait
        // idéal — prévu dans une itération ultérieure.
        let query_is_lower = query == word;
        candidates.sort_by_cached_key(|c| {
            let cand_chars: Vec<char> = c.to_lowercase().chars().collect();
            let dist = damerau_levenshtein(&query_chars, &cand_chars);
            let proper_noun_penalty =
                u8::from(query_is_lower && c.chars().next().is_some_and(|ch| ch.is_uppercase()));
            let len_diff = cand_chars.len().abs_diff(query_chars.len());
            (dist, proper_noun_penalty, len_diff, c.clone())
        });

        // Déduplication insensible à la casse (on conserve la première
        // occurrence, donc la variante minuscule grâce au tri ci-dessus).
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|c| seen.insert(c.to_lowercase()));
        candidates.truncate(max);
        candidates
    }
}

impl Default for SpellChecker {
    fn default() -> Self {
        SpellChecker::new()
    }
}

/// Distance de Damerau-Levenshtein restreinte (Optimal String Alignment) entre
/// deux suites de caractères : insertion, suppression, substitution et
/// transposition de caractères adjacents, chacune de coût 1.
fn damerau_levenshtein(a: &[char], b: &[char]) -> usize {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    // Trois lignes glissantes du tableau de programmation dynamique.
    let mut prev2 = vec![0usize; m + 1];
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut cur = vec![0usize; m + 1];

    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut val = (prev[j] + 1) // suppression
                .min(cur[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                val = val.min(prev2[j - 2] + 1); // transposition
            }
            cur[j] = val;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }

    prev[m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_words_are_accepted() {
        let sc = SpellChecker::new();
        for w in [
            "chat", "chats", "maison", "maisons", "mange", "être", "café",
        ] {
            assert!(sc.contains(w), "« {w} » devrait être dans le dictionnaire");
        }
    }

    #[test]
    fn case_insensitive() {
        let sc = SpellChecker::new();
        assert!(sc.contains("Bonjour"));
        assert!(sc.contains("MAISON"));
    }

    #[test]
    fn nonsense_is_rejected() {
        let sc = SpellChecker::new();
        assert!(!sc.contains("xyzzyqwf"));
    }

    #[test]
    fn suggests_correction() {
        let sc = SpellChecker::new();
        // « maisn » → « maison »
        let sugg = sc.suggest("maisn", 5);
        assert!(
            sugg.contains(&"maison".to_string()),
            "suggestions = {sugg:?}"
        );
    }

    #[test]
    fn suggests_transposition() {
        let sc = SpellChecker::new();
        // « chien » mal tapé « cihen » (transposition i/h).
        let sugg = sc.suggest("cihen", 5);
        assert!(
            sugg.contains(&"chien".to_string()),
            "suggestions = {sugg:?}"
        );
    }

    #[test]
    fn damerau_basics() {
        let chars = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(damerau_levenshtein(&chars("chat"), &chars("chat")), 0);
        assert_eq!(damerau_levenshtein(&chars("chat"), &chars("chats")), 1);
        assert_eq!(damerau_levenshtein(&chars("ab"), &chars("ba")), 1); // transposition
        assert_eq!(damerau_levenshtein(&chars(""), &chars("abc")), 3);
    }
}
