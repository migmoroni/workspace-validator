//! Version 4 validation report document types.
//!
//! Documented schema types override `schemars` metadata with empty values so
//! Rustdoc can evolve without changing the checked-in versioned JSON Schema.

use crate::contracts::config::{GroupMemberRef, RepositoryProvider};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

/// Validation report schema version emitted by this crate.
pub const REPORT_SCHEMA_VERSION: u32 = 4;

/// Outcome assigned to an individual tool, check, or integrity result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The operation completed successfully.
    Pass,
    /// The operation ran and found a validation failure.
    Fail,
    /// The operation could not run because a prerequisite was unavailable.
    Blocked,
    /// The operation was intentionally omitted, commonly after cancellation or dependency failure.
    Skipped,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(title = "", description = "", rename_all = "lowercase")]
enum StatusSchema {
    Pass,
    Fail,
    Blocked,
    Skipped,
}

impl JsonSchema for Status {
    fn schema_name() -> Cow<'static, str> {
        "Status".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        StatusSchema::json_schema(generator)
    }
}

/// Result of checking one declared tool during preflight.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Stable tool identifier from the configuration.
    pub id: String,
    /// Executable name or path requested from the operating system.
    pub program: String,
    /// Complete version-query command, including the executable.
    pub argv: Vec<String>,
    /// Outcome of tool discovery and version validation.
    pub status: Status,
    /// Parsed semantic version when discovery succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Diagnostic when discovery or validation does not pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// User selection that produced a validation report.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ValidationSelection {
    /// A group and every member reachable from it.
    Group {
        /// Selected group identifier.
        id: String,
    },
    /// One suite and its contextual check invocations.
    Suite {
        /// Selected suite identifier.
        id: String,
    },
    /// One direct check at the workspace root.
    Check {
        /// Selected check identifier.
        id: String,
    },
}

/// Execution context that gives a check result its stable identity.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CheckContext {
    /// Check invocation owned by a suite.
    Suite {
        /// Identifier of the owning suite.
        #[serde(rename = "suiteId")]
        suite_id: String,
    },
    /// Check invoked directly at the workspace root.
    Direct,
}

/// Complete observable result of one contextual check execution.
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct CheckExecutionResult {
    /// Stable check identifier from the configuration.
    pub check_id: String,
    /// Context that distinguishes direct and suite-bound invocations.
    pub context: CheckContext,
    /// Human-readable check name captured at execution time.
    pub label: String,
    /// Human-readable check description captured at execution time.
    pub description: String,
    /// Complete executed command, including the executable.
    pub argv: Vec<String>,
    /// Final operation status.
    pub status: Status,
    /// Process exit code when the operating system reports one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Elapsed execution time in milliseconds.
    pub duration_ms: u64,
    /// Configured timeout in seconds.
    pub timeout_seconds: u64,
    /// Whether timeout initiated process-tree termination.
    pub timed_out: bool,
    /// Whether captured standard output exceeded the configured byte limit.
    pub stdout_truncated: bool,
    /// Whether captured standard error exceeded the configured byte limit.
    pub stderr_truncated: bool,
    /// Captured standard output, omitted from JSON when empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    /// Captured standard error, omitted from JSON when empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    /// Diagnostic for failure, blocking, or omission when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl<'de> Deserialize<'de> for CheckExecutionResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Empty streams are omitted from emitted reports. Keeping that
        // compatibility rule out of the public type preserves the versioned
        // JSON Schema while still allowing emitted documents to round-trip.
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Document {
            check_id: String,
            context: CheckContext,
            label: String,
            description: String,
            argv: Vec<String>,
            status: Status,
            exit_code: Option<i32>,
            duration_ms: u64,
            timeout_seconds: u64,
            timed_out: bool,
            stdout_truncated: bool,
            stderr_truncated: bool,
            #[serde(default)]
            stdout: String,
            #[serde(default)]
            stderr: String,
            reason: Option<String>,
        }

        let document = Document::deserialize(deserializer)?;
        Ok(Self {
            check_id: document.check_id,
            context: document.context,
            label: document.label,
            description: document.description,
            argv: document.argv,
            status: document.status,
            exit_code: document.exit_code,
            duration_ms: document.duration_ms,
            timeout_seconds: document.timeout_seconds,
            timed_out: document.timed_out,
            stdout_truncated: document.stdout_truncated,
            stderr_truncated: document.stderr_truncated,
            stdout: document.stdout,
            stderr: document.stderr,
            reason: document.reason,
        })
    }
}

/// Aggregated result for one executable suite.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuiteResult {
    /// Stable suite identifier.
    pub id: String,
    /// Human-readable suite name.
    pub label: String,
    /// Human-readable suite description.
    pub description: String,
    /// Portable path relative to the workspace root.
    pub working_directory: String,
    /// Sum of concrete check durations in milliseconds.
    pub check_duration_ms: u64,
    /// Aggregate outcome of the suite's counted checks.
    pub summary: Summary,
    /// Ordered check identifiers belonging to the suite.
    pub check_ids: Vec<String>,
}

