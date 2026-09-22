//! Execution orchestration for immutable validation plans.

pub mod progress;

mod checks;
mod groups;
mod prerequisites;
mod suites;

use crate::{
    config::ValidatedConfig,
    contracts::report::*,
    planning::{CheckExecutionKey, ExecutionNodeRef, ValidationPlan},
    repository,
};
use progress::{ProgressPhase, ProgressReporter, SilentProgress};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use prerequisites::necessary_tool_ids;

/// Completed report together with the cancellation state observed by the run.
#[derive(Debug)]
#[non_exhaustive]
pub struct RunOutcome {
    /// Complete portable report produced by the run.
    pub report: ValidationReport,
    /// Whether cancellation was observed at any point during execution.
    pub interrupted: bool,
}

struct RunCollections {
    check_results: BTreeMap<CheckExecutionKey, CheckExecutionResult>,
    ordered_results: Vec<CheckExecutionResult>,
    group_results: BTreeMap<String, GroupResult>,
    suite_results: BTreeMap<String, SuiteResult>,
}

/// Executes a plan without transient progress output.
///
/// This is the preferred entry point for JSON and programmatic consumers.
pub fn run(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    cancelled: Arc<AtomicBool>,
) -> RunOutcome {
    run_with_progress(validated, plan, cancelled, &mut SilentProgress)
}

