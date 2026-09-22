//! Complete semantic validation and indexing of parsed configuration.

use crate::{
    config::{
        directories::{resolve_suite_directories, validate_relative_path},
        loader::ParsedConfig,
        parameters::placeholder_names,
    },
    contracts::config::*,
    dag::validate_graph,
    error::ValidatorError,
};
use semver::VersionReq;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use super::directories::ResolvedSuiteDirectory;

/// Fully validated configuration and indexes prepared for planning.
#[derive(Debug)]
pub struct ValidatedConfig {
    pub(crate) config: Config,
    pub(crate) path: PathBuf,
    pub(crate) workspace_root: PathBuf,
    pub(crate) tools: BTreeMap<String, ToolConfig>,
    pub(crate) checks: BTreeMap<String, CheckConfig>,
    pub(crate) suites: BTreeMap<String, SuiteConfig>,
    pub(crate) groups: BTreeMap<String, GroupConfig>,
    pub(crate) suite_directories: BTreeMap<String, ResolvedSuiteDirectory>,
}

impl ValidatedConfig {
    /// Returns the source configuration after complete semantic validation.
    pub fn configuration(&self) -> &Config {
        &self.config
    }

    /// Returns the canonical path of the loaded configuration document.
    pub fn configuration_path(&self) -> &Path {
        &self.path
    }

    /// Returns the canonical workspace root used to resolve suite directories.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns tools indexed by their stable identifiers.
    pub fn tools(&self) -> &BTreeMap<String, ToolConfig> {
        &self.tools
    }

    /// Returns checks indexed by their stable identifiers.
    pub fn checks(&self) -> &BTreeMap<String, CheckConfig> {
        &self.checks
    }

    /// Returns suites indexed by their stable identifiers.
    pub fn suites(&self) -> &BTreeMap<String, SuiteConfig> {
        &self.suites
    }

    /// Returns groups indexed by their stable identifiers.
    pub fn groups(&self) -> &BTreeMap<String, GroupConfig> {
        &self.groups
    }
}

