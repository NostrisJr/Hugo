//! Phase 7 — **Typographie, ponctuation, espaces, nombres**.
//!
//! Module entièrement **déterministe** : aucune règle n'a besoin du CRF ni de la
//! morphologie. Les règles travaillent sur les jetons de **ponctuation**,
//! d'**espace** et de **nombre** — précisément ceux que les règles grammaticales
//! ignorent — si bien qu'elles n'entrent pas en concurrence avec elles.
//!
//! Chaque sous-famille est une [`Rule`](super::Rule) distincte, dotée de son
//! propre identifiant (`typo_*`), de sorte qu'un intégrateur peut l'activer ou
//! la désactiver en filtrant sur `rule_id` (les conventions typographiques
//! varient selon les contextes). Principe directeur, comme ailleurs dans Hugo :
//! **précision > rappel** — on ne corrige que les cas à signal sûr, on documente
//! les gaps dans `corpus/typo-*.md`.
//!
//! - **Points de suspension** ([`ellipsis`]) : `...` → `…`.
//! - **Doublons de ponctuation** ([`punct_doubling`]) : `!!`, `??`, `,,`…
//! - **Espaces surnuméraires / manquants** ([`spacing`]).
//! - **Ligatures** ([`ligatures`]) : `coeur` → `cœur`.
//! - **Nombres ordinaux** ([`numbers`]) : `1ère` → `1re`, `2ème` → `2e`.

pub mod ellipsis;
pub mod ligatures;
pub mod numbers;
pub mod punct_doubling;
pub mod spacing;

pub use ellipsis::EllipsisRule;
pub use ligatures::LigatureRule;
pub use numbers::OrdinalRule;
pub use punct_doubling::PunctDoublingRule;
pub use spacing::SpacingRule;

/// Calque la casse de `original` sur la forme corrigée `lower` (donnée en
/// minuscules). Tout-majuscule → tout-majuscule ; initiale majuscule →
/// initiale majuscule ; sinon inchangé. Partagé par les règles qui réécrivent
/// un mot (ligatures, ordinaux).
pub(super) fn recase(original: &str, lower: &str) -> String {
    let has_alpha = original.chars().any(|c| c.is_alphabetic());
    if has_alpha && original.chars().all(|c| !c.is_lowercase()) {
        return lower.to_uppercase();
    }
    if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars = lower.chars();
        if let Some(first) = chars.next() {
            return first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    lower.to_string()
}
