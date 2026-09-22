mod common;
use serde_json::json;
use std::{
    process::Command,
    sync::{atomic::AtomicBool, Arc},
};
use tempfile::TempDir;
use workspace_validator::{config, contracts::report::Status, execution, planning, reporting};

#[test]
fn repository_integrity_is_typed_and_counted_once_outside_checks() {
    let temp = TempDir::new().unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(temp.path())
        .status()
        .unwrap()
        .success());
    let tool = common::process_fixture();
    let mut value = common::base_config(tool);
    value["checks"][0]["args"] = json!(["touch", "introduced"]);
    value["repository"] = json!({"provider":"git","toolId":"git","detectMutations":true});
    value["tools"].as_array_mut().unwrap().push(json!({"id":"git","program":"git","requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}));
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
    let repo = outcome.report.repository.as_ref().unwrap();
    assert_eq!(repo.integrity.status, Status::Fail);
    assert_eq!(repo.introduced.as_ref().unwrap(), &vec!["introduced"]);
    assert!(
        reporting::result::human::render(&outcome.report, &reporting::theme::Theme::plain())
            .contains("GATE")
    );
    assert!(outcome
        .report
        .checks
        .iter()
        .all(|v| v.check_id != "repository.integrity"));
    assert_eq!(
        outcome.report.summary.pass
            + outcome.report.summary.fail
            + outcome.report.summary.blocked
            + outcome.report.summary.skipped,
        outcome.report.checks.len() + 1
    );
}

#[test]
fn unavailable_repository_tool_still_produces_blocked_report() {
    let temp = TempDir::new().unwrap();
    let mut value = common::base_config("rustc");
    value["repository"] = json!({"provider":"git","toolId":"missing","detectMutations":true});
    value["tools"].as_array_mut().unwrap().push(json!({"id":"missing","program":"definitely-missing","requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}));
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
    let repo = outcome.report.repository.as_ref().unwrap();
    assert_eq!(repo.integrity.status, Status::Blocked);
    assert!(repo.before.is_none());
    assert!(repo.integrity.reason.is_some());
}
