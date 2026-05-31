//! Tokenizer français.
//!
//! Découpe un texte en [`Token`]s porteurs de leur [`Span`] (offsets d'octets
//! dans la source). Il gère les spécificités du français :
//!
//! - **élisions** (`l'`, `d'`, `j'`, `qu'`, `jusqu'`, `lorsqu'`…) avec
//!   apostrophe droite `'` (U+0027) **et** typographique `'` (U+2019) ;
//! - **mots composés à trait d'union** (`peut-être`, `dit-il`, `est-ce`,
//!   `arc-en-ciel`) préservés comme un seul mot ;
//! - **apostrophes internes** non élisives (`aujourd'hui`, `prud'homme`)
//!   conservées dans le mot ;
//! - **nombres**, **ponctuation** et **espaces** isolés.
//!
//! Le tokenizer est total : il ne panique sur aucune entrée (chaîne vide,
//! Unicode exotique, émojis), et la concaténation des spans recouvre
//! exactement la source — `&input[t.span.start..t.span.end] == t.text` pour
//! tout token produit.

use crate::Span;

/// Un jeton issu de la tokenisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Texte exact du jeton, tel qu'il apparaît dans la source.
    pub text: String,
    /// Position du jeton dans le texte source (offsets d'octets).
    pub span: Span,
    /// Nature du jeton.
    pub kind: TokenKind,
}

impl Token {
    /// Indique si le jeton est un mot ou une élision (porteur de sens lexical).
    pub fn is_lexical(&self) -> bool {
        matches!(self.kind, TokenKind::Word | TokenKind::Elision)
    }
}

/// Nature d'un [`Token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenKind {
    /// Un mot ordinaire, éventuellement composé (`chat`, `peut-être`).
    Word,
    /// Un signe de ponctuation isolé (`.`, `,`, `!`…).
    Punctuation,
    /// Une suite d'espaces / caractères blancs.
    Whitespace,
    /// Un préfixe élidé incluant l'apostrophe (`l'`, `qu'`, `jusqu'`).
    Elision,
    /// Une suite de chiffres (`42`).
    Number,
}

/// Préfixes susceptibles d'être élidés en français (forme minuscule, sans
/// apostrophe).
const ELISION_PREFIXES: &[&str] = &[
    "l", "d", "j", "n", "m", "t", "s", "c", "qu", "jusqu", "lorsqu", "puisqu", "quelqu", "quoiqu",
];

/// Vrai si le caractère est une apostrophe droite ou typographique.
fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// Vrai si la suite de lettres (déjà en minuscule) est un préfixe élidable.
fn is_elision_prefix(prefix: &str) -> bool {
    ELISION_PREFIXES.contains(&prefix)
}

/// Découpe `input` en jetons.
pub fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let n = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    // Offset d'octet correspondant à l'index de caractère `idx` (ou la fin).
    let byte_at = |idx: usize| -> usize {
        if idx < n {
            chars[idx].0
        } else {
            input.len()
        }
    };

    while i < n {
        let (start, c) = chars[i];

        if c.is_whitespace() {
            let mut j = i + 1;
            while j < n && chars[j].1.is_whitespace() {
                j += 1;
            }
            push(&mut tokens, input, start, byte_at(j), TokenKind::Whitespace);
            i = j;
        } else if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < n && chars[j].1.is_ascii_digit() {
                j += 1;
            }
            push(&mut tokens, input, start, byte_at(j), TokenKind::Number);
            i = j;
        } else if c.is_alphabetic() {
            let (end_idx, kind) = scan_word(&chars, i);
            push(&mut tokens, input, start, byte_at(end_idx), kind);
            i = end_idx;
        } else {
            // Tout le reste (ponctuation, symboles, émojis) : un jeton par
            // caractère, ce qui garantit des spans toujours valides.
            push(
                &mut tokens,
                input,
                start,
                byte_at(i + 1),
                TokenKind::Punctuation,
            );
            i += 1;
        }
    }

    tokens
}

