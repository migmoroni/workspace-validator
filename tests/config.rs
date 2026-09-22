mod common;

use serde_json::json;
use std::fs;
use tempfile::TempDir;
use workspace_validator::{config, planning};

fn rejects(root: &TempDir, value: &serde_json::Value) -> bool {
    config::load(Some(&common::write_config(root.path(), value)), root.path()).is_err()
}

#[test]
fn validates_distinct_groups_suites_order_levels_and_directories() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("child")).unwrap();
    let mut value = common::base_config("rustc");
    value["defaultGroup"] = json!("root");
    value["suites"] = json!([
        {"id":"root.suite","label":"Root suite","description":"Runs at root.","checks":[{"checkId":"fixture.check","dependsOn":[]}]},
        {"id":"child.suite","label":"Child suite","description":"Runs in child.","workingDirectory":"child","checks":[{"checkId":"fixture.check","dependsOn":[]}]}
    ]);
    value["groups"] = json!([
        {"id":"shared","label":"Shared","description":"Shared group.","members":[{"kind":"suite","id":"child.suite"}]},
        {"id":"branch","label":"Branch","description":"Mixed branch.","members":[{"kind":"suite","id":"root.suite"},{"kind":"group","id":"shared"}]},
        {"id":"root","label":"Root","description":"Root group.","members":[{"kind":"group","id":"shared"},{"kind":"suite","id":"root.suite"},{"kind":"group","id":"branch"}]}
    ]);
    let loaded = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&loaded, None).unwrap();
    assert_eq!(
        plan.suites()
            .iter()
            .find(|suite| suite.id() == "root.suite")
            .unwrap()
            .relative_working_directory(),
        "."
    );
    assert_eq!(
        plan.suites()
            .iter()
            .find(|suite| suite.id() == "child.suite")
            .unwrap()
            .relative_working_directory(),
        "child"
    );
    assert_eq!(
        plan.groups()
            .iter()
            .find(|group| group.id() == "root")
            .unwrap()
            .level(),
        3
    );
    assert_eq!(
        plan.groups()
            .iter()
            .map(|group| group.id())
            .collect::<Vec<_>>(),
        ["root", "shared", "branch"]
    );
    assert_eq!(
        plan.suites()
            .iter()
            .map(|suite| suite.id())
            .collect::<Vec<_>>(),
        ["child.suite", "root.suite"]
    );
    assert_eq!(plan.check_executions().len(), 2);
}

#[test]
fn rejects_previous_schema_collisions_invalid_defaults_and_closed_shapes() {
    let temp = TempDir::new().unwrap();
    let mut previous = common::base_config("rustc");
    previous["schemaVersion"] = json!(5);
    assert!(rejects(&temp, &previous));

    let mut collision = common::base_config("rustc");
    collision["groups"][0]["id"] = json!("fixture");
    collision["defaultGroup"] = json!("fixture");
    assert!(rejects(&temp, &collision));

    let mut suite_default = common::base_config("rustc");
    suite_default["defaultGroup"] = json!("fixture");
    assert!(rejects(&temp, &suite_default));

    let mut group_check = common::base_config("rustc");
    group_check["groups"][0]["checks"] = json!(["fixture.check"]);
    assert!(rejects(&temp, &group_check));

    let mut group_directory = common::base_config("rustc");
    group_directory["groups"][0]["workingDirectory"] = json!(".");
    assert!(rejects(&temp, &group_directory));

    let mut suite_members = common::base_config("rustc");
    suite_members["suites"][0]["members"] = json!([]);
    assert!(rejects(&temp, &suite_members));

    let mut invalid_kind = common::base_config("rustc");
    invalid_kind["groups"][0]["members"][0]["kind"] = json!("check");
    assert!(rejects(&temp, &invalid_kind));

    let mut check_dependency = common::base_config("rustc");
    check_dependency["checks"][0]["dependsOn"] = json!([]);
    assert!(rejects(&temp, &check_dependency));

    let mut string_check_reference = common::base_config("rustc");
    string_check_reference["suites"][0]["checks"] = json!(["fixture.check"]);
    assert!(rejects(&temp, &string_check_reference));
}

#[test]
fn rejects_empty_repeated_missing_wrong_namespace_and_cyclic_groups() {
    let temp = TempDir::new().unwrap();
    let mut empty = common::base_config("rustc");
    empty["groups"][0]["members"] = json!([]);
    assert!(rejects(&temp, &empty));
    empty = common::base_config("rustc");
    empty["suites"][0]["checks"] = json!([]);
    assert!(rejects(&temp, &empty));

    let mut repeated = common::base_config("rustc");
    repeated["groups"][0]["members"] = json!([
        {"kind":"suite","id":"fixture"},
        {"kind":"suite","id":"fixture"}
    ]);
    assert!(rejects(&temp, &repeated));

    let mut wrong_namespace = common::base_config("rustc");
    wrong_namespace["groups"][0]["members"][0] = json!({"kind":"group","id":"fixture"});
    assert!(rejects(&temp, &wrong_namespace));

    let mut missing = common::base_config("rustc");
    missing["groups"][0]["members"][0] = json!({"kind":"suite","id":"missing"});
    assert!(rejects(&temp, &missing));

    let mut cycle = common::base_config("rustc");
    cycle["groups"] = json!([
        {"id":"all","label":"All","description":"All.","members":[{"kind":"group","id":"other"}]},
        {"id":"other","label":"Other","description":"Other.","members":[{"kind":"group","id":"all"}]}
    ]);
    assert!(rejects(&temp, &cycle));
}

