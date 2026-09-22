//! Loading and semantic validation for workspace-validator configuration.

mod directories;
mod loader;
pub(crate) mod parameters;
mod validation;

pub use validation::ValidatedConfig;

use crate::error::ValidatorError;
use std::path::Path;

/// Loads and fully validates a configuration before any subprocess starts.
///
/// # Errors
///
/// Returns [`ValidatorError`] when discovery, file access, JSON deserialization,
/// or semantic validation fails. No declared tool or check is executed before
/// this function returns successfully.
pub fn load(explicit: Option<&Path>, current: &Path) -> Result<ValidatedConfig, ValidatorError> {
    validation::validate(loader::load(explicit, current)?)
}