/// Scanne un mot à partir de l'index de caractère `start`.
///
/// Retourne l'index de caractère juste après le mot, et le [`TokenKind`]
/// (`Elision` si le mot se termine par une apostrophe élisive, sinon `Word`).
fn scan_word(chars: &[(usize, char)], start: usize) -> (usize, TokenKind) {
    let n = chars.len();
    let mut j = start;

    loop {
        // Consommer les lettres.
        while j < n && chars[j].1.is_alphabetic() {
            j += 1;
        }

        if j >= n {
            break;
        }

        let cj = chars[j].1;

        if is_apostrophe(cj) {
            let prefix: String = chars[start..j]
                .iter()
                .map(|&(_, c)| c)
                .collect::<String>()
                .to_lowercase();
            if is_elision_prefix(&prefix) {
                // Élision : on inclut l'apostrophe et on s'arrête.
                return (j + 1, TokenKind::Elision);
            }
            // Apostrophe interne non élisive (aujourd'hui) : on la garde et on
            // continue à scanner.
            j += 1;
            continue;
        }

        if cj == '-' && j + 1 < n && chars[j + 1].1.is_alphabetic() {
            // Trait d'union interne (peut-être, dit-il) : on l'inclut.
            j += 1;
            continue;
        }

        break;
    }

    (j, TokenKind::Word)
}

/// Ajoute un jeton dont le texte est exactement `input[start..end]`.
fn push(tokens: &mut Vec<Token>, input: &str, start: usize, end: usize, kind: TokenKind) {
    tokens.push(Token {
        text: input[start..end].to_string(),
        span: Span::new(start, end),
        kind,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(input: &str) -> Vec<String> {
        tokenize(input)
            .into_iter()
            .filter(|t| t.kind == TokenKind::Word)
            .map(|t| t.text)
            .collect()
    }

    #[test]
    fn test_simple_sentence() {
        let tokens = tokenize("Le chat dort.");
        let words: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Word)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(words, vec!["Le", "chat", "dort"]);
    }

    #[test]
    fn test_elision() {
        let tokens = tokenize("L'homme arrive.");
        let words: Vec<&str> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Word | TokenKind::Elision))
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(words, vec!["L'", "homme", "arrive"]);
    }

    #[test]
    fn test_typographic_apostrophe() {
        let tokens = tokenize("l\u{2019}homme");
        let elision = tokens.iter().find(|t| t.kind == TokenKind::Elision);
        assert!(elision.is_some());
        assert_eq!(elision.unwrap().text, "l\u{2019}");
    }

    #[test]
    fn test_inversion() {
        let tokens = tokenize("Peut-être dit-il vrai.");
        let words: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Word)
            .map(|t| t.text.as_str())
            .collect();
        assert!(words.contains(&"dit") || words.contains(&"dit-il"));
    }

    #[test]
    fn test_spans_are_correct() {
        let input = "Le chat dort.";
        let tokens = tokenize(input);
        for token in &tokens {
            assert_eq!(&input[token.span.start..token.span.end], token.text);
        }
    }

    #[test]
    fn test_numbers() {
        let tokens = tokenize("Il a 42 ans.");
        let num = tokens.iter().find(|t| t.kind == TokenKind::Number);
        assert!(num.is_some());
        assert_eq!(num.unwrap().text, "42");
    }

    #[test]
    fn test_qu_elision() {
        let tokens = tokenize("qu'il vienne");
        assert_eq!(tokens[0].kind, TokenKind::Elision);
        assert_eq!(tokens[0].text, "qu'");
    }

    #[test]
    fn test_jusqu_elision() {
        let tokens = tokenize("jusqu'ici");
        assert_eq!(tokens[0].kind, TokenKind::Elision);
        assert_eq!(tokens[0].text, "jusqu'");
        assert_eq!(words("jusqu'ici"), vec!["ici"]);
    }

    #[test]
    fn test_internal_apostrophe_not_elision() {
        // « aujourd'hui » : l'apostrophe est interne, pas élisive.
        let tokens = tokenize("aujourd'hui");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[0].text, "aujourd'hui");
    }

    #[test]
    fn test_compound_hyphen() {
        assert_eq!(words("un arc-en-ciel"), vec!["un", "arc-en-ciel"]);
    }

    #[test]
    fn test_empty_string() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn test_unicode_and_emoji_no_panic() {
        let input = "Café 🦀 — déjà ✓ Ωμέγα";
        let tokens = tokenize(input);
        // Spans valides et concordants pour TOUS les jetons.
        for token in &tokens {
            assert_eq!(&input[token.span.start..token.span.end], token.text);
        }
        assert!(tokens.iter().any(|t| t.text == "Café"));
    }

    #[test]
    fn test_spans_cover_source_contiguously() {
        let input = "L'enfant, lui, peut-être 3 fois.";
        let tokens = tokenize(input);
        let mut cursor = 0;
        for t in &tokens {
            assert_eq!(t.span.start, cursor, "trou ou chevauchement de span");
            cursor = t.span.end;
        }
        assert_eq!(cursor, input.len());
    }

    #[test]
    fn test_punctuation_kind() {
        let tokens = tokenize("Bonjour, monde!");
        let puncts: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Punctuation)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(puncts, vec![",", "!"]);
    }
}