#[test]
fn rejects_missing_late_check_dependency_and_reserved_id() {
    let temp = TempDir::new().unwrap();
    let mut value = common::base_config("rustc");
    value["checks"] = json!([
        {"id":"dependent","label":"Dependent","description":"Depends.","toolId":"fixture","args":[],"requiresTools":[],"timeoutSeconds":5},
        {"id":"base","label":"Base","description":"Base.","toolId":"fixture","args":[],"requiresTools":[],"timeoutSeconds":5}
    ]);
    value["suites"][0]["checks"] = json!([
        {"checkId":"dependent","dependsOn":["base"]},
        {"checkId":"base","dependsOn":[]}
    ]);
    assert!(rejects(&temp, &value));
    value["checks"][1]["id"] = json!("repository.integrity");
    value["suites"][0]["checks"] = json!([
        {"checkId":"repository.integrity","dependsOn":[]},
        {"checkId":"dependent","dependsOn":["repository.integrity"]}
    ]);
    assert!(rejects(&temp, &value));
}

#[test]
fn rejects_repeated_checks_and_invalid_suite_parameters() {
    let temp = TempDir::new().unwrap();
    let mut repeated = common::base_config("rustc");
    repeated["suites"][0]["checks"] = json!([
        {"checkId":"fixture.check","dependsOn":[]},
        {"checkId":"fixture.check","dependsOn":[]}
    ]);
    assert!(rejects(&temp, &repeated));

    let mut unknown_parameter = common::base_config("rustc");
    unknown_parameter["suites"][0]["checks"][0]["parameters"] = json!({"unknown":"value"});
    assert!(rejects(&temp, &unknown_parameter));

    let mut nul_parameter = common::base_config("rustc");
    nul_parameter["checks"][0]["args"] = json!(["run", "{target}"]);
    nul_parameter["suites"][0]["checks"][0]["parameters"] = json!({"target":["bad\0argument"]});
    assert!(rejects(&temp, &nul_parameter));

    let mut missing_dependency = common::base_config("rustc");
    missing_dependency["suites"][0]["checks"][0]["dependsOn"] = json!(["missing"]);
    assert!(rejects(&temp, &missing_dependency));
}

#[test]
fn resolves_inferred_parameters_at_exact_argument_positions() {
    let temp = TempDir::new().unwrap();
    let mut value = common::base_config("rustc");
    value["checks"][0]["args"] = json!([
        "run",
        "{single}",
        "middle",
        "{multiple}",
        "{absent}",
        "{single}",
        "--literal={single}"
    ]);
    value["suites"][0]["checks"][0]["parameters"] = json!({
        "single": "one argument",
        "multiple": ["two", "{single}"]
    });

    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::suite(&validated, "fixture").unwrap();

    assert_eq!(
        plan.suites()[0].checks()[0].arguments(),
        [
            "run",
            "one argument",
            "middle",
            "two",
            "{single}",
            "one argument",
            "--literal={single}"
        ]
    );
}

#[test]
fn rejects_unsafe_missing_file_and_external_symlink_suite_directories() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file"), "not a directory").unwrap();
    for invalid in [
        json!(""),
        json!("missing"),
        json!("file"),
        json!("../outside"),
        json!(temp.path()),
    ] {
        let mut value = common::base_config("rustc");
        value["suites"][0]["workingDirectory"] = invalid;
        assert!(rejects(&temp, &value));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = TempDir::new().unwrap();
        symlink(outside.path(), temp.path().join("outside-link")).unwrap();
        let mut value = common::base_config("rustc");
        value["suites"][0]["workingDirectory"] = json!(std::path::Path::new("outside-link"));
        assert!(rejects(&temp, &value));
    }
}

#[test]
fn plans_each_group_and_suite_once_in_a_convergent_deep_dag() {
    let temp = TempDir::new().unwrap();
    let mut value = common::base_config("rustc");
    value["defaultGroup"] = json!("root");
    let depth = 32;
    let mut groups = Vec::new();
    for level in (0..depth).rev() {
        let members = if level + 1 == depth {
            json!([{"kind":"suite","id":"fixture"}])
        } else {
            json!([
                {"kind":"group","id":format!("layer{}.a", level + 1)},
                {"kind":"group","id":format!("layer{}.b", level + 1)}
            ])
        };
        groups.push(json!({"id":format!("layer{level}.a"),"label":"A","description":"A.","members":members}));
        groups.push(json!({"id":format!("layer{level}.b"),"label":"B","description":"B.","members":members}));
    }
    groups.push(json!({"id":"root","label":"Root","description":"Root.","members":[{"kind":"group","id":"layer0.a"},{"kind":"group","id":"layer0.b"}]}));
    value["groups"] = serde_json::Value::Array(groups);
    let validated = config::load(
        Some(&common::write_config(temp.path(), &value)),
        temp.path(),
    )
    .unwrap();
    let plan = planning::target(&validated, None).unwrap();
    assert_eq!(plan.groups().len(), 2 * depth + 1);
    assert_eq!(plan.suites().len(), 1);
    assert_eq!(plan.check_executions().len(), 1);
}
