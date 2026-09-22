mod common;
use serde_json::json;
use std::{
    fs,
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::Duration,
};
use tempfile::TempDir;
use workspace_validator::{
    config,
    contracts::report::{
        CheckContext, CheckExecutionResult, GroupResult, OverallResult, Status, SuiteResult,
        ValidationSelection,
    },
    execution::{self, progress::ProgressReporter},
    planning::{self, ExecutionNodeRef, PlannedGroup, PlannedSuite},
};

#[derive(Default)]
struct Recorder(Vec<String>);

impl ProgressReporter for Recorder {
    fn validation_started(&mut self, _: &ValidationSelection) {
        self.0.push("validation".into());
    }
    fn group_started(&mut self, _: &[ExecutionNodeRef], group: &PlannedGroup, _: usize) {
        self.0.push(format!("group-start:{}", group.id()));
    }
    fn group_reused(&mut self, _: &[ExecutionNodeRef], group: &PlannedGroup, _: &GroupResult) {
        self.0.push(format!("group-reuse:{}", group.id()));
    }
    fn suite_started(&mut self, _: &[ExecutionNodeRef], suite: &PlannedSuite, _: usize) {
        self.0.push(format!("start:{}", suite.id()));
    }
    fn suite_reused(&mut self, _: &[ExecutionNodeRef], suite: &PlannedSuite, _: &SuiteResult) {
        self.0.push(format!("reuse:{}", suite.id()));
    }
    fn check_finished(
        &mut self,
        _: &[ExecutionNodeRef],
        _: usize,
        _: usize,
        result: &CheckExecutionResult,
    ) {
        self.0.push(format!("check:{}", result.check_id));
    }
    fn suite_finished(&mut self, _: &[ExecutionNodeRef], suite: &PlannedSuite, _: &SuiteResult) {
        self.0.push(format!("finish:{}", suite.id()));
    }
    fn validation_finished(&mut self, _: OverallResult) {
        self.0.push("finished".into());
    }
}

