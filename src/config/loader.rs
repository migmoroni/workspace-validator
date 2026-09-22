//! Configuration discovery, file safety checks, and JSON deserialization.

use crate::{contracts::config::Config, error::ValidatorError};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Deserialized document paired with its canonical source path.
#[derive(Debug)]
pub struct ParsedConfig {
    /// Raw versioned configuration before semantic validation.
    pub config: Config,
    /// Canonical configuration path used in diagnostics.
    pub path: PathBuf,
}

/// Finds the nearest `.validation/config.json` from `start` upward.
pub fn discover(start: &Path) -> Result<PathBuf, ValidatorError> {
    // Canonicalizing once makes ancestor traversal deterministic even when the
    // caller enters the workspace through a symlinked path.
    let start = start
        .canonicalize()
        .map_err(|error| ValidatorError::Internal(error.to_string()))?;
    for directory in start.ancestors() {
        let candidate = directory.join(".validation/config.json");
        if candidate.exists() {
            return require_regular_file(&candidate);
        }
    }
    Err(ValidatorError::ConfigNotFound(start))
}

/// Reads an explicit or discovered configuration without semantic validation.
pub fn load(explicit: Option<&Path>, current: &Path) -> Result<ParsedConfig, ValidatorError> {
    let path = match explicit {
        Some(path) => require_regular_file(path)?,
        None => discover(current)?,
    };
    let contents = fs::read_to_string(&path).map_err(|source| ValidatorError::ConfigRead {
        path: path.clone(),
        source,
    })?;
    let config = serde_json::from_str(&contents)
        .map_err(|error| ValidatorError::invalid(&path, error.to_string()))?;
    Ok(ParsedConfig { config, path })
}

fn require_regular_file(path: &Path) -> Result<PathBuf, ValidatorError> {
    // `symlink_metadata` deliberately rejects a symlink at the configuration
    // boundary before canonicalization can hide its file type.
    let metadata = fs::symlink_metadata(path).map_err(|source| ValidatorError::ConfigRead {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.file_type().is_file() {
        return Err(ValidatorError::invalid(
            path,
            "configuration must be a regular file",
        ));
    }
    path.canonicalize()
        .map_err(|source| ValidatorError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })
}
