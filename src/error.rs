//! Error contract shared by configuration loading and CLI composition.

use std::path::PathBuf;

/// Failures surfaced by configuration loading or CLI orchestration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidatorError {
    /// Command-line options form an unsupported presentation combination.
    #[error("invalid command usage: {0}")]
    Usage(String),
    /// No configuration was found while walking upward from the given path.
    #[error("configuration was not found from {0}")]
    ConfigNotFound(PathBuf),
    /// A discovered configuration could not be read from disk.
    #[error("cannot read configuration {path}: {source}")]
    ConfigRead {
        /// Canonical or explicitly requested configuration path.
        path: PathBuf,
        /// Filesystem error returned while reading the document.
        source: std::io::Error,
    },
    /// The configuration document failed syntactic or semantic validation.
    #[error("invalid configuration {path}: {detail}")]
    InvalidConfig {
        /// Configuration path associated with the diagnostic.
        path: PathBuf,
        /// Complete validation diagnostic, including aggregated errors.
        detail: String,
    },
    /// An unexpected executor or presentation failure occurred.
    #[error("internal executor failure: {0}")]
    Internal(String),
}

impl ValidatorError {
    /// Creates an invalid-configuration error from path-like and text values.
    pub fn invalid(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
        Self::InvalidConfig {
            path: path.into(),
            detail: detail.into(),
        }
    }
}