#[test]
fn memoizes_groups_and_suites_but_executes_checks_per_suite_context() {
    let temp = TempDir::new().unwrap();
    let validator_cwd = std::env::current_dir().unwrap();
    for d in ["one", "two"] {
        fs::create_dir(temp.path().join(d)).unwrap();
    }
    let log = temp.path().join("runs.log");
    let tool = common::process_fixture();
    let value = json!({
      "schemaVersion":6,"workspaceRoot":".","defaultGroup":"root","outputLimitBytes":4096,
      "tools":[{"id":"fixture","program":tool,"requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}],
      "checks":[
       {"id":"same","label":"Same","description":"Same definition.","toolId":"fixture","args":["record-context",log,"same","{context}"],"requiresTools":[],"timeoutSeconds":5},
       {"id":"shared","label":"Shared","description":"Shared suite check.","toolId":"fixture","args":["record-context",log,"shared","{context}"],"requiresTools":[],"timeoutSeconds":5}],
      "suites":[
       {"id":"shared.suite","label":"Shared","description":"Shared suite.","workingDirectory":".","checks":[{"checkId":"shared","parameters":{"context":"shared-context"},"dependsOn":[]}]},
       {"id":"suite.one","label":"One","description":"First suite.","workingDirectory":"one","checks":[{"checkId":"same","parameters":{"context":"one-context"},"dependsOn":[]}]},
       {"id":"suite.two","label":"Two","description":"Second suite.","workingDirectory":"two","checks":[{"checkId":"same","parameters":{"context":"two-context"},"dependsOn":[]}]}],
      "groups":[
       {"id":"shared.group","label":"Shared group","description":"Shared group.","members":[{"kind":"suite","id":"shared.suite"}]},
       {"id":"branch.one","label":"Branch one","description":"First branch.","members":[{"kind":"group","id":"shared.group"},{"kind":"suite","id":"suite.one"}]},
       {"id":"branch.two","label":"Branch two","description":"Second branch.","members":[{"kind":"group","id":"shared.group"},{"kind":"suite","id":"shared.suite"},{"kind":"suite","id":"suite.two"}]},
       {"id":"root","label":"Root","description":"Root group.","members":[{"kind":"group","id":"branch.one"},{"kind":"group","id":"branch.two"}]}]});
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    let mut recorder = Recorder::default();
    let outcome = execution::run_with_progress(
        &validated,
        &plan,
        Arc::new(AtomicBool::new(false)),
        &mut recorder,
    );
    assert_eq!(outcome.report.checks.len(), 3);
    assert!(outcome
        .report
        .checks
        .iter()
        .all(|v| v.status == Status::Pass));
    let same = outcome
        .report
        .checks
        .iter()
        .filter(|v| v.check_id == "same")
        .collect::<Vec<_>>();
    assert_eq!(same.len(), 2);
    let suite_one = same
        .iter()
        .find(|v| matches!(&v.context,CheckContext::Suite{suite_id} if suite_id=="suite.one"))
        .unwrap();
    assert_eq!(
        suite_one.argv.last().map(String::as_str),
        Some("one-context")
    );
    assert_eq!(fs::read_to_string(log).unwrap().lines().count(), 3);
    let runs = fs::read_to_string(temp.path().join("runs.log")).unwrap();
    assert!(runs.contains(&format!(
        "same:one-context:{}",
        temp.path().join("one").display()
    )));
    assert!(runs.contains(&format!(
        "same:two-context:{}",
        temp.path().join("two").display()
    )));
    assert_eq!(std::env::current_dir().unwrap(), validator_cwd);
    assert_eq!(
        outcome
            .report
            .suites
            .iter()
            .filter(|v| v.id == "shared.suite")
            .count(),
        1
    );
    assert_eq!(
        recorder
            .0
            .iter()
            .filter(|v| *v == "start:shared.suite")
            .count(),
        1
    );
    assert_eq!(
        recorder
            .0
            .iter()
            .filter(|value| *value == "group-reuse:shared.group")
            .count(),
        1
    );
    assert_eq!(
        recorder
            .0
            .iter()
            .filter(|v| *v == "reuse:shared.suite")
            .count(),
        1
    );
    assert_eq!(
        recorder
            .0
            .iter()
            .filter(|v| v.starts_with("check:"))
            .count(),
        3
    );
    assert_eq!(recorder.0.last().map(String::as_str), Some("finished"));
    let suite_starts = recorder
        .0
        .iter()
        .filter(|event| event.starts_with("start:"))
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        suite_starts,
        ["start:shared.suite", "start:suite.one", "start:suite.two"]
    );
    let report = serde_json::to_value(&outcome.report).unwrap();
    let root_group = report["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|group| group["id"] == "root")
        .unwrap();
    assert!(root_group.get("workingDirectory").is_none());
    let suite = report["suites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suite| suite["id"] == "suite.one")
        .unwrap();
    assert_eq!(suite["workingDirectory"], "one");
}

#[test]
fn direct_suite_selection_creates_no_group_result() {
    let temp = TempDir::new().unwrap();
    let tool = common::process_fixture();
    let value = common::base_config(tool);
    let mut value = value;
    value["checks"][0]["args"] = json!(["pass"]);
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, Some("fixture")).unwrap();
    assert!(matches!(
        plan.selection(),
        ValidationSelection::Suite { ref id } if id == "fixture"
    ));
    let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
    assert!(outcome.report.groups.is_empty());
    assert_eq!(outcome.report.suites.len(), 1);
    assert_eq!(outcome.report.checks.len(), 1);
}

#[test]
fn direct_check_uses_workspace_root_and_structured_selection() {
    let temp = TempDir::new().unwrap();
    let marker = temp.path().join("cwd");
    let tool = common::process_fixture();
    let mut value = common::base_config(tool);
    value["checks"][0]["args"] = json!(["write-cwd", marker, "{context}"]);
    value["suites"][0]["checks"][0]["parameters"] = json!({"context":"suite-only"});
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::check(&validated, "fixture.check").unwrap();
    let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
    assert!(outcome.report.groups.is_empty());
    assert!(outcome.report.suites.is_empty());
    assert_eq!(outcome.report.workspace_root, ".");
    assert!(matches!(
        outcome.report.checks[0].context,
        CheckContext::Direct
    ));
    assert!(!outcome.report.checks[0]
        .argv
        .iter()
        .any(|argument| argument == "suite-only"));
    assert_eq!(
        fs::read_to_string(marker).unwrap().trim(),
        temp.path().canonicalize().unwrap().display().to_string()
    );
}

#[test]
fn failed_check_skips_its_dependency_and_does_not_stop_independent_work() {
    let temp = TempDir::new().unwrap();
    let log = temp.path().join("runs.log");
    let tool = common::process_fixture();
    let value = json!({
      "schemaVersion":6,"workspaceRoot":".","defaultGroup":"all","outputLimitBytes":4096,
      "tools":[{"id":"fixture","program":tool,"requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}],
      "checks":[
       {"id":"first","label":"First","description":"Fails.","toolId":"fixture","args":["record-exit",log,"fail","7"],"requiresTools":[],"timeoutSeconds":5},
       {"id":"dependent","label":"Dependent","description":"Depends on first.","toolId":"fixture","args":["record-exit",log,"dependent","0"],"requiresTools":[],"timeoutSeconds":5},
       {"id":"independent","label":"Independent","description":"Still runs.","toolId":"fixture","args":["record-exit",log,"independent","0"],"requiresTools":[],"timeoutSeconds":5}],
      "suites":[{"id":"all.suite","label":"All","description":"All checks.","checks":[
        {"checkId":"first","dependsOn":[]},
        {"checkId":"dependent","dependsOn":["first"]},
        {"checkId":"independent","dependsOn":[]}
      ]}],
      "groups":[{"id":"all","label":"All","description":"All group.","members":[{"kind":"suite","id":"all.suite"}]}]
    });
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
    let statuses = outcome
        .report
        .checks
        .iter()
        .map(|result| (result.check_id.as_str(), result.status))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(statuses["first"], Status::Fail);
    assert_eq!(statuses["dependent"], Status::Skipped);
    assert_eq!(statuses["independent"], Status::Pass);
    assert_eq!(outcome.report.summary.result, OverallResult::Fail);
    assert_eq!(fs::read_to_string(log).unwrap(), "fail\nindependent\n");
}

#[test]
fn cancellation_interrupts_the_active_check_and_skips_remaining_work() {
    let temp = TempDir::new().unwrap();
    let tool = common::process_fixture();
    let value = json!({
      "schemaVersion":6,"workspaceRoot":".","defaultGroup":"all","outputLimitBytes":4096,
      "tools":[{"id":"fixture","program":tool,"requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}],
      "checks":[
       {"id":"active","label":"Active","description":"Is interrupted.","toolId":"fixture","args":["sleep","30000"],"requiresTools":[],"timeoutSeconds":60},
       {"id":"remaining","label":"Remaining","description":"Is skipped.","toolId":"fixture","args":["pass"],"requiresTools":[],"timeoutSeconds":60}],
      "suites":[{"id":"all.suite","label":"All","description":"All checks.","checks":[
        {"checkId":"active","dependsOn":[]},
        {"checkId":"remaining","dependsOn":[]}
      ]}],
      "groups":[{"id":"all","label":"All","description":"All group.","members":[{"kind":"suite","id":"all.suite"}]}]
    });
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = cancelled.clone();
    let trigger = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        signal.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    let outcome = execution::run(&validated, &plan, cancelled);
    trigger.join().unwrap();
    assert!(outcome.interrupted);
    assert_eq!(outcome.report.checks[0].status, Status::Fail);
    assert_eq!(
        outcome.report.checks[0].reason.as_deref(),
        Some("check interrupted")
    );
    assert_eq!(outcome.report.checks[1].status, Status::Skipped);
    assert_eq!(
        outcome.report.checks[1].reason.as_deref(),
        Some("execution interrupted")
    );
}
