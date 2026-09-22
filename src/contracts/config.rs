//! Version 6 configuration document types.
//!
//! Documented schema types override `schemars` metadata with empty values so
//! Rustdoc can evolve without changing the checked-in versioned JSON Schema.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap, path::PathBuf};

/// Configuration schema version accepted by this crate.
pub const CONFIG_SCHEMA_VERSION: u32 = 6;
const ID_PATTERN: &str = r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$";

/// Complete version 6 configuration document.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// Optional local JSON Schema reference used by editors.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    /// Version of the configuration contract.
    #[schemars(range(min = 6, max = 6))]
    pub schema_version: u32,
    /// Path from the configuration directory to the workspace root.
    pub workspace_root: PathBuf,
    /// Group selected when the CLI receives no explicit target.
    #[schemars(regex(pattern = ID_PATTERN))]
    pub default_group: String,
    /// Maximum captured bytes for each standard output stream.
    #[schemars(range(min = 4096, max = 16_777_216))]
    pub output_limit_bytes: usize,
    /// Optional repository-integrity provider.
    #[serde(default)]
    #[schemars(!default)]
    pub repository: Option<RepositoryConfig>,
    /// Executable tools available to checks and repository providers.
    pub tools: Vec<ToolConfig>,
    /// Reusable validation operations.
    pub checks: Vec<CheckConfig>,
    /// Contextual check compositions.
    pub suites: Vec<SuiteConfig>,
    /// Ordered compositions of suites and nested groups.
    pub groups: Vec<GroupConfig>,
}

/// Repository mutation detection configured for a validation run.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryConfig {
    /// Provider used to capture and compare repository state.
    pub provider: RepositoryProvider,
    /// Declared tool that implements the provider command.
    #[schemars(regex(pattern = ID_PATTERN))]
    pub tool_id: String,
    /// Whether Git-visible mutations make repository integrity fail.
    pub detect_mutations: bool,
}

/// Repository implementation supported by the versioned contract.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RepositoryProvider {
    /// Git status and content fingerprints.
    Git,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(title = "", description = "", rename_all = "camelCase")]
enum RepositoryProviderSchema {
    Git,
}

impl JsonSchema for RepositoryProvider {
    fn schema_name() -> Cow<'static, str> {
        "RepositoryProvider".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        RepositoryProviderSchema::json_schema(generator)
    }
}

/// Executable prerequisite available to checks and repository integration.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolConfig {
    /// Stable identifier referenced by checks and other tools.
    #[schemars(length(min = 1, max = 96), regex(pattern = ID_PATTERN))]
    pub id: String,
    /// Executable name or path passed directly to the operating system.
    #[schemars(length(min = 1), regex(pattern = r"^[^\u0000]*$"))]
    pub program: String,
    /// Tool prerequisites that must pass before this tool is checked.
    #[schemars(inner(regex(pattern = ID_PATTERN)))]
    pub requires_tools: Vec<String>,
    /// Arguments used to query the installed version.
    #[schemars(inner(regex(pattern = r"^[^\u0000]*$")))]
    pub version_args: Vec<String>,
    /// Parser applied to version-command output.
    pub version_parser: VersionParser,
    /// Optional semantic-version requirement for the detected version.
    #[serde(default)]
    pub version_requirement: Option<String>,
}

/// Strategy used to extract a version from prerequisite command output.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionParser {
    /// Selects the first semantic version found in command output.
    FirstSemver,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(title = "", description = "", rename_all = "camelCase")]
enum VersionParserSchema {
    FirstSemver,
}

impl JsonSchema for VersionParser {
    fn schema_name() -> Cow<'static, str> {
        "VersionParser".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        VersionParserSchema::json_schema(generator)
    }
}

/// One executable validation operation.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckConfig {
    /// Stable identifier used by suites and reports.
    #[schemars(length(min = 1, max = 96), regex(pattern = ID_PATTERN))]
    pub id: String,
    /// Human-readable check name.
    #[schemars(regex(pattern = r"\S"))]
    pub label: String,
    /// Human-readable explanation of the validation performed.
    #[schemars(regex(pattern = r"\S"))]
    pub description: String,
    /// Tool whose executable runs the check.
    #[schemars(regex(pattern = ID_PATTERN))]
    pub tool_id: String,
    /// Argument template passed directly to the executable without a shell.
    #[schemars(inner(regex(pattern = r"^[^\u0000]*$")))]
    pub args: Vec<String>,
    /// Additional tools required by this check.
    #[schemars(inner(regex(pattern = ID_PATTERN)))]
    pub requires_tools: Vec<String>,
    /// Maximum execution time in seconds.
    #[schemars(range(min = 1, max = 86_400))]
    pub timeout_seconds: u64,
}

/// One literal argument or argument sequence bound to a check placeholder.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(untagged)]
pub enum ParameterValue {
    /// Replaces one placeholder with one argument.
    Single(String),
    /// Expands one placeholder into multiple consecutive arguments.
    Multiple(Vec<String>),
}

/// One use of a reusable check within a suite.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuiteCheckConfig {
    /// Reusable check selected for this invocation.
    #[schemars(regex(pattern = ID_PATTERN))]
    pub check_id: String,
    /// Values bound to placeholders declared in the check arguments.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterValue>,
    /// Earlier checks in the same suite required by this invocation.
    #[schemars(inner(regex(pattern = ID_PATTERN)))]
    pub depends_on: Vec<String>,
}

/// Executable suite containing checks in one working directory.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuiteConfig {
    /// Stable identifier referenced by groups and the CLI.
    #[schemars(length(min = 1, max = 96), regex(pattern = ID_PATTERN))]
    pub id: String,
    /// Human-readable suite name.
    #[schemars(regex(pattern = r"\S"))]
    pub label: String,
    /// Human-readable explanation of the suite scope.
    #[schemars(regex(pattern = r"\S"))]
    pub description: String,
    /// Optional directory below the workspace root used for every check.
    #[serde(default)]
    pub working_directory: Option<PathBuf>,
    /// Ordered check invocations executed by the suite.
    #[schemars(length(min = 1))]
    pub checks: Vec<SuiteCheckConfig>,
}

/// Ordered composition of groups and executable suites.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupConfig {
    /// Stable identifier referenced by groups and the CLI.
    #[schemars(length(min = 1, max = 96), regex(pattern = ID_PATTERN))]
    pub id: String,
    /// Human-readable group name.
    #[schemars(regex(pattern = r"\S"))]
    pub label: String,
    /// Human-readable explanation of the grouped validation context.
    #[schemars(regex(pattern = r"\S"))]
    pub description: String,
    /// Ordered group or suite references.
    #[schemars(length(min = 1))]
    pub members: Vec<GroupMemberRef>,
}

/// Typed reference preserving the total order of members in a group.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[schemars(title = "", description = "")]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum GroupMemberRef {
    /// Reference to another group.
    Group {
        /// Referenced group identifier.
        #[schemars(regex(pattern = ID_PATTERN))]
        id: String,
    },
    /// Reference to an executable suite.
    Suite {
        /// Referenced suite identifier.
        #[schemars(regex(pattern = ID_PATTERN))]
        id: String,
    },
}

impl GroupMemberRef {
    /// Returns the referenced ID independently of its namespace.
    pub fn id(&self) -> &str {
        match self {
            Self::Group { id } | Self::Suite { id } => id,
        }
    }
}