/// Aggregated result for one ordered group composition.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupResult {
    /// Stable group identifier.
    pub id: String,
    /// Human-readable group name.
    pub label: String,
    /// Human-readable group description.
    pub description: String,
    /// Derived hierarchy level, where leaf groups start at one.
    pub level: usize,
    /// Sum of distinct descendant check durations in milliseconds.
    pub check_duration_ms: u64,
    /// Aggregate outcome of distinct descendant checks.
    pub summary: Summary,
    /// Ordered references that compose the group.
    pub members: Vec<GroupMemberRef>,
}

/// Synthetic result describing repository mutation detection itself.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct RepositoryIntegrityResult {
    /// Synthetic result identifier.
    pub id: String,
    /// Human-readable integrity-check name.
    pub label: String,
    /// Outcome of repository capture and comparison.
    pub status: Status,
    /// Total repository capture and comparison time in milliseconds.
    pub duration_ms: u64,
    /// Diagnostic when integrity does not pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Repository snapshots and their classified differences.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct RepositoryReport {
    /// Repository provider used for this report.
    pub provider: RepositoryProvider,
    /// Counted synthetic integrity result.
    pub integrity: RepositoryIntegrityResult,
    /// Sorted repository entries captured before checks, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Vec<String>>,
    /// Sorted repository entries captured after checks, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<String>>,
    /// Entries introduced by validation, when comparison succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced: Option<Vec<String>>,
    /// Entries removed by validation, when comparison succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<Vec<String>>,
    /// Entries whose visible content changed, when comparison succeeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<Vec<String>>,
}

/// Overall validation result derived from every counted status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallResult {
    /// Every counted operation passed.
    Pass,
    /// At least one counted operation failed.
    Fail,
    /// No operation failed, but at least one was blocked or skipped.
    Blocked,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(title = "", description = "", rename_all = "lowercase")]
enum OverallResultSchema {
    Pass,
    Fail,
    Blocked,
}

impl JsonSchema for OverallResult {
    fn schema_name() -> Cow<'static, str> {
        "OverallResult".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        OverallResultSchema::json_schema(generator)
    }
}

/// Counts of concrete statuses together with their aggregate result.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// Number of passing counted operations.
    pub pass: usize,
    /// Number of failing counted operations.
    pub fail: usize,
    /// Number of blocked counted operations.
    pub blocked: usize,
    /// Number of skipped counted operations.
    pub skipped: usize,
    /// Aggregate result derived from the counts.
    pub result: OverallResult,
}

/// Portable version 4 document emitted after a validation run.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(title = "", description = "")]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    /// Version of the report contract.
    #[schemars(range(min = 4, max = 4))]
    pub schema_version: u32,
    /// User selection that produced this report.
    pub selection: ValidationSelection,
    /// Portable workspace marker, always `.` in version 4.
    #[schemars(schema_with = "dot_schema")]
    pub workspace_root: String,
    /// Unix timestamp in milliseconds captured at validation start.
    pub started_at_unix_ms: u64,
    /// Total validation duration in milliseconds.
    pub duration_ms: u64,
    /// Preflight results for every required tool.
    pub tools: Vec<ToolResult>,
    /// Aggregate results for reached groups in planning order.
    pub groups: Vec<GroupResult>,
    /// Aggregate results for reached suites in planning order.
    pub suites: Vec<SuiteResult>,
    /// Concrete check results in deterministic execution order.
    pub checks: Vec<CheckExecutionResult>,
    /// Repository integrity details when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryReport>,
    /// Aggregate result across concrete checks and repository integrity.
    pub summary: Summary,
}

fn dot_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"const": "."})
}

impl Summary {
    /// Counts statuses and derives the strongest aggregate outcome.
    pub fn from_statuses(statuses: impl IntoIterator<Item = Status>) -> Self {
        let mut counts = BTreeMap::from([
            (Status::Pass, 0),
            (Status::Fail, 0),
            (Status::Blocked, 0),
            (Status::Skipped, 0),
        ]);
        for status in statuses {
            *counts.get_mut(&status).expect("known status") += 1;
        }
        let fail = counts[&Status::Fail];
        let blocked = counts[&Status::Blocked];
        let skipped = counts[&Status::Skipped];
        Self {
            pass: counts[&Status::Pass],
            fail,
            blocked,
            skipped,
            result: if fail > 0 {
                OverallResult::Fail
            } else if blocked > 0 || skipped > 0 {
                OverallResult::Blocked
            } else {
                OverallResult::Pass
            },
        }
    }

    /// Maps the aggregate result to the process exit code used by the CLI.
    pub fn exit_code(&self) -> i32 {
        match self.result {
            OverallResult::Pass => 0,
            OverallResult::Fail => 1,
            OverallResult::Blocked => 2,
        }
    }
}
