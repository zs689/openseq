//! Error type shared across the core crate.

use std::path::PathBuf;

/// All fallible operations in the core crate return this.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("graph directory does not exist: {0}")]
    GraphMissing(PathBuf),

    #[error("page not found: {0}")]
    PageNotFound(String),

    #[error("invalid page name: {0}")]
    InvalidPageName(String),

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to (de)serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl CoreError {
    /// Convenience constructor that attaches the offending path to an [`std::io::Error`].
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        CoreError::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
