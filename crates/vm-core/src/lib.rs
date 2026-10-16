//! Cœur de VM Manager — inventaire purement informatif de machines virtuelles.
//!
//! Ce crate ne dépend d'aucune interface graphique : il porte les modèles, la validation
//! des saisies et la persistance JSON, ce qui le rend testable seul (`cargo test -p vm-core`).

pub mod error;
pub mod models;
pub mod query;
pub mod store;
pub mod validation;

pub use error::{CoreError, CoreResult};
pub use models::*;
pub use store::Store;
