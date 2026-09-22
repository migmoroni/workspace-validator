use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

pub fn base_config(program: impl AsRef<Path>) -> Value {
    json!({
        "schemaVersion":6,"workspaceRoot":".","defaultGroup":"all","outputLimitBytes":4096,
        "tools":[{"id":"fixture","program":program.as_ref(),"requiresTools":[],"versionArgs":["--version"],"versionParser":"firstSemver"}],
        "checks":[{"id":"fixture.check","label":"Fixture","description":"Runs the fixture.","toolId":"fixture","args":["run"],"requiresTools":[],"timeoutSeconds":5}],
        "suites":[{"id":"fixture","label":"Fixture","description":"All checks.","checks":[{"checkId":"fixture.check","dependsOn":[]}]}],
        "groups":[{"id":"all","label":"All","description":"All validation.","members":[{"kind":"suite","id":"fixture"}]}]
    })
}

pub fn write_config(root: &Path, value: &Value) -> std::path::PathBuf {
    let path = root.join("config.json");
    fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    path
}

#[allow(dead_code)]
pub fn process_fixture() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/process-tool");
            let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
                .args(["build", "--quiet", "--locked", "--manifest-path"])
                .arg(fixture.join("Cargo.toml"))
                .status()
                .expect("build process fixture");
            assert!(status.success(), "process fixture did not compile");
            fixture.join("target/debug").join(format!(
                "workspace-validator-process-fixture{}",
                std::env::consts::EXE_SUFFIX
            ))
        })
        .as_path()
}