/// Executes a plan while publishing lifecycle events to a progress reporter.
///
/// Preflight always runs before repository capture and checks. A configured
/// repository is sampled around check execution only when its provider tool
/// passes preflight. Cancellation is cooperative and reflected in both the
/// report and [`RunOutcome::interrupted`].
pub fn run_with_progress(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    cancelled: Arc<AtomicBool>,
    progress: &mut impl ProgressReporter,
) -> RunOutcome {
    let started = Instant::now();
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    progress.validation_started(&plan.selection);
    let relevant_checks = plan
        .check_executions
        .iter()
        .map(|execution| execution.check_id.clone())
        .collect::<Vec<_>>();
    progress.phase_started(ProgressPhase::Prerequisites);
    let tools = prerequisites::check_tools(validated, &relevant_checks, &cancelled);
    let prerequisite_status = if tools.iter().all(|tool| tool.status == Status::Pass) {
        Status::Pass
    } else {
        Status::Blocked
    };
    progress.phase_finished(ProgressPhase::Prerequisites, prerequisite_status);
    let tool_status: BTreeMap<_, _> = tools
        .iter()
        .map(|tool| (tool.id.clone(), tool.status))
        .collect();
    let repository_ready = validated
        .config
        .repository
        .as_ref()
        .is_some_and(|repo| tool_status.get(&repo.tool_id) == Some(&Status::Pass));
    let mut repository_duration = 0u64;
    // Snapshots intentionally bracket only selected check execution. Preflight
    // commands are read-only prerequisites and are not part of mutation blame.
    let before = if validated.config.repository.is_some() && repository_ready {
        progress.phase_started(ProgressPhase::InitialRepositorySnapshot);
        let instant = Instant::now();
        let value = repository::snapshot(validated, &cancelled);
        repository_duration += instant.elapsed().as_millis() as u64;
        progress.phase_finished(
            ProgressPhase::InitialRepositorySnapshot,
            if value.is_ok() {
                Status::Pass
            } else {
                Status::Blocked
            },
        );
        Some(value)
    } else {
        None
    };

    let mut collections = RunCollections {
        check_results: BTreeMap::new(),
        ordered_results: Vec::new(),
        group_results: BTreeMap::new(),
        suite_results: BTreeMap::new(),
    };
    match &plan.selection {
        ValidationSelection::Check { .. } => checks::run_direct(
            validated,
            plan,
            &tool_status,
            &cancelled,
            progress,
            &mut collections.check_results,
            &mut collections.ordered_results,
        ),
        ValidationSelection::Group { id } => groups::run(
            validated,
            plan,
            id,
            &tool_status,
            &cancelled,
            progress,
            &mut collections,
        ),
        ValidationSelection::Suite { id } => suites::run(
            suites::SuiteRunContext {
                validated,
                plan,
                tools: &tool_status,
                cancelled: &cancelled,
            },
            id,
            &[ExecutionNodeRef::Suite { id: id.clone() }],
            progress,
            &mut collections,
        ),
    }

    let repository_report = validated
        .config
        .repository
        .as_ref()
        .map(|repository_config| {
            let after = if repository_ready {
                progress.phase_started(ProgressPhase::FinalRepositorySnapshot);
                let instant = Instant::now();
                let value = repository::snapshot(validated, &cancelled);
                repository_duration += instant.elapsed().as_millis() as u64;
                progress.phase_finished(
                    ProgressPhase::FinalRepositorySnapshot,
                    if value.is_ok() {
                        Status::Pass
                    } else if cancelled.load(Ordering::SeqCst) {
                        Status::Skipped
                    } else {
                        Status::Blocked
                    },
                );
                Some(value)
            } else {
                None
            };
            // Repository integrity is synthetic and counted once outside the
            // ordinary check list, regardless of how many suites share it.
            let (
                integrity_status,
                reason,
                before_entries,
                after_entries,
                introduced,
                removed,
                changed,
            ) = match (before.as_ref(), after.as_ref()) {
                (Some(Ok(before)), Some(Ok(after))) => {
                    let instant = Instant::now();
                    let (introduced, removed, changed) = repository::compare(before, after);
                    repository_duration += instant.elapsed().as_millis() as u64;
                    let mutated = repository_config.detect_mutations
                        && !(introduced.is_empty() && removed.is_empty() && changed.is_empty());
                    (
                        if mutated { Status::Fail } else { Status::Pass },
                        mutated.then(|| "validation changed Git-visible workspace state".into()),
                        Some(before.entries().to_vec()),
                        Some(after.entries().to_vec()),
                        Some(introduced),
                        Some(removed),
                        Some(changed),
                    )
                }
                (Some(Err(reason)), _) | (_, Some(Err(reason))) => (
                    if cancelled.load(Ordering::SeqCst) {
                        Status::Skipped
                    } else {
                        Status::Blocked
                    },
                    Some(reason.clone()),
                    before
                        .as_ref()
                        .and_then(|v| v.as_ref().ok())
                        .map(|v| v.entries().to_vec()),
                    after
                        .as_ref()
                        .and_then(|v| v.as_ref().ok())
                        .map(|v| v.entries().to_vec()),
                    None,
                    None,
                    None,
                ),
                _ => (
                    if cancelled.load(Ordering::SeqCst) {
                        Status::Skipped
                    } else {
                        Status::Blocked
                    },
                    Some("repository tool is unavailable".into()),
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            };
            RepositoryReport {
                provider: repository_config.provider.clone(),
                integrity: RepositoryIntegrityResult {
                    id: "repository.integrity".into(),
                    label: "Integridade do repositório".into(),
                    status: integrity_status,
                    duration_ms: repository_duration,
                    reason,
                },
                before: before_entries,
                after: after_entries,
                introduced,
                removed,
                changed,
            }
        });
    let mut statuses = collections
        .ordered_results
        .iter()
        .map(|result| result.status)
        .collect::<Vec<_>>();
    if let Some(repository) = &repository_report {
        statuses.push(repository.integrity.status);
    }
    let summary = Summary::from_statuses(statuses);
    let groups = plan
        .groups
        .iter()
        .filter_map(|group| collections.group_results.get(&group.id).cloned())
        .collect();
    let suites = plan
        .suites
        .iter()
        .filter_map(|suite| collections.suite_results.get(&suite.id).cloned())
        .collect();
    let report = ValidationReport {
        schema_version: REPORT_SCHEMA_VERSION,
        selection: plan.selection.clone(),
        workspace_root: ".".into(),
        started_at_unix_ms,
        duration_ms: started.elapsed().as_millis() as u64,
        tools,
        groups,
        suites,
        checks: collections.ordered_results,
        repository: repository_report,
        summary,
    };
    progress.validation_finished(report.summary.result);
    RunOutcome {
        interrupted: cancelled.load(Ordering::SeqCst),
        report,
    }
}