/// Validates all semantic contracts and creates indexed runtime state.
pub(super) fn validate(parsed: ParsedConfig) -> Result<ValidatedConfig, ValidatorError> {
    let path = parsed.path;
    let config = parsed.config;
    let invalid = |details: Vec<String>| ValidatorError::invalid(&path, details.join("\n"));
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(invalid(vec![format!(
            "unsupported schemaVersion {}",
            config.schema_version
        )]));
    }
    let config_dir = path
        .parent()
        .ok_or_else(|| invalid(vec!["configuration has no parent".into()]))?;
    let workspace_root = config_dir
        .join(&config.workspace_root)
        .canonicalize()
        .map_err(|error| invalid(vec![format!("invalid workspaceRoot: {error}")]))?;
    if !workspace_root.is_dir() {
        return Err(invalid(vec!["workspaceRoot is not a directory".into()]));
    }

    let tools = collect_unique(config.tools.clone(), |value| &value.id, "tool", &path)?;
    let checks = collect_unique(config.checks.clone(), |value| &value.id, "check", &path)?;
    let suites = collect_unique(config.suites.clone(), |value| &value.id, "suite", &path)?;
    let groups = collect_unique(config.groups.clone(), |value| &value.id, "group", &path)?;
    let mut errors = Vec::new();

    if !(4096..=16_777_216).contains(&config.output_limit_bytes) {
        errors.push("outputLimitBytes must be between 4096 and 16777216".into());
    }
    if !groups.contains_key(&config.default_group) {
        errors.push(format!(
            "default group {} does not exist",
            config.default_group
        ));
    }
    for id in suites.keys() {
        if groups.contains_key(id) {
            errors.push(format!("group and suite IDs collide at {id}"));
        }
    }

    for tool in tools.values() {
        valid_id(&tool.id, &mut errors);
        if tool.program.is_empty()
            || has_nul(&tool.program)
            || tool.version_args.iter().any(|value| has_nul(value))
        {
            errors.push(format!(
                "tool {} has an empty program or NUL argument",
                tool.id
            ));
        }
        if let Some(requirement) = &tool.version_requirement {
            if let Err(error) = VersionReq::parse(requirement) {
                errors.push(format!(
                    "tool {} has invalid version requirement: {error}",
                    tool.id
                ));
            }
        }
    }

    for check in checks.values() {
        valid_id(&check.id, &mut errors);
        if check.id == "repository.integrity" {
            errors.push("repository.integrity is reserved".into());
        }
        validate_text(
            "check",
            &check.id,
            &check.label,
            &check.description,
            &mut errors,
        );
        if check.args.iter().any(|value| has_nul(value)) {
            errors.push(format!("check {} has a NUL argument", check.id));
        }
        if !(1..=86_400).contains(&check.timeout_seconds) {
            errors.push(format!("check {} has an invalid timeout", check.id));
        }
        if !tools.contains_key(&check.tool_id) {
            errors.push(format!(
                "check {} references missing tool {}",
                check.id, check.tool_id
            ));
        }
        for tool in &check.requires_tools {
            if !tools.contains_key(tool) {
                errors.push(format!("check {} references missing tool {tool}", check.id));
            }
        }
    }
    if let Some(repository) = &config.repository {
        if !tools.contains_key(&repository.tool_id) {
            errors.push(format!(
                "repository references missing tool {}",
                repository.tool_id
            ));
        }
    }

    for suite in suites.values() {
        valid_id(&suite.id, &mut errors);
        validate_text(
            "suite",
            &suite.id,
            &suite.label,
            &suite.description,
            &mut errors,
        );
        if suite.checks.is_empty() {
            errors.push(format!(
                "suite {} must contain at least one check",
                suite.id
            ));
        }
        let mut seen = BTreeSet::new();
        for invocation in &suite.checks {
            if !seen.insert(&invocation.check_id) {
                errors.push(format!(
                    "suite {} repeats check {}",
                    suite.id, invocation.check_id
                ));
            }
        }
        let positions: BTreeMap<_, _> = suite
            .checks
            .iter()
            .enumerate()
            .map(|(index, invocation)| (&invocation.check_id, index))
            .collect();
        for (index, invocation) in suite.checks.iter().enumerate() {
            match checks.get(&invocation.check_id) {
                None => errors.push(format!(
                    "suite {} references missing check {}",
                    suite.id, invocation.check_id
                )),
                Some(check) => {
                    let placeholders = placeholder_names(&check.args);
                    for (name, value) in &invocation.parameters {
                        valid_id(name, &mut errors);
                        if !placeholders.contains(name) {
                            errors.push(format!(
                                "suite {} check {} provides unknown parameter {name}",
                                suite.id, invocation.check_id
                            ));
                        }
                        if parameter_has_nul(value) {
                            errors.push(format!(
                                "suite {} check {} parameter {name} contains NUL",
                                suite.id, invocation.check_id
                            ));
                        }
                    }
                    for dependency in &invocation.depends_on {
                        if positions
                            .get(dependency)
                            .is_none_or(|dependency_index| *dependency_index >= index)
                        {
                            errors.push(format!(
                                "suite {} does not include dependency {dependency} before {}",
                                suite.id, invocation.check_id
                            ));
                        }
                    }
                }
            }
        }
        if let Some(directory) = &suite.working_directory {
            if let Err(error) = validate_relative_path(directory) {
                errors.push(format!("suite {}: {error}", suite.id));
            }
        }
    }

    for group in groups.values() {
        valid_id(&group.id, &mut errors);
        validate_text(
            "group",
            &group.id,
            &group.label,
            &group.description,
            &mut errors,
        );
        if group.members.is_empty() {
            errors.push(format!(
                "group {} must contain at least one member",
                group.id
            ));
        }
        let mut seen = BTreeSet::new();
        for member in &group.members {
            if !seen.insert(member.clone()) {
                errors.push(format!(
                    "group {} repeats {} member {}",
                    group.id,
                    member_kind(member),
                    member.id()
                ));
            }
            match member {
                GroupMemberRef::Group { id } if !groups.contains_key(id) => {
                    errors.push(format!("group {} references missing group {id}", group.id))
                }
                GroupMemberRef::Suite { id } if !suites.contains_key(id) => {
                    errors.push(format!("group {} references missing suite {id}", group.id))
                }
                _ => {}
            }
        }
    }

    let tool_ids = tools.keys().cloned().collect();
    let tool_graph = tools
        .iter()
        .map(|(id, value)| (id.clone(), value.requires_tools.clone()))
        .collect();
    if let Err(error) = validate_graph(&tool_ids, &tool_graph, "tool") {
        errors.push(error);
    }
    let group_ids = groups.keys().cloned().collect();
    let group_graph = groups
        .iter()
        .map(|(id, value)| {
            (
                id.clone(),
                value
                    .members
                    .iter()
                    .filter_map(|member| match member {
                        GroupMemberRef::Group { id } => Some(id.clone()),
                        GroupMemberRef::Suite { .. } => None,
                    })
                    .collect(),
            )
        })
        .collect();
    if let Err(error) = validate_graph(&group_ids, &group_graph, "group") {
        errors.push(error);
    }
    if !errors.is_empty() {
        return Err(invalid(errors));
    }

    let suite_directories = resolve_suite_directories(&suites, &workspace_root).map_err(invalid)?;
    Ok(ValidatedConfig {
        config,
        path,
        workspace_root,
        tools,
        checks,
        suites,
        groups,
        suite_directories,
    })
}

fn validate_text(kind: &str, id: &str, label: &str, description: &str, errors: &mut Vec<String>) {
    if label.trim().is_empty() || description.trim().is_empty() {
        errors.push(format!(
            "{kind} {id} must have non-blank label and description"
        ));
    }
}

fn member_kind(member: &GroupMemberRef) -> &'static str {
    match member {
        GroupMemberRef::Group { .. } => "group",
        GroupMemberRef::Suite { .. } => "suite",
    }
}

fn collect_unique<T: Clone>(
    values: Vec<T>,
    id: impl Fn(&T) -> &str,
    kind: &str,
    path: &Path,
) -> Result<BTreeMap<String, T>, ValidatorError> {
    let mut result = BTreeMap::new();
    for value in values {
        let key = id(&value).to_string();
        if result.insert(key.clone(), value).is_some() {
            return Err(ValidatorError::invalid(
                path,
                format!("duplicate {kind} id {key}"),
            ));
        }
    }
    Ok(result)
}

fn valid_id(id: &str, errors: &mut Vec<String>) {
    if id.is_empty()
        || id.len() > 96
        || !id.bytes().enumerate().all(|(index, byte)| {
            (index > 0 || byte.is_ascii_lowercase())
                && (byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-'))
        })
        || id.ends_with(['.', '_', '-'])
        || id.as_bytes().windows(2).any(|window| {
            matches!(window[0], b'.' | b'_' | b'-') && matches!(window[1], b'.' | b'_' | b'-')
        })
    {
        errors.push(format!("invalid id {id:?}"));
    }
}

fn has_nul(value: &str) -> bool {
    value.as_bytes().contains(&0)
}

fn parameter_has_nul(value: &ParameterValue) -> bool {
    match value {
        ParameterValue::Single(value) => has_nul(value),
        ParameterValue::Multiple(values) => values.iter().any(|value| has_nul(value)),
    }
}
