//! Règle : confusion **dans / d'en** — tranche 6.
//!
//! - **`dans`** = préposition de lieu ou de temps (*«il est dans la maison»*) ;
//! - **`d'en`** = préposition *de* + pronom/adverbe *en*, suivi d'un **infinitif**
//!   (*«il essaie d'en sortir»*, *«il se garde d'en parler»*).
//!
//! ## dans → d'en
//!
//! *«il essaie dans sortir»* → *«d'en sortir»*
//!
//! Signal : `dans` directement suivi d'un **infinitif** (le groupe `dans + INF`
//! est toujours fautif : *«dans»* préposition ne gouverne jamais un infinitif nu
//! — il lui faut un GN : *«dans un bois»*, *«dans la forêt»*).
//!
//! Limite : *«dans le but de partir»* est correct (dans + GN + de + INF).

use super::{is_infinitive, normalize};
use crate::rules::{lexical_sentences, Rule};
use crate::tokenizer::Token;
use crate::Suggestion;

pub struct DansDenConfusion;

const RULE_ID: &str = "confusion_dans_den";

impl Rule for DansDenConfusion {
    fn check(&self, tokens: &[Token]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for sentence in lexical_sentences(tokens) {
            let len = sentence.len();
            for i in 0..len {
                if normalize(sentence[i].1.text.as_str()) != "dans" {
                    continue;
                }
                // Suivi directement d'un infinitif
                let Some((_, next_tok)) = sentence.get(i + 1) else {
                    continue;
                };
                if !is_infinitive(&next_tok.text) {
                    continue;
                }
                let tok = sentence[i].1;
                suggestions.push(Suggestion {
                    span: tok.span,
                    message: "Confusion «\u{a0}dans\u{a0}»/«\u{a0}d'en\u{a0}» : la préposition «\u{a0}dans\u{a0}» ne gouverne pas un infinitif nu.".to_string(),
                    replacements: vec!["d'en".to_string()],
                    rule_id: RULE_ID,
                });
            }
        }
        suggestions
    }

    fn name(&self) -> &'static str {
        "Confusion « dans » / « d'en »"
    }

    fn id(&self) -> &'static str {
        RULE_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn first(text: &str) -> Option<String> {
        DansDenConfusion
            .check(&tokenize(text))
            .into_iter()
            .next()
            .and_then(|s| s.replacements.into_iter().next())
    }

    fn count(text: &str) -> usize {
        DansDenConfusion.check(&tokenize(text)).len()
    }

    #[test]
    fn dans_before_infinitive() {
        assert_eq!(first("il essaie dans sortir"), Some("d'en".into()));
        assert_eq!(first("il se garde dans parler"), Some("d'en".into()));
        assert_eq!(first("je tente dans savoir"), Some("d'en".into()));
    }

    #[test]
    fn dans_correct_before_noun() {
        assert_eq!(count("il est dans la maison"), 0);
        assert_eq!(count("dans un bois sombre"), 0);
        assert_eq!(count("dans le but de partir"), 0);
    }

    #[test]
    fn den_correct() {
        assert_eq!(count("il essaie d'en sortir"), 0);
        assert_eq!(count("il se garde d'en parler"), 0);
    }
}
