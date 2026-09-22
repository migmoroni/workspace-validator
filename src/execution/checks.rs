//! Direct and suite-contextual check execution.

use super::progress::ProgressReporter;
use crate::{
    config::{parameters, ValidatedConfig},
    contracts::{config::CheckConfig, report::*},
    planning::{CheckExecutionKey, ValidationPlan},
    process,
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

/// Suite-local or direct context for one concrete check invocation.
pub(super) struct CheckRunContext<'a> {
    pub arguments: &'a [String],
    pub depends_on: &'a [String],
    pub report_context: CheckContext,
    pub cwd: &'a Path,
    pub local: &'a BTreeMap<String, Status>,
    pub tools: &'a BTreeMap<String, Status>,
    pub cancelled: &'a Arc<AtomicBool>,
}

/// Executes a direct-check plan from the workspace root.
pub(super) fn run_direct(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    tools: &BTreeMap<String, Status>,
    cancelled: &Arc<AtomicBool>,
    progress: &mut impl ProgressReporter,
    results: &mut BTreeMap<CheckExecutionKey, CheckExecutionResult>,
    ordered: &mut Vec<CheckExecutionResult>,
) {
    // Dependency status is local to this direct closure; contextual runs in a
    // different suites must never suppress one another.
    let mut local = BTreeMap::new();
    let total = plan.check_executions.len();
    for (index, execution) in plan.check_executions.iter().enumerate() {
        let check = &validated.checks[&execution.check_id];
        let arguments = parameters::expand(&check.args, &BTreeMap::new());
        progress.direct_check_started(index + 1, total, check);
        let result = execute(
            validated,
            check,
            CheckRunContext {
                arguments: &arguments,
                depends_on: &[],
                report_context: CheckContext::Direct,
                cwd: &validated.workspace_root,
                local: &local,
                tools,
                cancelled,
            },
        );
        local.insert(check.id.clone(), result.status);
        results.insert(execution.key.clone(), result.clone());
        ordered.push(result.clone());
        progress.direct_check_finished(index + 1, total, &result);
    }
}

/// Produces an executed, blocked, or skipped result for one check context.
pub(super) fn execute(
    validated: &ValidatedConfig,
    check: &CheckConfig,
    context: CheckRunContext<'_>,
) -> CheckExecutionResult {
    let command_args = context.arguments.to_vec();
    let argv = std::iter::once(validated.tools[&check.tool_id].program.clone())
        .chain(command_args.clone())
        .collect();
    // A failed check dependency means the command is semantically skipped,
    // while an unavailable executable is an environmental block. Keeping the
    // classifications distinct makes aggregate reports actionable.
    let dependency = context
        .depends_on
        .iter()
        .find(|id| context.local.get(*id) != Some(&Status::Pass));
    let unavailable = std::iter::once(&check.tool_id)
        .chain(&check.requires_tools)
        .find(|id| context.tools.get(*id) != Some(&Status::Pass));
    if let Some(id) = dependency {
        return nonexecuted(
            check,
            context.report_context,
            argv,
            Status::Skipped,
            format!("dependency {id} did not pass"),
        );
    }
    if let Some(id) = unavailable {
        return nonexecuted(
            check,
            context.report_context,
            argv,
            Status::Blocked,
            format!("required tool {id} is unavailable"),
        );
    }
    if context.cancelled.load(Ordering::SeqCst) {
        return nonexecuted(
            check,
            context.report_context,
            argv,
            Status::Skipped,
            "execution interrupted".into(),
        );
    }
    let output = process::run(
        &validated.tools[&check.tool_id].program,
        &command_args,
        context.cwd,
        Duration::from_secs(check.timeout_seconds),
        validated.config.output_limit_bytes,
        context.cancelled,
    );
    // Process start, timeout, and interruption failures do not always provide
    // a meaningful exit code, so they retain an explicit reason as well.
    let reason = output
        .start_error
        .clone()
        .or_else(|| output.timed_out.then(|| "check timed out".into()))
        .or_else(|| output.interrupted.then(|| "check interrupted".into()));
    let status = if output.exit_code == Some(0) && reason.is_none() {
        Status::Pass
    } else {
        Status::Fail
    };
    CheckExecutionResult {
        check_id: check.id.clone(),
        context: context.report_context,
        label: check.label.clone(),
        description: check.description.clone(),
        argv,
        status,
        exit_code: output.exit_code,
        duration_ms: output.duration_ms,
        timeout_seconds: check.timeout_seconds,
        timed_out: output.timed_out,
        stdout_truncated: output.stdout_truncated,
        stderr_truncated: output.stderr_truncated,
        stdout: output.stdout,
        stderr: output.stderr,
        reason,
    }
}

fn nonexecuted(
    check: &CheckConfig,
    context: CheckContext,
    argv: Vec<String>,
    status: Status,
    reason: String,
) -> CheckExecutionResult {
    CheckExecutionResult {
        check_id: check.id.clone(),
        context,
        label: check.label.clone(),
        description: check.description.clone(),
        argv,
        status,
        exit_code: None,
        duration_ms: 0,
        timeout_seconds: check.timeout_seconds,
        timed_out: false,
        stdout_truncated: false,
        stderr_truncated: false,
        stdout: String::new(),
        stderr: String::new(),
        reason: Some(reason),
    }
}
