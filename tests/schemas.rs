use schemars::schema_for;
use serde_json::json;
use workspace_validator::contracts::{config::Config, report::ValidationReport};

#[test]
fn checked_in_schemas_match_rust_contracts() {
    let config = serde_json::to_value(schema_for!(Config)).unwrap();
    let report = serde_json::to_value(schema_for!(ValidationReport)).unwrap();
    assert_eq!(
        config,
        serde_json::from_str::<serde_json::Value>(include_str!("../schemas/config.schema.json"))
            .unwrap()
    );
    assert_eq!(
        report,
        serde_json::from_str::<serde_json::Value>(include_str!("../schemas/report.schema.json"))
            .unwrap()
    );
}

#[test]
fn config_schema_exposes_runtime_bounds_and_id_contracts() {
    let schema = serde_json::to_value(schema_for!(Config)).unwrap();
    assert_eq!(
        schema.pointer("/properties/schemaVersion/minimum"),
        Some(&json!(6))
    );
    assert_eq!(
        schema.pointer("/properties/schemaVersion/maximum"),
        Some(&json!(6))
    );
    assert_eq!(
        schema.pointer("/properties/outputLimitBytes/minimum"),
        Some(&json!(4096))
    );
    assert_eq!(
        schema.pointer("/properties/outputLimitBytes/maximum"),
        Some(&json!(16_777_216))
    );
    assert_eq!(
        schema.pointer("/$defs/CheckConfig/properties/timeoutSeconds/minimum"),
        Some(&json!(1))
    );
    assert_eq!(
        schema.pointer("/$defs/CheckConfig/properties/timeoutSeconds/maximum"),
        Some(&json!(86_400))
    );
    assert!(schema.pointer("/$defs/SuiteConfig/oneOf").is_none());
    assert!(schema.pointer("/properties/groups").is_some());
    assert!(schema.pointer("/properties/defaultGroup").is_some());
    assert_eq!(
        schema.pointer("/$defs/CheckConfig/properties/id/pattern"),
        Some(&json!(r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$"))
    );
    assert!(schema
        .pointer("/$defs/CheckConfig/properties/workingDirectory")
        .is_none());
    assert!(schema
        .pointer("/$defs/SuiteConfig/properties/workingDirectory")
        .is_some());
    assert!(schema
        .pointer("/$defs/GroupConfig/properties/workingDirectory")
        .is_none());
    assert_eq!(
        schema.pointer("/$defs/CheckConfig/properties/label/pattern"),
        Some(&json!(r"\S"))
    );
    assert_eq!(
        schema.pointer("/$defs/CheckConfig/properties/args/items/pattern"),
        Some(&json!(r"^[^\u0000]*$"))
    );
    assert!(schema
        .pointer("/$defs/CheckConfig/properties/dependsOn")
        .is_none());
    assert_eq!(
        schema.pointer("/$defs/SuiteCheckConfig/properties/checkId/pattern"),
        Some(&json!(r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$"))
    );
    assert!(schema
        .pointer("/$defs/SuiteCheckConfig/properties/args")
        .is_none());
    assert!(schema
        .pointer("/$defs/SuiteCheckConfig/properties/parameters")
        .is_some());
    assert!(schema.pointer("/$defs/ParameterValue/anyOf").is_some());
    assert!(schema
        .pointer("/$defs/SuiteCheckConfig/properties/dependsOn")
        .is_some());
    assert!(schema.pointer("/properties/repository/default").is_none());
    assert_eq!(
        schema["$defs"]["RepositoryProvider"]["enum"],
        json!(["git"])
    );
    assert_eq!(
        schema["$defs"]["VersionParser"]["enum"],
        json!(["firstSemver"])
    );
}

#[test]
fn report_schema_separates_group_suite_and_check_results() {
    let schema = serde_json::to_value(schema_for!(ValidationReport)).unwrap();
    assert_eq!(
        schema.pointer("/properties/schemaVersion/minimum"),
        Some(&json!(4))
    );
    assert_eq!(
        schema.pointer("/properties/schemaVersion/maximum"),
        Some(&json!(4))
    );
    assert!(schema
        .pointer("/$defs/SuiteResult/properties/workingDirectory")
        .is_some());
    assert!(schema
        .pointer("/$defs/GroupResult/properties/workingDirectory")
        .is_none());
    assert!(schema.pointer("/properties/groups").is_some());
    let suite_result_definitions = schema["$defs"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| name.ends_with("SuiteResult"))
        .collect::<Vec<_>>();
    assert_eq!(suite_result_definitions, ["SuiteResult"]);
    let required_check_fields = schema["$defs"]["CheckExecutionResult"]["required"]
        .as_array()
        .unwrap();
    assert!(required_check_fields.contains(&json!("stdout")));
    assert!(required_check_fields.contains(&json!("stderr")));
    assert_eq!(schema["$defs"]["Status"]["type"], json!("string"));
    assert_eq!(
        schema["$defs"]["Status"]["enum"],
        json!(["pass", "fail", "blocked", "skipped"])
    );
}

#[test]
fn configuration_contract_round_trips_without_loss() {
    let document = json!({
        "$schema": ".validation/config.schema.json",
        "schemaVersion": 6,
        "workspaceRoot": ".",
        "defaultGroup": "all",
        "outputLimitBytes": 4096,
        "repository": {
            "provider": "git",
            "toolId": "git",
            "detectMutations": true
        },
        "tools": [{
            "id": "cargo",
            "program": "cargo",
            "requiresTools": [],
            "versionArgs": ["--version"],
            "versionParser": "firstSemver",
            "versionRequirement": ">=1.87"
        }],
        "checks": [{
            "id": "rust.test",
            "label": "Rust tests",
            "description": "Runs one Cargo package.",
            "toolId": "cargo",
            "args": ["test", "-p", "{package}"],
            "requiresTools": [],
            "timeoutSeconds": 120
        }],
        "suites": [{
            "id": "rust",
            "label": "Rust",
            "description": "Validates one Rust package.",
            "workingDirectory": ".",
            "checks": [{
                "checkId": "rust.test",
                "parameters": {"package": "workspace-validator"},
                "dependsOn": []
            }]
        }],
        "groups": [{
            "id": "all",
            "label": "All",
            "description": "Runs every validation.",
            "members": [{"kind": "suite", "id": "rust"}]
        }]
    });
    let parsed: Config = serde_json::from_value(document).unwrap();
    let encoded = serde_json::to_value(parsed).unwrap();
    let decoded: Config = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
}

#[test]
fn report_contract_round_trips_without_loss() {
    let document = json!({
        "schemaVersion": 4,
        "selection": {"kind": "suite", "id": "rust"},
        "workspaceRoot": ".",
        "startedAtUnixMs": 1,
        "durationMs": 12,
        "tools": [{
            "id": "cargo",
            "program": "cargo",
            "argv": ["cargo", "--version"],
            "status": "pass",
            "version": "1.87.0"
        }],
        "groups": [],
        "suites": [{
            "id": "rust",
            "label": "Rust",
            "description": "Validates Rust.",
            "workingDirectory": ".",
            "checkDurationMs": 10,
            "summary": {"pass": 1, "fail": 0, "blocked": 0, "skipped": 0, "result": "pass"},
            "checkIds": ["rust.test"]
        }],
        "checks": [{
            "checkId": "rust.test",
            "context": {"kind": "suite", "suiteId": "rust"},
            "label": "Rust tests",
            "description": "Runs Rust tests.",
            "argv": ["cargo", "test"],
            "status": "pass",
            "exitCode": 0,
            "durationMs": 10,
            "timeoutSeconds": 120,
            "timedOut": false,
            "stdoutTruncated": false,
            "stderrTruncated": false
        }],
        "summary": {"pass": 1, "fail": 0, "blocked": 0, "skipped": 0, "result": "pass"}
    });
    let parsed: ValidationReport = serde_json::from_value(document).unwrap();
    let encoded = serde_json::to_value(parsed).unwrap();
    let decoded: ValidationReport = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
}
