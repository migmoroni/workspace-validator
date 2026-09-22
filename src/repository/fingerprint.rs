//! Stable fingerprints for Git-visible worktree objects.

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata},
    io::{self, Read},
    path::Path,
};

/// Content identity for a path as observed in the worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum WorktreeState {
    Missing,
    File { digest: [u8; 32], executable: bool },
    Symlink(String),
    Directory([u8; 32]),
    Other,
}

/// Fingerprints one repository-relative path without following symlinks.
pub(super) fn path(root: &Path, relative: &str) -> Result<WorktreeState, String> {
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(WorktreeState::Missing),
        Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = fs::read_link(&path)
            .map_err(|error| format!("cannot read symlink {}: {error}", path.display()))?;
        return Ok(WorktreeState::Symlink(
            target.to_string_lossy().into_owned(),
        ));
    }
    if file_type.is_file() {
        return Ok(WorktreeState::File {
            digest: hash_file(&path)?,
            executable: is_executable(&metadata),
        });
    }
    if file_type.is_dir() {
        return Ok(WorktreeState::Directory(hash_directory(&path)?));
    }
    Ok(WorktreeState::Other)
}

fn hash_file(path: &Path) -> Result<[u8; 32], String> {
    let mut file =
        File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn hash_directory(path: &Path) -> Result<[u8; 32], String> {
    fn visit(root: &Path, directory: &Path, hasher: &mut Sha256) -> Result<(), String> {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| format!("cannot read directory {}: {error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot read directory {}: {error}", directory.display()))?;
        // Filesystem iteration order is undefined; sorting is required for a
        // stable digest across repeated snapshots of unchanged content.
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            // A status entry may itself be a directory. Repository metadata is
            // excluded because only user-visible workspace content is tracked.
            if entry.file_name() == ".git" {
                continue;
            }
            let entry_path = entry.path();
            let relative = entry_path.strip_prefix(root).map_err(|error| {
                format!(
                    "cannot normalize directory {}: {error}",
                    entry_path.display()
                )
            })?;
            hasher.update(relative.to_string_lossy().as_bytes());
            hasher.update([0]);
            let metadata = fs::symlink_metadata(&entry_path)
                .map_err(|error| format!("cannot inspect {}: {error}", entry_path.display()))?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                hasher.update(b"symlink\0");
                let target = fs::read_link(&entry_path).map_err(|error| {
                    format!("cannot read symlink {}: {error}", entry_path.display())
                })?;
                hasher.update(target.to_string_lossy().as_bytes());
            } else if file_type.is_file() {
                hasher.update(b"file\0");
                hasher.update(hash_file(&entry_path)?);
                hasher.update([u8::from(is_executable(&metadata))]);
            } else if file_type.is_dir() {
                hasher.update(b"directory\0");
                visit(root, &entry_path, hasher)?;
            } else {
                hasher.update(b"other\0");
            }
            hasher.update([0]);
        }
        Ok(())
    }

    let mut hasher = Sha256::new();
    visit(path, path, &mut hasher)?;
    Ok(hasher.finalize().into())
}

#[cfg(unix)]
fn is_executable(metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &Metadata) -> bool {
    false
}
