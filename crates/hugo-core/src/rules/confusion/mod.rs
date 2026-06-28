//! Moteur de **confusions** de la phase 6 : familles d'homophones grammaticaux
//! tranchées en s'adossant au CRF ([`crate::pos`]) plutôt qu'à de fragiles vetos
//! de voisinage. Chaque tranche est un module, livrée avec son corpus
//! reconstitué (`corpus/confusion-*.md`).
//!
//! - **Tranche 1 — a/à** ([`a_a`], [`a_a::AAConfusion`]) :
//!   cf. [`corpus/confusion-a-a.md`](../../../../../corpus/confusion-a-a.md).
//! - **Tranche 2 — ce/se, c'est/s'est** ([`ce_se`], [`ce_se::CeSeConfusion`]) :
//!   cf. [`corpus/confusion-ce-se.md`](../../../../../corpus/confusion-ce-se.md).
//! - **Tranche 3 — ou/où, la/là/l'a, leur/leurs, peu/peut/peux**
//!   ([`ou_ou`], [`la_la`], [`leur_leurs`], [`peu_peut`]) : une famille par module,
//!   chacune avec son corpus reconstitué (`corpus/confusion-*.md`).
//! - **Tranche 4 — quel(s)/quelle(s)/qu'elle(s), quand/quant, sans/s'en**
//!   ([`quel_quelle`], [`quand_quant`], [`sans_sen`]) : idem, une famille par
//!   module avec son corpus.
//! - **Tranche 5 — terminaisons homophones -er/-é/-ez** ([`terminaisons`],
//!   [`terminaisons::TerminaisonsConfusion`]) :
//!   cf. [`corpus/confusion-terminaisons.md`](../../../../../corpus/confusion-terminaisons.md).
//!
//! Les helpers communs aux tranches (normalisation, calque de casse, et —
//! depuis la tranche 3 — accès au POS et tests morphologiques de
//! participe/verbe/infinitif) vivent ici.

pub mod a_a;
pub mod accents;
pub mod ce_se;
pub mod dans_den;
pub mod dont_donc;
pub mod et_est;
pub mod la_la;
pub mod leur_leurs;
pub mod ni_ny;
pub mod ou_ou;
pub mod peu_peut;
pub mod plutot;
pub mod pres_pret;
pub mod quand_quant;
pub mod quel_quelle;
pub mod sa_ca;
pub mod sans_sen;
pub mod terminaisons;

pub use a_a::AAConfusion;
pub use accents::AccentsConfusion;
pub use ce_se::CeSeConfusion;
pub use dans_den::DansDenConfusion;
pub use dont_donc::DontDoncConfusion;
pub use et_est::EtEstConfusion;
pub use la_la::LaConfusion;
pub use leur_leurs::LeurConfusion;
pub use ni_ny::NiNyConfusion;
pub use ou_ou::OuConfusion;
pub use peu_peut::PeuConfusion;
pub use plutot::PlutotConfusion;
pub use pres_pret::PresPreConfusion;
pub use quand_quant::QuandConfusion;
pub use quel_quelle::QuelConfusion;
pub use sa_ca::SaCaConfusion;
pub use sans_sen::SansConfusion;
pub use terminaisons::TerminaisonsConfusion;

use crate::morpho;
use crate::pos::{Tagged, Upos};
use crate::tokenizer::Token;

/// Minuscules + apostrophe finale ôtée (`l'` → `l`, `C'` → `c`).
pub(super) fn normalize(text: &str) -> String {
    text.to_lowercase()
        .trim_end_matches(['\'', '\u{2019}'])
        .to_string()
}

/// Calque la casse initiale de `original` sur `replacement`.
pub(super) fn match_case(original: &str, replacement: &str) -> String {
    if !original.chars().next().is_some_and(|c| c.is_uppercase()) {
        return replacement.to_string();
    }
    let mut chars = replacement.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => replacement.to_string(),
    }
}

// --- Helpers partagés (tranche 3) ---

/// Catégorie POS du jeton lexical à la position `k` de la phrase.
pub(super) fn upos(sentence: &[(usize, &Token)], k: usize, tags: &[Tagged]) -> Upos {
    tags[sentence[k].0].upos
}

/// Vrai si `form` admet une analyse verbale **finie** (forme conjuguée).
pub(super) fn is_finite_verb(form: &str) -> bool {
    !morpho::verb_forms(form).is_empty()
}

/// Vrai si `form` admet une analyse de **participe passé** : enregistrement
/// verbal sans personne mais porteur d'un genre ou d'un nombre (« mangé »,
/// « vue », « venus »).
pub(super) fn is_past_participle(form: &str) -> bool {
    morpho::lookup(form).iter().any(|m| {
        m.category == morpho::MorphCategory::Verb
            && m.person.is_none()
            && (m.gender.is_some() || m.number.is_some())
    })
}

/// Vrai si `form` admet une analyse **infinitive** : un enregistrement verbal
/// dont le lemme est la forme elle-même (« marcher », « venir »).
pub(super) fn is_infinitive(form: &str) -> bool {
    let lower = form.to_lowercase();
    morpho::lookup(form)
        .iter()
        .any(|m| m.category == morpho::MorphCategory::Verb && m.lemma == lower)
}
