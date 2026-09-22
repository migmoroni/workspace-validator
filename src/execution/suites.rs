//! Execution and memoization of concrete suites.

use super::{checks, progress::ProgressReporter, RunCollections};
use crate::{
    config::ValidatedConfig,
    contracts::report::{CheckContext, Status, SuiteResult, Summary},
    planning::{CheckExecutionKey, ExecutionNodeRef, PlannedSuite, ValidationPlan},
};
use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicBool, Arc},
};

pub(super) struct SuiteRunContext<'a> {
    pub validated: &'a ValidatedConfig,
    pub plan: &'a ValidationPlan,
    pub tools: &'a BTreeMap<String, Status>,
    pub cancelled: &'a Arc<AtomicBool>,
}

pub(super) fn run(
    context: SuiteRunContext<'_>,
    suite_id: &str,
    path: &[ExecutionNodeRef],
    progress: &mut impl ProgressReporter,
    collections: &mut RunCollections,
) {
    let suite = context
        .plan
        .suites
        .iter()
        .find(|suite| suite.id == suite_id)
        .expect("planned suite");
    if let Some(result) = collections.suite_results.get(suite_id) {
        progress.suite_reused(path, suite, result);
        return;
    }
    progress.suite_started(path, suite, suite.checks.len());
    let mut local = BTreeMap::new();
    for (index, invocation) in suite.checks.iter().enumerate() {
        let check = &context.validated.checks[&invocation.check_id];
        progress.check_started(path, index + 1, suite.checks.len(), check);
        let result = checks::execute(
            context.validated,
            check,
            checks::CheckRunContext {
                arguments: &invocation.arguments,
                depends_on: &invocation.depends_on,
                report_context: CheckContext::Suite {
                    suite_id: suite.id.clone(),
                },
                cwd: &suite.working_directory,
                local: &local,
                tools: context.tools,
                cancelled: context.cancelled,
            },
        );
        local.insert(invocation.check_id.clone(), result.status);
        collections.check_results.insert(
            CheckExecutionKey::Suite {
                suite_id: suite.id.clone(),
                check_id: invocation.check_id.clone(),
            },
            result.clone(),
        );
        collections.ordered_results.push(result.clone());
        progress.check_finished(path, index + 1, suite.checks.len(), &result);
    }
    let result = build_result(suite, context.plan, &collections.check_results);
    progress.suite_finished(path, suite, &result);
    collections.suite_results.insert(suite.id.clone(), result);
}

fn build_result(
    suite: &PlannedSuite,
    plan: &ValidationPlan,
    results: &BTreeMap<CheckExecutionKey, crate::contracts::report::CheckExecutionResult>,
) -> SuiteResult {
    let concrete = plan.suite_check_executions[&suite.id]
        .iter()
        .filter_map(|key| results.get(key))
        .collect::<Vec<_>>();
    SuiteResult {
        id: suite.id.clone(),
        label: suite.label.clone(),
        description: suite.description.clone(),
        working_directory: suite.relative_working_directory.clone(),
        check_duration_ms: concrete.iter().map(|result| result.duration_ms).sum(),
        summary: Summary::from_statuses(concrete.iter().map(|result| result.status)),
        check_ids: suite
            .checks
            .iter()
            .map(|invocation| invocation.check_id.clone())
            .collect(),
    }
}
