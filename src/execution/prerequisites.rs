//! Required-tool closure, dependency ordering, and version preflight.

use crate::{
    config::ValidatedConfig,
    contracts::{
        config::VersionParser,
        report::{Status, ToolResult},
    },
    dag::dependency_order,
    process,
};
use semver::{Version, VersionReq};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

/// Executes version preflight for the tools required by selected checks.
pub fn check_tools(
    loaded: &ValidatedConfig,
    check_ids: &[String],
    cancelled: &Arc<AtomicBool>,
) -> Vec<ToolResult> {
    let ordered = necessary_tool_ids(loaded, check_ids);
    let mut statuses = BTreeMap::new();
    let mut results = Vec::new();
    for id in ordered {
        let tool = &loaded.tools[&id];
        // Dependency-first ordering lets a missing runtime block its dependent
        // tools without attempting commands that cannot start meaningfully.
        if let Some(dependency) = tool
            .requires_tools
            .iter()
            .find(|dependency| statuses.get(*dependency) != Some(&Status::Pass))
        {
            statuses.insert(id.clone(), Status::Blocked);
            results.push(ToolResult {
                id,
                program: tool.program.clone(),
                argv: tool.version_args.clone(),
                status: Status::Blocked,
                version: None,
                reason: Some(format!("required tool {dependency} is unavailable")),
            });
            continue;
        }
        let output = process::run(
            &tool.program,
            &tool.version_args,
            &loaded.workspace_root,
            Duration::from_secs(30),
            loaded.config.output_limit_bytes,
            cancelled,
        );
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        let version = match tool.version_parser {
            VersionParser::FirstSemver => first_semver(&combined),
        };
        let reason = if output.start_error.is_some() {
            Some(format!(
                "cannot start {}: {}",
                tool.program,
                output.start_error.unwrap_or_default()
            ))
        } else if output.interrupted {
            Some("interrupted".into())
        } else if output.timed_out {
            Some("version command timed out".into())
        } else if output.exit_code != Some(0) {
            Some(format!(
                "version command exited with {:?}",
                output.exit_code
            ))
        } else if version.is_none() {
            Some("version command did not produce a semantic version".into())
        } else if let (Some(requirement), Some(version)) = (&tool.version_requirement, &version) {
            let requirement = VersionReq::parse(requirement).expect("validated requirement");
            (!requirement.matches(version))
                .then(|| format!("version {version} does not satisfy {requirement}"))
        } else {
            None
        };
        let status = if reason.is_some() {
            Status::Blocked
        } else {
            Status::Pass
        };
        statuses.insert(id.clone(), status);
        results.push(ToolResult {
            id,
            program: tool.program.clone(),
            argv: tool.version_args.clone(),
            status,
            version: version.map(|version| version.to_string()),
            reason,
        });
    }
    results
}

/// Computes the deterministic dependency-first tool closure without execution.
pub(crate) fn necessary_tool_ids(loaded: &ValidatedConfig, check_ids: &[String]) -> Vec<String> {
    let graph: BTreeMap<_, _> = loaded
        .tools
        .iter()
        .map(|(id, tool)| (id.clone(), tool.requires_tools.clone()))
        .collect();
    // Build one closure for checks and repository integration so preflight and
    // read-only CLI inspection always describe the same required tools.
    let mut relevant = BTreeSet::new();
    for id in check_ids {
        let check = &loaded.checks[id];
        for tool in std::iter::once(&check.tool_id).chain(&check.requires_tools) {
            relevant.extend(dependency_order(tool, &graph));
        }
    }
    if let Some(repository) = &loaded.config.repository {
        relevant.extend(dependency_order(&repository.tool_id, &graph));
    }
    // Re-expanding each relevant root in sorted order preserves dependency
    // order while deduplicating convergent tool graphs deterministically.
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();
    for id in relevant.clone() {
        for dependency in dependency_order(&id, &graph) {
            if relevant.contains(&dependency) && seen.insert(dependency.clone()) {
                ordered.push(dependency);
            }
        }
    }
    ordered
}

/// Extracts the first complete semantic version token from command output.
pub fn first_semver(output: &str) -> Option<Version> {
    for token in output.split_whitespace() {
        let candidate = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '.' | '-' | '+')
        });
        let candidate = candidate.strip_prefix('v').unwrap_or(candidate);
        if let Ok(version) = Version::parse(candidate) {
            return Some(version);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::first_semver;

    #[test]
    fn extracts_first_complete_semver() {
        assert_eq!(
            first_semver("tool v22.21.1 build 9.0.0")
                .unwrap()
                .to_string(),
            "22.21.1"
        );
        assert!(first_semver("tool version unknown").is_none());
    }
}
