/// Erreurs du cœur — messages en français, affichés tels quels dans l'interface
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Validation(String),

    #[error("{kind} introuvable : {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("{0}")]
    Conflict(String),

    #[error("Erreur d'accès au fichier : {0}")]
    Io(#[from] std::io::Error),

    #[error("Fichier de données invalide : {0}")]
    Json(#[from] serde_json::Error),
}

impl CoreError {
    pub fn validation(msg: impl Into<String>) -> Self {
        CoreError::Validation(msg.into())
    }

    pub fn not_found(kind: &'static str, id: impl Into<String>) -> Self {
        CoreError::NotFound {
            kind,
            id: id.into(),
        }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
