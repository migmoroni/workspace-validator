//! Git-visible repository snapshots and mutation detection.

mod fingerprint;
mod status;

use crate::{config::ValidatedConfig, process};
use fingerprint::WorktreeState;
use status::StatusRecord;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

const INDEX_PATH_BATCH_BYTES: usize = 8 * 1024;
const INDEX_PATH_BATCH_COUNT: usize = 512;

/// Git-visible paths and fingerprints captured at one point in time.
#[derive(Clone, Debug)]
pub struct RepositorySnapshot {
    entries: Vec<String>,
    states: BTreeMap<String, EntryState>,
}

impl RepositorySnapshot {
    /// Returns sorted porcelain-style entries for portable report output.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntryState {
    status: String,
    related_path: Option<String>,
    index_entries: Vec<String>,
    worktree: Vec<(String, WorktreeState)>,
}

/// Captures the configured repository state without modifying it.
pub fn snapshot(
    loaded: &ValidatedConfig,
    cancelled: &Arc<AtomicBool>,
) -> Result<RepositorySnapshot, String> {
    let status = run_repository_command(
        loaded,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        cancelled,
    )?;
    let records = status::parse(&status)?;
    // Porcelain status alone cannot detect content changes that retain the same
    // status code. Index metadata and worktree fingerprints complete identity.
    let index_entries = read_index_entries(loaded, &records, cancelled)?;
    let mut entries = Vec::with_capacity(records.len());
    let mut states = BTreeMap::new();

    for record in records {
        let mut indexed = Vec::new();
        if !record.is_untracked() {
            for path in record.paths() {
                if let Some(values) = index_entries.get(path) {
                    indexed.extend(values.iter().map(|value| format!("{path}\0{value}")));
                }
            }
        }
        indexed.sort();

        let worktree = record
            .paths()
            .map(|path| {
                fingerprint::path(&loaded.workspace_root, path)
                    .map(|fingerprint| (path.to_string(), fingerprint))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let display = record.display();
        let path = record.path.clone();
        let state = EntryState {
            status: record.status,
            related_path: record.related_path,
            index_entries: indexed,
            worktree,
        };
        if states.insert(path.clone(), state).is_some() {
            return Err(format!("repository status repeats path {path:?}"));
        }
        entries.push(display);
    }

    entries.sort();
    Ok(RepositorySnapshot { entries, states })
}

/// Compares snapshots into introduced, removed, and content-changed paths.
pub fn compare(
    before: &RepositorySnapshot,
    after: &RepositorySnapshot,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let before_paths: BTreeSet<_> = before.states.keys().cloned().collect();
    let after_paths: BTreeSet<_> = after.states.keys().cloned().collect();
    let introduced = after_paths.difference(&before_paths).cloned().collect();
    let removed = before_paths.difference(&after_paths).cloned().collect();
    let changed = before_paths
        .intersection(&after_paths)
        .filter(|path| before.states.get(*path) != after.states.get(*path))
        .cloned()
        .collect();
    (introduced, removed, changed)
}

fn run_repository_command(
    loaded: &ValidatedConfig,
    args: &[&str],
    cancelled: &Arc<AtomicBool>,
) -> Result<String, String> {
    let repository = loaded
        .config
        .repository
        .as_ref()
        .ok_or("repository is not configured")?;
    let tool = &loaded.tools[&repository.tool_id];
    let args: Vec<String> = args.iter().map(|argument| (*argument).into()).collect();
    let output = process::run(
        &tool.program,
        &args,
        &loaded.workspace_root,
        Duration::from_secs(120),
        loaded.config.output_limit_bytes,
        cancelled,
    );
    if let Some(error) = output.start_error {
        return Err(format!("cannot start repository provider: {error}"));
    }
    if output.interrupted {
        return Err("repository snapshot interrupted".into());
    }
    if output.timed_out {
        return Err("repository snapshot timed out".into());
    }
    if output.exit_code != Some(0) {
        return Err(format!(
            "repository snapshot exited with {:?}: {}",
            output.exit_code,
            output.stderr.trim()
        ));
    }
    if output.stdout_truncated {
        return Err(format!(
            "repository snapshot exceeds outputLimitBytes ({})",
            loaded.config.output_limit_bytes
        ));
    }
    Ok(output.stdout)
}

fn read_index_entries(
    loaded: &ValidatedConfig,
    records: &[StatusRecord],
    cancelled: &Arc<AtomicBool>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let paths: BTreeSet<_> = records
        .iter()
        .filter(|record| !record.is_untracked())
        .flat_map(StatusRecord::paths)
        .map(str::to_owned)
        .collect();
    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut batch = Vec::new();
    let mut batch_bytes = 0;

    // Batching caps both argument count and command-line bytes, avoiding OS
    // limits in repositories with many tracked changes.
    for path in paths {
        let path_bytes = path.len() + 1;
        if !batch.is_empty()
            && (batch.len() >= INDEX_PATH_BATCH_COUNT
                || batch_bytes + path_bytes > INDEX_PATH_BATCH_BYTES)
        {
            append_index_batch(loaded, &batch, cancelled, &mut result)?;
            batch.clear();
            batch_bytes = 0;
        }
        batch_bytes += path_bytes;
        batch.push(path);
    }
    if !batch.is_empty() {
        append_index_batch(loaded, &batch, cancelled, &mut result)?;
    }
    for entries in result.values_mut() {
        entries.sort();
    }
    Ok(result)
}

fn append_index_batch(
    loaded: &ValidatedConfig,
    paths: &[String],
    cancelled: &Arc<AtomicBool>,
    result: &mut BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    // Literal pathspecs prevent filenames containing Git metacharacters from
    // expanding into unrelated index entries.
    let mut args = vec!["--literal-pathspecs", "ls-files", "--stage", "-z", "--"];
    args.extend(paths.iter().map(String::as_str));
    let output = run_repository_command(loaded, &args, cancelled)?;
    for entry in output.split('\0').filter(|entry| !entry.is_empty()) {
        let (metadata, path) = entry
            .split_once('\t')
            .ok_or_else(|| format!("invalid index entry {entry:?}"))?;
        result
            .entry(path.to_string())
            .or_default()
            .push(metadata.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compare, EntryState, RepositorySnapshot, WorktreeState};
    use std::collections::BTreeMap;

    fn snapshot(digest: u8, status: &str) -> RepositorySnapshot {
        RepositorySnapshot {
            entries: vec![format!("{status} shared")],
            states: BTreeMap::from([(
                "shared".into(),
                EntryState {
                    status: status.into(),
                    related_path: None,
                    index_entries: vec!["100644 object 0".into()],
                    worktree: vec![(
                        "shared".into(),
                        WorktreeState::File {
                            digest: [digest; 32],
                            executable: false,
                        },
                    )],
                },
            )]),
        }
    }

    #[test]
    fn detects_content_changes_without_status_changes() {
        let before = snapshot(1, "AM");
        let after = snapshot(2, "AM");
        assert_eq!(
            compare(&before, &after),
            (vec![], vec![], vec!["shared".into()])
        );
    }
}
