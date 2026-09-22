//! Secure resolution of suite working directories.

use crate::contracts::config::SuiteConfig;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

/// Canonical directory for a suite and its portable workspace-relative form.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedSuiteDirectory {
    /// Canonical absolute path used as the subprocess working directory.
    pub(crate) absolute: PathBuf,
    /// Portable path relative to the workspace root, or `.` for the root itself.
    pub(crate) relative: String,
}

/// Resolves every suite directory and aggregates path diagnostics.
pub(super) fn resolve_suite_directories(
    suites: &BTreeMap<String, SuiteConfig>,
    root: &Path,
) -> Result<BTreeMap<String, ResolvedSuiteDirectory>, Vec<String>> {
    let mut result = BTreeMap::new();
    let mut errors = Vec::new();
    for (id, suite) in suites {
        let declared = suite
            .working_directory
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        // Canonicalization resolves symlinks before the containment check;
        // validating lexical path components alone cannot prevent an escape.
        match root.join(declared).canonicalize() {
            Ok(absolute) if !absolute.starts_with(root) => errors.push(format!(
                "suite {id} workingDirectory {} escapes workspaceRoot",
                declared.display()
            )),
            Ok(absolute) if !absolute.is_dir() => errors.push(format!(
                "suite {id} workingDirectory {} is not a directory",
                declared.display()
            )),
            Ok(absolute) => {
                result.insert(
                    id.clone(),
                    ResolvedSuiteDirectory {
                        relative: relative_path(root, &absolute),
                        absolute,
                    },
                );
            }
            Err(error) => errors.push(format!(
                "suite {id} has invalid workingDirectory {}: {error}",
                declared.display()
            )),
        }
    }
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(errors)
    }
}

/// Rejects path forms that cannot safely be resolved below a workspace root.
pub(super) fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.is_absolute() || path.as_os_str().is_empty() {
        return Err(format!(
            "workingDirectory {} must be a non-empty relative path",
            path.display()
        ));
    }
    if path.to_string_lossy().as_bytes().contains(&0)
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("workingDirectory {} is unsafe", path.display()));
    }
    Ok(())
}

fn relative_path(root: &Path, absolute: &Path) -> String {
    let relative = absolute.strip_prefix(root).expect("contained path");
    if relative.as_os_str().is_empty() {
        ".".into()
    } else {
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}
