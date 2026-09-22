//! Immutable planning for group, suite, and direct-check selections.

use crate::{
    config::{parameters, ValidatedConfig},
    contracts::{
        config::{GroupMemberRef, ParameterValue},
        report::ValidationSelection,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
};

/// Stable identity for a direct or suite-contextual check execution.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CheckExecutionKey {
    /// A reusable check invoked inside a suite.
    Suite {
        /// Identifier of the suite that owns the invocation.
        suite_id: String,
        /// Identifier of the invoked check.
        check_id: String,
    },
    /// A check invoked directly at the workspace root.
    Direct {
        /// Identifier of the invoked check.
        check_id: String,
    },
}

/// Typed node carried by active execution paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionNodeRef {
    /// A group on the active execution path.
    Group {
        /// Stable group identifier.
        id: String,
    },
    /// A suite on the active execution path.
    Suite {
        /// Stable suite identifier.
        id: String,
    },
}

impl ExecutionNodeRef {
    /// Returns the referenced group or suite ID.
    pub fn id(&self) -> &str {
        match self {
            Self::Group { id } | Self::Suite { id } => id,
        }
    }
}

/// Group metadata and ordered members prepared for execution.
#[derive(Clone, Debug)]
pub struct PlannedGroup {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) members: Vec<GroupMemberRef>,
    pub(crate) level: usize,
}

impl PlannedGroup {
    /// Returns the stable group identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the human-readable group name.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the human-readable group description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns ordered group and suite references.
    pub fn members(&self) -> &[GroupMemberRef] {
        &self.members
    }

    /// Returns the derived hierarchy level, starting at one for leaf groups.
    pub fn level(&self) -> usize {
        self.level
    }
}

/// Executable suite metadata with a resolved working directory.
#[derive(Clone, Debug)]
pub struct PlannedSuite {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) checks: Vec<PlannedSuiteCheck>,
    pub(crate) working_directory: PathBuf,
    pub(crate) relative_working_directory: String,
}

impl PlannedSuite {
    /// Returns the stable suite identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the human-readable suite name.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the human-readable suite description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the ordered contextual check invocations.
    pub fn checks(&self) -> &[PlannedSuiteCheck] {
        &self.checks
    }

    /// Returns the canonical directory used to execute suite checks.
    pub fn working_directory(&self) -> &std::path::Path {
        &self.working_directory
    }

    /// Returns the portable workspace-relative execution directory.
    pub fn relative_working_directory(&self) -> &str {
        &self.relative_working_directory
    }
}

/// Resolved arguments, bindings, and dependencies for one reusable check.
#[derive(Clone, Debug)]
pub struct PlannedSuiteCheck {
    pub(crate) check_id: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) parameters: BTreeMap<String, ParameterValue>,
    pub(crate) depends_on: Vec<String>,
}

impl PlannedSuiteCheck {
    /// Returns the stable reusable check identifier.
    pub fn check_id(&self) -> &str {
        &self.check_id
    }

    /// Returns the fully expanded argument vector, excluding the executable.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Returns the parameter bindings used to expand the invocation.
    pub fn parameters(&self) -> &BTreeMap<String, ParameterValue> {
        &self.parameters
    }

    /// Returns earlier checks required by this invocation.
    pub fn dependencies(&self) -> &[String] {
        &self.depends_on
    }
}

/// One contextual check invocation in a validation plan.
#[derive(Clone, Debug)]
pub struct PlannedCheckExecution {
    pub(crate) key: CheckExecutionKey,
    pub(crate) check_id: String,
    pub(crate) suite_id: Option<String>,
}

impl PlannedCheckExecution {
    /// Returns the stable contextual identity of this invocation.
    pub fn key(&self) -> &CheckExecutionKey {
        &self.key
    }

    /// Returns the reusable check identifier.
    pub fn check_id(&self) -> &str {
        &self.check_id
    }

    /// Returns the owning suite identifier, or `None` for direct checks.
    pub fn suite_id(&self) -> Option<&str> {
        self.suite_id.as_deref()
    }
}

/// Immutable closure selected for one validation run.
#[derive(Clone, Debug)]
pub struct ValidationPlan {
    pub(crate) selection: ValidationSelection,
    pub(crate) groups: Vec<PlannedGroup>,
    pub(crate) suites: Vec<PlannedSuite>,
    pub(crate) check_executions: Vec<PlannedCheckExecution>,
    pub(crate) descendant_check_executions: BTreeMap<String, Vec<CheckExecutionKey>>,
    pub(crate) suite_check_executions: BTreeMap<String, Vec<CheckExecutionKey>>,
}

impl ValidationPlan {
    /// Returns the selection represented by this immutable plan.
    pub fn selection(&self) -> &ValidationSelection {
        &self.selection
    }

