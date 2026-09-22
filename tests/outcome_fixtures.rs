use std::{path::PathBuf, process::Command};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/outcomes")
}

fn validate(selection: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_workspace-validator"));
    command.arg("validate");
    if let Some(selection) = selection {
        command.arg(selection);
    }
    command.current_dir(fixture_root()).output().unwrap()
}

#[test]
#[ignore = "opt-in validation outcome fixture"]
fn opt_in_fixture_exposes_every_check_status_and_diagnostic() {
    let output = validate(None);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let report = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "CHECK PASS  outcome.pass",
        "CHECK FAIL  outcome.fail",
        "CHECK SKIPPED  outcome.skipped",
        "CHECK BLOCKED  outcome.blocked",
        "stdout:\n│       │\n│       │ running 1 test",
        "intentional workspace-validator fixture failure",
        "Reason: dependency outcome.fail did not pass",
        "Reason: required tool intentionally-unavailable is unavailable",
        "Summary: 1 pass · 1 fail · 1 blocked · 1 skipped",
    ] {
        assert!(
            report.contains(expected),
            "missing {expected:?} in:\n{report}"
        );
    }
}

#[test]
#[ignore = "opt-in validation outcome fixture"]
fn fixture_groups_preserve_their_documented_exit_codes() {
    let passing = validate(Some("outcomes.pass"));
    assert_eq!(passing.status.code(), Some(0));
    let passing_report = String::from_utf8(passing.stdout).unwrap();
    assert!(passing_report.contains("Result: Pass"));
    assert!(passing_report.contains("CHECK PASS  outcome.pass"));
    assert!(passing_report.contains("Summary: 1 check"));

    let failing = validate(Some("outcomes.fail"));
    assert_eq!(failing.status.code(), Some(1));
    assert!(String::from_utf8(failing.stdout)
        .unwrap()
        .contains("Summary: 1 fail · 1 skipped"));

    let blocked = validate(Some("outcomes.blocked"));
    assert_eq!(blocked.status.code(), Some(2));
    assert!(String::from_utf8(blocked.stdout)
        .unwrap()
        .contains("Summary: 1 blocked"));
}
