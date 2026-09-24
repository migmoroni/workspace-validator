use semver::{Version, VersionReq};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

const SKILL_NAME: &str = "workspace-validator";
const SKILL_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/skills/workspace-validator");

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.as_ref().display()))
}

fn manifest() -> Value {
    serde_json::from_str(&read(Path::new(SKILL_ROOT).join("manifest.json")))
        .expect("skill manifest must be valid JSON")
}

fn assert_safe_relative_path(path: &Path) {
    assert!(!path.as_os_str().is_empty(), "skill path must not be empty");
    assert!(!path.is_absolute(), "skill path must be relative");
    assert!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "skill path must not traverse directories: {}",
        path.display()
    );
}

fn assert_version_header(
    document: &str,
    crate_version: &str,
    bundle_version: u64,
    compatibility: &str,
) {
    assert!(document.contains(&format!("- Tool: `{SKILL_NAME}`")));
    assert!(document.contains(&format!("- Source crate version: `{crate_version}`")));
    assert!(document.contains(&format!("- Skill bundle version: `{bundle_version}`")));
    assert!(document.contains(&format!("- Compatible CLI: `{compatibility}`")));
    assert!(
        !document.contains("TODO"),
        "skill documents must be complete"
    );
}

#[test]
fn bundled_skill_matches_the_crate_and_routes_every_workflow() {
    let root = Path::new(SKILL_ROOT);
    let manifest = manifest();
    let crate_version = env!("CARGO_PKG_VERSION");

    assert_eq!(manifest["name"], SKILL_NAME);
    assert_eq!(manifest["sourceCrateVersion"], crate_version);

    let bundle_version = manifest["skillBundleVersion"]
        .as_u64()
        .expect("skillBundleVersion must be a positive integer");
    assert!(bundle_version > 0);

    let compatibility = manifest["compatibleCli"]
        .as_str()
        .expect("compatibleCli must be a SemVer requirement");
    let requirement =
        VersionReq::parse(compatibility).expect("compatibleCli must use valid SemVer syntax");
    assert!(requirement.matches(
        &Version::parse(crate_version).expect("the crate version must use valid SemVer syntax")
    ));

    let entrypoint = PathBuf::from(
        manifest["entrypoint"]
            .as_str()
            .expect("entrypoint must be a relative path"),
    );
    assert_safe_relative_path(&entrypoint);
    let entrypoint_document = read(root.join(&entrypoint));
    assert!(entrypoint_document.starts_with("---\nname: workspace-validator\ndescription:"));
    assert!(entrypoint_document.contains(&format!("- Tool: `{SKILL_NAME}`")));
    assert!(entrypoint_document.contains("`manifest.json`"));
    assert!(!entrypoint_document.contains("- Source crate version:"));
    assert!(!entrypoint_document.contains("- Skill bundle version:"));
    assert!(!entrypoint_document.contains("- Compatible CLI:"));

    let workflows = manifest["workflows"]
        .as_object()
        .expect("workflows must be an object");
    let workflow_names = workflows
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        workflow_names,
        BTreeSet::from(["audit", "config", "run", "triage"])
    );

    for (name, value) in workflows {
        let relative = PathBuf::from(
            value
                .as_str()
                .unwrap_or_else(|| panic!("workflow {name} must be a relative path")),
        );
        assert_safe_relative_path(&relative);
        assert!(
            entrypoint_document.contains(&format!("`{}`", relative.display())),
            "entrypoint must route workflow {name}"
        );
        assert_version_header(
            &read(root.join(relative)),
            crate_version,
            bundle_version,
            compatibility,
        );
    }
}
