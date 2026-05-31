//! Bindings C pour Hugo.
//!
//! Cette crate expose une API C minimale et sûre permettant d'utiliser le
//! correcteur depuis Swift, Kotlin/JNI, C ou C++. Le header est généré par
//! `cbindgen` (phase 3) ; la compilation produit un `staticlib` (iOS/macOS) et
//! un `cdylib`.
//!
//! # Cycle de vie
//!
//! ```c
//! HugoChecker *c = hugo_checker_new();
//! HugoResults r  = hugo_checker_check(c, "il il mange");
//! // ... lire r.suggestions[0..r.len] ...
//! hugo_free_results(r);
//! hugo_checker_free(c);
//! ```
//!
//! Toutes les fonctions tolèrent les pointeurs nuls et les chaînes non-UTF-8
//! (elles renvoient alors un résultat vide), et ne paniquent jamais à travers
//! la frontière FFI.

use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_void;
use std::ptr;

use hugo_core::Checker;

/// Poignée opaque vers un [`Checker`].
pub struct HugoChecker {
    inner: Checker,
}

/// Une suggestion, représentation compatible C.
#[repr(C)]
pub struct HugoSuggestion {
    /// Offset d'octet de début dans le texte source.
    pub start: usize,
    /// Offset d'octet de fin (exclu).
    pub end: usize,
    /// Message explicatif (chaîne C terminée par `\0`).
    pub message: *mut c_char,
    /// Tableau de corrections proposées (chaînes C).
    pub replacements: *mut *mut c_char,
    /// Nombre de corrections dans `replacements`.
    pub replacements_len: usize,
    /// Identifiant de la règle (chaîne C).
    pub rule_id: *mut c_char,
}

/// Tableau de suggestions retourné par [`hugo_checker_check`].
#[repr(C)]
pub struct HugoResults {
    /// Pointeur vers le premier élément (ou nul si `len == 0`).
    pub suggestions: *mut HugoSuggestion,
    /// Nombre de suggestions.
    pub len: usize,
}

impl HugoResults {
    fn empty() -> Self {
        HugoResults {
            suggestions: ptr::null_mut(),
            len: 0,
        }
    }
}

/// Convertit une `String` Rust en chaîne C allouée sur le tas. Les octets nuls
/// internes sont remplacés afin de garantir une conversion infaillible.
fn to_c_string(s: &str) -> *mut c_char {
    let cleaned = s.replace('\0', " ");
    match CString::new(cleaned) {
        Ok(cs) => cs.into_raw(),
        Err(_) => CString::new("").unwrap_or_default().into_raw(),
    }
}

/// Crée un nouveau correcteur. À libérer avec [`hugo_checker_free`].
#[no_mangle]
pub extern "C" fn hugo_checker_new() -> *mut HugoChecker {
    Box::into_raw(Box::new(HugoChecker {
        inner: Checker::new(),
    }))
}

/// Libère un correcteur créé par [`hugo_checker_new`]. Sans effet si nul.
///
/// # Safety
/// `checker` doit provenir de [`hugo_checker_new`] et ne pas être réutilisé
/// après l'appel.
#[no_mangle]
pub unsafe extern "C" fn hugo_checker_free(checker: *mut HugoChecker) {
    if !checker.is_null() {
        drop(Box::from_raw(checker));
    }
}

/// Vérifie `text` et retourne les suggestions. Le résultat doit être libéré
/// avec [`hugo_free_results`].
///
/// # Safety
/// `checker` doit être valide ou nul ; `text` doit être une chaîne C valide
/// terminée par `\0` ou nul.
#[no_mangle]
pub unsafe extern "C" fn hugo_checker_check(
    checker: *const HugoChecker,
    text: *const c_char,
) -> HugoResults {
    if checker.is_null() || text.is_null() {
        return HugoResults::empty();
    }

    let checker = &*checker;
    let text = match CStr::from_ptr(text).to_str() {
        Ok(t) => t,
        Err(_) => return HugoResults::empty(),
    };

    let mut out: Vec<HugoSuggestion> = checker
        .inner
        .check(text)
        .into_iter()
        .map(|s| {
            let mut reps: Vec<*mut c_char> =
                s.replacements.iter().map(|r| to_c_string(r)).collect();
            reps.shrink_to_fit();
            let replacements_len = reps.len();
            let replacements = if reps.is_empty() {
                ptr::null_mut()
            } else {
                let ptr = reps.as_mut_ptr();
                std::mem::forget(reps);
                ptr
            };
            HugoSuggestion {
                start: s.span.start,
                end: s.span.end,
                message: to_c_string(&s.message),
                replacements,
                replacements_len,
                rule_id: to_c_string(s.rule_id),
            }
        })
        .collect();

    out.shrink_to_fit();
    let len = out.len();
    let suggestions = if out.is_empty() {
        ptr::null_mut()
    } else {
        let ptr = out.as_mut_ptr();
        std::mem::forget(out);
        ptr
    };

    HugoResults { suggestions, len }
}

/// Libère un [`HugoResults`] et toutes les chaînes qu'il référence.
///
/// # Safety
/// `results` doit provenir de [`hugo_checker_check`] et ne pas être réutilisé.
#[no_mangle]
pub unsafe extern "C" fn hugo_free_results(results: HugoResults) {
    if results.suggestions.is_null() || results.len == 0 {
        return;
    }

    let suggestions = Vec::from_raw_parts(results.suggestions, results.len, results.len);
    for s in suggestions {
        free_c_string(s.message);
        free_c_string(s.rule_id);
        if !s.replacements.is_null() && s.replacements_len > 0 {
            let reps = Vec::from_raw_parts(s.replacements, s.replacements_len, s.replacements_len);
            for r in reps {
                free_c_string(r);
            }
        }
    }
}

/// Libère une chaîne C allouée par cette crate. Sans effet si nul.
unsafe fn free_c_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Alias générique de libération de pointeur, exposé pour symétrie d'API.
///
/// # Safety
/// `ptr` doit être un pointeur retourné par cette crate, du type attendu.
#[no_mangle]
pub unsafe extern "C" fn hugo_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut u8));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_roundtrip() {
        unsafe {
            let checker = hugo_checker_new();
            assert!(!checker.is_null());

            let text = CString::new("il il mange").expect("cstring");
            let results = hugo_checker_check(checker, text.as_ptr());
            assert!(results.len >= 1);

            // Lire la première suggestion sans paniquer.
            let first = &*results.suggestions;
            assert!(!first.message.is_null());
            let rule = CStr::from_ptr(first.rule_id).to_str().expect("utf8");
            assert_eq!(rule, "duplicate_word");

            hugo_free_results(results);
            hugo_checker_free(checker);
        }
    }

    #[test]
    fn null_inputs_are_safe() {
        unsafe {
            let r = hugo_checker_check(ptr::null(), ptr::null());
            assert_eq!(r.len, 0);
            hugo_free_results(r);
            hugo_checker_free(ptr::null_mut());
        }
    }
}