    /// Returns reached groups in deterministic planning order.
    pub fn groups(&self) -> &[PlannedGroup] {
        &self.groups
    }

    /// Returns reached suites in deterministic planning order.
    pub fn suites(&self) -> &[PlannedSuite] {
        &self.suites
    }

    /// Returns every concrete check invocation in deterministic order.
    pub fn check_executions(&self) -> &[PlannedCheckExecution] {
        &self.check_executions
    }
}

/// Failure to construct a valid execution plan from validated configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlanningError {
    /// Neither a group nor a suite matches the requested target.
    #[error("group or suite {id} does not exist")]
    TargetNotFound {
        /// Requested target identifier.
        id: String,
    },
    /// The requested group is not declared.
    #[error("group {id} does not exist")]
    GroupNotFound {
        /// Requested group identifier.
        id: String,
    },
    /// The requested suite is not declared.
    #[error("suite {id} does not exist")]
    SuiteNotFound {
        /// Requested suite identifier.
        id: String,
    },
    /// The requested direct check is not declared.
    #[error("check {id} does not exist")]
    CheckNotFound {
        /// Requested check identifier.
        id: String,
    },
    /// Validated group relationships could not be converted into levels.
    #[error("group levels could not be derived")]
    InconsistentGraph,
}

/// Plans the default group or an explicitly requested group or suite.
///
/// # Errors
///
/// Returns [`PlanningError::TargetNotFound`] when the requested identifier is
/// neither a group nor a suite. It can also return an inconsistent-graph error
/// if validated group relationships cannot be planned.
pub fn target(
    validated: &ValidatedConfig,
    requested: Option<&str>,
) -> Result<ValidationPlan, PlanningError> {
    let id = requested.unwrap_or(&validated.config.default_group);
    if validated.groups.contains_key(id) {
        group(validated, id)
    } else if validated.suites.contains_key(id) {
        suite(validated, id)
    } else {
        Err(PlanningError::TargetNotFound { id: id.into() })
    }
}

/// Plans an explicitly selected group and its reachable members.
///
/// # Errors
///
/// Returns [`PlanningError::GroupNotFound`] when `root` is not declared, or
/// [`PlanningError::InconsistentGraph`] when group levels cannot be derived.
pub fn group(validated: &ValidatedConfig, root: &str) -> Result<ValidationPlan, PlanningError> {
    if !validated.groups.contains_key(root) {
        return Err(PlanningError::GroupNotFound { id: root.into() });
    }
    let levels = group_levels(validated)?;
    let mut group_order = Vec::new();
    let mut suite_order = Vec::new();
    let mut seen_groups = BTreeSet::new();
    let mut seen_suites = BTreeSet::new();
    let mut stack = vec![GroupMemberRef::Group { id: root.into() }];
    while let Some(member) = stack.pop() {
        match member {
            GroupMemberRef::Group { id } => {
                if !seen_groups.insert(id.clone()) {
                    continue;
                }
                group_order.push(id.clone());
                for child in validated.groups[&id].members.iter().rev() {
                    stack.push(child.clone());
                }
            }
            GroupMemberRef::Suite { id } => {
                if seen_suites.insert(id.clone()) {
                    suite_order.push(id);
                }
            }
        }
    }
    build_plan(
        validated,
        ValidationSelection::Group { id: root.into() },
        group_order,
        suite_order,
        &levels,
    )
}

/// Plans an explicitly selected executable suite without creating a group.
///
/// # Errors
///
/// Returns [`PlanningError::SuiteNotFound`] when `id` is not declared.
pub fn suite(validated: &ValidatedConfig, id: &str) -> Result<ValidationPlan, PlanningError> {
    if !validated.suites.contains_key(id) {
        return Err(PlanningError::SuiteNotFound { id: id.into() });
    }
    build_plan(
        validated,
        ValidationSelection::Suite { id: id.into() },
        Vec::new(),
        vec![id.into()],
        &BTreeMap::new(),
    )
}

