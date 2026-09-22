//! Read-only configuration inspection commands.

use crate::{
    config::{parameters, ValidatedConfig},
    contracts::config::GroupMemberRef,
    execution,
    planning::{self, ExecutionNodeRef, ValidationPlan},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    io::Write,
};

pub(super) fn config_validate(
    validated: &ValidatedConfig,
    output: &mut impl Write,
) -> Result<(), String> {
    let distinct = validated
        .suite_directories
        .values()
        .map(|directory| &directory.absolute)
        .collect::<BTreeSet<_>>()
        .len();
    writeln!(
        output,
        "Configuration: {}\nworkspaceRoot: {}\nTools: {}\nChecks: {}\nSuites: {}\nGroups: {}\nSuite directories: {}\nDefault group: {}",
        validated.path.display(),
        validated.workspace_root.display(),
        validated.tools.len(),
        validated.checks.len(),
        validated.suites.len(),
        validated.groups.len(),
        distinct,
        validated.config.default_group,
    )
    .map_err(|error| error.to_string())
}

pub(super) fn list(
    validated: &ValidatedConfig,
    tree: bool,
    output: &mut impl Write,
) -> Result<(), String> {
    writeln!(output, "Default group: {}", validated.config.default_group)
        .map_err(|error| error.to_string())?;
    writeln!(output, "Groups:").map_err(|error| error.to_string())?;
    let levels = planning::group_levels(validated).expect("healthy group graph");
    for group in validated.groups.values() {
        writeln!(
            output,
            "  GROUP N{} {} - {}",
            levels[&group.id], group.id, group.label
        )
        .map_err(|error| error.to_string())?;
    }
    writeln!(output, "Suites:").map_err(|error| error.to_string())?;
    for suite in validated.suites.values() {
        writeln!(
            output,
            "  SUITE {} - {} [{}]",
            suite.id, suite.label, validated.suite_directories[&suite.id].relative
        )
        .map_err(|error| error.to_string())?;
    }
    if tree {
        writeln!(output, "Tree:").map_err(|error| error.to_string())?;
        let plan = planning::group(validated, &validated.config.default_group)
            .expect("healthy default group");
        print_tree(
            &plan,
            ExecutionNodeRef::Group {
                id: validated.config.default_group.clone(),
            },
            output,
        )?;
    }
    writeln!(output, "Checks:").map_err(|error| error.to_string())?;
    for check in validated.checks.values() {
        writeln!(output, "  CHECK {} - {}", check.id, check.label)
            .map_err(|error| error.to_string())?;
    }
    writeln!(output, "Tools:").map_err(|error| error.to_string())?;
    for tool in validated.tools.values() {
        writeln!(output, "  TOOL {} - {}", tool.id, tool.program)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn explain_group(
    validated: &ValidatedConfig,
    id: &str,
    output: &mut impl Write,
) -> Result<(), String> {
    let plan = planning::group(validated, id).map_err(|error| error.to_string())?;
    let group = plan.groups.iter().find(|group| group.id == id).unwrap();
    writeln!(
        output,
        "GROUP {} - {}\n{}\nLevel: {}\nMembers:",
        id, group.label, group.description, group.level
    )
    .map_err(|error| error.to_string())?;
    for member in &group.members {
        writeln!(output, "  {} {}", member_kind(member), member.id())
            .map_err(|error| error.to_string())?;
    }
    writeln!(output, "Membership paths:").map_err(|error| error.to_string())?;
    print_membership_paths(validated, ExecutionNodeRef::Group { id: id.into() }, output)?;
    writeln!(output, "Reachable hierarchy:").map_err(|error| error.to_string())?;
    print_tree(&plan, ExecutionNodeRef::Group { id: id.into() }, output)?;
    print_commands_and_tools(validated, &plan, output)
}

pub(super) fn explain_suite(
    validated: &ValidatedConfig,
    id: &str,
    output: &mut impl Write,
) -> Result<(), String> {
    let plan = planning::suite(validated, id).map_err(|error| error.to_string())?;
    let suite = &plan.suites[0];
    let definition = &validated.suites[id];
    writeln!(
        output,
        "SUITE {} - {}\n{}\nDeclared directory: {}\nEffective directory: {}\nChecks:",
        id,
        suite.label,
        suite.description,
        definition
            .working_directory
            .as_ref()
            .map_or("<workspace root>".into(), |value| value
                .display()
                .to_string()),
        suite.relative_working_directory,
    )
    .map_err(|error| error.to_string())?;
    for invocation in &suite.checks {
        writeln!(output, "  {}", invocation.check_id).map_err(|error| error.to_string())?;
        if !invocation.parameters.is_empty() {
            let parameters =
                serde_json::to_string(&invocation.parameters).map_err(|error| error.to_string())?;
            writeln!(output, "    Parameters: {parameters}").map_err(|error| error.to_string())?;
        }
        if !invocation.depends_on.is_empty() {
            writeln!(output, "    Depends on: {:?}", invocation.depends_on)
                .map_err(|error| error.to_string())?;
        }
    }
    writeln!(output, "Membership paths:").map_err(|error| error.to_string())?;
    print_membership_paths(validated, ExecutionNodeRef::Suite { id: id.into() }, output)?;
    print_commands_and_tools(validated, &plan, output)
}

pub(super) fn explain_check(
    validated: &ValidatedConfig,
    id: &str,
    output: &mut impl Write,
) -> Result<(), String> {
    let check = validated
        .checks
        .get(id)
        .ok_or_else(|| format!("check {id} does not exist"))?;
    let direct_arguments = parameters::expand(&check.args, &Default::default());
    writeln!(
        output,
        "CHECK {} - {}\n{}\nTool: {}\nArgument template: {}\nDirect command: {}\nDirect directory: .\nTimeout: {} s\nSuites:",
        id,
        check.label,
        check.description,
        check.tool_id,
        command(&validated.tools[&check.tool_id].program, &check.args),
        command(
            &validated.tools[&check.tool_id].program,
            &direct_arguments
        ),
        check.timeout_seconds,
    )
    .map_err(|error| error.to_string())?;
    for suite in validated.suites.values().filter(|suite| {
        suite
            .checks
            .iter()
            .any(|invocation| invocation.check_id == id)
    }) {
        let invocation = suite
            .checks
            .iter()
            .find(|invocation| invocation.check_id == id)
            .expect("filtered suite invocation");
        let args = parameters::expand(&check.args, &invocation.parameters);
        writeln!(
            output,
            "  {} @ {}: {}",
            suite.id,
            validated.suite_directories[&suite.id].relative,
            command(&validated.tools[&check.tool_id].program, &args)
        )
        .map_err(|error| error.to_string())?;
        print_membership_paths(
            validated,
            ExecutionNodeRef::Suite {
                id: suite.id.clone(),
            },
            output,
        )?;
    }
    Ok(())
}

fn print_commands_and_tools(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    output: &mut impl Write,
) -> Result<(), String> {
    writeln!(output, "Commands:").map_err(|error| error.to_string())?;
    for suite in &plan.suites {
        for invocation in &suite.checks {
            let check = &validated.checks[&invocation.check_id];
            writeln!(
                output,
                "  {} / {} @ {}: {}",
                suite.id,
                invocation.check_id,
                suite.relative_working_directory,
                command(
                    &validated.tools[&check.tool_id].program,
                    &invocation.arguments
                )
            )
            .map_err(|error| error.to_string())?;
        }
    }
    writeln!(output, "Required tools:").map_err(|error| error.to_string())?;
    let check_ids = plan
        .check_executions
        .iter()
        .map(|execution| execution.check_id.clone())
        .collect::<Vec<_>>();
    for tool_id in execution::necessary_tool_ids(validated, &check_ids) {
        let tool = &validated.tools[&tool_id];
        let requirement = tool
            .version_requirement
            .as_deref()
            .map_or_else(String::new, |value| format!(" ({value})"));
        writeln!(output, "  {}: {}{}", tool.id, tool.program, requirement)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn print_tree(
    plan: &ValidationPlan,
    root: ExecutionNodeRef,
    output: &mut impl Write,
) -> Result<(), String> {
    let groups: BTreeMap<_, _> = plan
        .groups
        .iter()
        .map(|group| (group.id.as_str(), group))
        .collect();
    let suites: BTreeMap<_, _> = plan
        .suites
        .iter()
        .map(|suite| (suite.id.as_str(), suite))
        .collect();
    let mut seen_groups = BTreeSet::new();
    let mut seen_suites = BTreeSet::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((node, depth)) = stack.pop() {
        match node {
            ExecutionNodeRef::Group { id } => {
                let group = groups[&id.as_str()];
                let reused = !seen_groups.insert(id.clone());
                writeln!(
                    output,
                    "{}GROUP N{} {}{}",
                    tree_indent(depth),
                    group.level,
                    id,
                    if reused { " (shared)" } else { "" }
                )
                .map_err(|error| error.to_string())?;
                if !reused {
                    for member in group.members.iter().rev() {
                        stack.push((member_node(member), depth + 1));
                    }
                }
            }
            ExecutionNodeRef::Suite { id } => {
                let suite = suites[&id.as_str()];
                let reused = !seen_suites.insert(id.clone());
                writeln!(
                    output,
                    "{}SUITE {} [{}]{}",
                    tree_indent(depth),
                    id,
                    suite.relative_working_directory,
                    if reused { " (shared)" } else { "" }
                )
                .map_err(|error| error.to_string())?;
                if !reused {
                    for invocation in &suite.checks {
                        writeln!(
                            output,
                            "{}CHECK {}",
                            tree_indent(depth + 1),
                            invocation.check_id
                        )
                        .map_err(|error| error.to_string())?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn print_membership_paths(
    validated: &ValidatedConfig,
    target: ExecutionNodeRef,
    output: &mut impl Write,
) -> Result<(), String> {
    struct Frame {
        parents: Vec<String>,
        next_parent: usize,
        emitted_root: bool,
    }
    let mut reversed = vec![target.clone()];
    let mut stack = vec![Frame {
        parents: parents(validated, &target),
        next_parent: 0,
        emitted_root: false,
    }];
    while let Some(frame) = stack.last_mut() {
        if frame.parents.is_empty() {
            if !frame.emitted_root {
                let path = reversed
                    .iter()
                    .rev()
                    .map(|node| format!("{} {}", node_kind(node), node.id()))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                writeln!(output, "  {path}").map_err(|error| error.to_string())?;
                frame.emitted_root = true;
                continue;
            }
        } else if frame.next_parent < frame.parents.len() {
            let parent = frame.parents[frame.next_parent].clone();
            frame.next_parent += 1;
            let node = ExecutionNodeRef::Group { id: parent };
            reversed.push(node.clone());
            stack.push(Frame {
                parents: parents(validated, &node),
                next_parent: 0,
                emitted_root: false,
            });
            continue;
        }
        stack.pop();
        reversed.pop();
    }
    Ok(())
}

fn parents(validated: &ValidatedConfig, target: &ExecutionNodeRef) -> Vec<String> {
    validated
        .groups
        .values()
        .filter(|group| {
            group.members.iter().any(|member| match (member, target) {
                (GroupMemberRef::Group { id }, ExecutionNodeRef::Group { id: target })
                | (GroupMemberRef::Suite { id }, ExecutionNodeRef::Suite { id: target }) => {
                    id == target
                }
                _ => false,
            })
        })
        .map(|group| group.id.clone())
        .collect()
}

fn member_node(member: &GroupMemberRef) -> ExecutionNodeRef {
    match member {
        GroupMemberRef::Group { id } => ExecutionNodeRef::Group { id: id.clone() },
        GroupMemberRef::Suite { id } => ExecutionNodeRef::Suite { id: id.clone() },
    }
}

fn member_kind(member: &GroupMemberRef) -> &'static str {
    match member {
        GroupMemberRef::Group { .. } => "GROUP",
        GroupMemberRef::Suite { .. } => "SUITE",
    }
}

fn node_kind(node: &ExecutionNodeRef) -> &'static str {
    match node {
        ExecutionNodeRef::Group { .. } => "GROUP",
        ExecutionNodeRef::Suite { .. } => "SUITE",
    }
}

fn command(program: &str, args: &[String]) -> String {
    let mut rendered = format!("{program:?}");
    for argument in args {
        let _ = write!(rendered, " {argument:?}");
    }
    rendered
}

fn tree_indent(depth: usize) -> String {
    const MAX_VISIBLE_DEPTH: usize = 32;
    if depth <= MAX_VISIBLE_DEPTH {
        "  ".repeat(depth)
    } else {
        format!(
            "{}...(+{}) ",
            "  ".repeat(MAX_VISIBLE_DEPTH),
            depth - MAX_VISIBLE_DEPTH
        )
    }
}
