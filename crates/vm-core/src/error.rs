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

    /// Le fichier de données est chiffré : il faut le mot de passe maître pour l'ouvrir
    #[error("Inventaire chiffré : mot de passe maître requis")]
    Locked,

    #[error("Mot de passe maître incorrect")]
    WrongPassword,

    /// Donnée chiffrée illisible (altérée, tronquée ou mal encodée)
    #[error("Erreur de chiffrement : {0}")]
    Crypto(String),

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