fn build_plan(
    validated: &ValidatedConfig,
    selection: ValidationSelection,
    group_order: Vec<String>,
    suite_order: Vec<String>,
    levels: &BTreeMap<String, usize>,
) -> Result<ValidationPlan, PlanningError> {
    let groups = group_order
        .iter()
        .map(|id| {
            let definition = &validated.groups[id];
            PlannedGroup {
                id: id.clone(),
                label: definition.label.clone(),
                description: definition.description.clone(),
                members: definition.members.clone(),
                level: levels[id],
            }
        })
        .collect();
    let suites = suite_order
        .iter()
        .map(|id| {
            let definition = &validated.suites[id];
            let directory = &validated.suite_directories[id];
            PlannedSuite {
                id: id.clone(),
                label: definition.label.clone(),
                description: definition.description.clone(),
                checks: definition
                    .checks
                    .iter()
                    .map(|invocation| PlannedSuiteCheck {
                        check_id: invocation.check_id.clone(),
                        arguments: parameters::expand(
                            &validated.checks[&invocation.check_id].args,
                            &invocation.parameters,
                        ),
                        parameters: invocation.parameters.clone(),
                        depends_on: invocation.depends_on.clone(),
                    })
                    .collect(),
                working_directory: directory.absolute.clone(),
                relative_working_directory: directory.relative.clone(),
            }
        })
        .collect::<Vec<_>>();
    let mut check_executions = Vec::new();
    let mut suite_check_executions = BTreeMap::new();
    for suite in &suites {
        let keys = suite
            .checks
            .iter()
            .map(|invocation| CheckExecutionKey::Suite {
                suite_id: suite.id.clone(),
                check_id: invocation.check_id.clone(),
            })
            .collect::<Vec<_>>();
        for (invocation, key) in suite.checks.iter().zip(keys.iter().cloned()) {
            check_executions.push(PlannedCheckExecution {
                key,
                check_id: invocation.check_id.clone(),
                suite_id: Some(suite.id.clone()),
            });
        }
        suite_check_executions.insert(suite.id.clone(), keys);
    }

    let mut descendant_check_executions = BTreeMap::new();
    for group_id in &group_order {
        let mut keys = Vec::new();
        let mut seen_keys = BTreeSet::new();
        let mut seen_groups = BTreeSet::new();
        let mut seen_suites = BTreeSet::new();
        let mut pending = vec![GroupMemberRef::Group {
            id: group_id.clone(),
        }];
        while let Some(member) = pending.pop() {
            match member {
                GroupMemberRef::Group { id } => {
                    if seen_groups.insert(id.clone()) {
                        for child in validated.groups[&id].members.iter().rev() {
                            pending.push(child.clone());
                        }
                    }
                }
                GroupMemberRef::Suite { id } => {
                    if seen_suites.insert(id.clone()) {
                        for key in &suite_check_executions[&id] {
                            if seen_keys.insert(key.clone()) {
                                keys.push(key.clone());
                            }
                        }
                    }
                }
            }
        }
        descendant_check_executions.insert(group_id.clone(), keys);
    }
    Ok(ValidationPlan {
        selection,
        groups,
        suites,
        check_executions,
        descendant_check_executions,
        suite_check_executions,
    })
}

/// Plans a direct check whose placeholders are omitted at the workspace root.
///
/// # Errors
///
/// Returns [`PlanningError::CheckNotFound`] when `id` is not declared.
pub fn check(validated: &ValidatedConfig, id: &str) -> Result<ValidationPlan, PlanningError> {
    if !validated.checks.contains_key(id) {
        return Err(PlanningError::CheckNotFound { id: id.into() });
    }
    let key = CheckExecutionKey::Direct {
        check_id: id.into(),
    };
    let check_executions = vec![PlannedCheckExecution {
        key,
        check_id: id.into(),
        suite_id: None,
    }];
    Ok(ValidationPlan {
        selection: ValidationSelection::Check { id: id.into() },
        groups: Vec::new(),
        suites: Vec::new(),
        check_executions,
        descendant_check_executions: BTreeMap::new(),
        suite_check_executions: BTreeMap::new(),
    })
}

/// Derives group levels from typed members without recursive traversal.
pub(crate) fn group_levels(
    validated: &ValidatedConfig,
) -> Result<BTreeMap<String, usize>, PlanningError> {
    let mut parents: BTreeMap<String, Vec<String>> = validated
        .groups
        .keys()
        .map(|id| (id.clone(), Vec::new()))
        .collect();
    let mut remaining = BTreeMap::new();
    let mut max_member_level = BTreeMap::new();
    let mut queue = VecDeque::new();
    for (id, group) in &validated.groups {
        let child_groups = group
            .members
            .iter()
            .filter_map(|member| match member {
                GroupMemberRef::Group { id } => Some(id),
                GroupMemberRef::Suite { .. } => None,
            })
            .collect::<Vec<_>>();
        remaining.insert(id.clone(), child_groups.len());
        max_member_level.insert(id.clone(), 0usize);
        if child_groups.is_empty() {
            queue.push_back(id.clone());
        }
        for child in child_groups {
            parents.get_mut(child).unwrap().push(id.clone());
        }
    }
    let mut result = BTreeMap::new();
    while let Some(id) = queue.pop_front() {
        let level = max_member_level[&id] + 1;
        result.insert(id.clone(), level);
        for parent in &parents[&id] {
            let current = max_member_level.get_mut(parent).unwrap();
            *current = (*current).max(level);
            let count = remaining.get_mut(parent).unwrap();
            *count -= 1;
            if *count == 0 {
                queue.push_back(parent.clone());
            }
        }
    }
    if result.len() != validated.groups.len() {
        return Err(PlanningError::InconsistentGraph);
    }
    Ok(result)
}
