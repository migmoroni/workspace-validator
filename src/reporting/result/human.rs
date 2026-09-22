//! Human rendering of immutable completed validation reports.

use crate::contracts::{
    config::GroupMemberRef,
    report::{CheckContext, CheckExecutionResult, Status, ValidationReport, ValidationSelection},
};
use crate::reporting::format::{format_duration, format_selection, format_summary, status_label};
use crate::reporting::theme::{Role, Theme};
use crate::reporting::tree;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

/// Renders a completed report with semantic styles from the selected theme.
pub fn render(report: &ValidationReport, theme: &Theme) -> String {
    let mut output = String::new();
    let border = vertical_border(theme);
    let _ = writeln!(
        output,
        "{}",
        theme.frame_rule('╭', "Workspace Validator · Result", Role::Heading)
    );
    write_section_gap(&mut output, theme);
    let mut metadata = ItemSpacing::default();
    metadata.before_item(&mut output, theme);
    let _ = writeln!(
        output,
        "{border} {} {}",
        theme.paint(Role::Metadata, "Selection:"),
        format_selection(&report.selection)
    );
    metadata.before_item(&mut output, theme);
    let _ = writeln!(
        output,
        "{border} {} {:?}",
        theme.paint(Role::Metadata, "Result:"),
        report.summary.result
    );
    metadata.before_item(&mut output, theme);
    let _ = writeln!(
        output,
        "{border} {} {}",
        theme.paint(Role::Metadata, "Duration:"),
        theme.paint(Role::Duration, format_duration(report.duration_ms))
    );
    write_section_gap(&mut output, theme);
    let _ = writeln!(output, "{}", theme.frame_rule('├', "Tools", Role::Section));
    write_section_gap(&mut output, theme);
    let mut tools = ItemSpacing::default();
    for tool in &report.tools {
        tools.before_item(&mut output, theme);
        let _ = writeln!(
            output,
            "{border} {}  {}  {}{}",
            theme.paint(Role::Tool, "TOOL"),
            painted_status(theme, tool.status),
            tool.id,
            tool.version
                .as_deref()
                .map_or_else(String::new, |version| format!(" ({version})"))
        );
        if let Some(reason) = &tool.reason {
            let _ = writeln!(
                output,
                "{border}       {}",
                theme.paint(Role::DiagnosticContent, reason)
            );
        }
    }
    write_section_gap(&mut output, theme);
    let _ = writeln!(
        output,
        "{}",
        theme.frame_rule('├', "Execution tree", Role::Section)
    );
    write_section_gap(&mut output, theme);
    render_tree(report, theme, &mut output);
    write_section_gap(&mut output, theme);
    let _ = writeln!(
        output,
        "{}",
        theme.frame_rule('├', "Counted outcomes", Role::Section)
    );
    write_section_gap(&mut output, theme);
    let mut outcomes = ItemSpacing::default();
    for check in &report.checks {
        outcomes.before_item(&mut output, theme);
        let _ = writeln!(
            output,
            "{border} {} {}  {}  {}",
            theme.paint(Role::Check, "CHECK"),
            painted_status(theme, check.status),
            check.check_id,
            theme.paint(Role::Duration, format_duration(check.duration_ms))
        );
        render_failure(check, theme, &mut output);
    }
    if let Some(repository) = &report.repository {
        outcomes.before_item(&mut output, theme);
        let gate = &repository.integrity;
        let _ = writeln!(
            output,
            "{border} {}  {}  {}  {}",
            theme.paint(Role::Gate, "GATE"),
            painted_status(theme, gate.status),
            gate.id,
            theme.paint(Role::Duration, format_duration(gate.duration_ms))
        );
        if let Some(reason) = &gate.reason {
            let _ = writeln!(
                output,
                "{border}       {}",
                theme.paint(Role::DiagnosticContent, reason)
            );
        }
    }
    write_section_gap(&mut output, theme);
    let gate_count = usize::from(report.repository.is_some());
    let mut summary = ItemSpacing::default();
    summary.before_item(&mut output, theme);
    let _ = writeln!(
        output,
        "{border} Counted: {} checks + {} repository gate",
        report.checks.len(),
        gate_count
    );
    summary.before_item(&mut output, theme);
    let _ = writeln!(
        output,
        "{border} {} {}",
        theme.paint(Role::Metadata, "Summary:"),
        format_summary(&report.summary)
    );
    write_section_gap(&mut output, theme);
    let _ = writeln!(
        output,
        "{}",
        theme.frame_rule('╰', "Validation complete", Role::Border)
    );
    output
}

fn render_tree(report: &ValidationReport, theme: &Theme, output: &mut String) {
    let mut items = TreeSpacing::default();
    match &report.selection {
        ValidationSelection::Check { id } => {
            if let Some(check) = report.checks.iter().find(|check| check.check_id == *id) {
                render_check_line(check, &[], theme, output, &mut items);
            }
        }
        ValidationSelection::Suite { id } => {
            render_suite(report, id, &[], theme, output, &mut items)
        }
        ValidationSelection::Group { id } => {
            render_group_tree(report, id, theme, output, &mut items)
        }
    }
}

fn render_group_tree(
    report: &ValidationReport,
    root: &str,
    theme: &Theme,
    output: &mut String,
    items: &mut TreeSpacing,
) {
    enum Node {
        Group(String, Vec<bool>),
        Suite(String, Vec<bool>),
    }
    let groups: BTreeMap<_, _> = report
        .groups
        .iter()
        .map(|group| (group.id.as_str(), group))
        .collect();
    let mut seen_groups = BTreeSet::new();
    let mut seen_suites = BTreeSet::new();
    let mut stack = vec![Node::Group(root.into(), Vec::new())];
    while let Some(node) = stack.pop() {
        match node {
            Node::Group(id, branches) => {
                let group = groups[&id.as_str()];
                let reused = !seen_groups.insert(id.clone());
                items.before_item(output, theme, &branches);
                let _ = writeln!(
                    output,
                    "{}{} {}  {}  {}{}",
                    theme.paint(Role::Border, tree::prefix(&branches)),
                    theme.paint(Role::Group, "GROUP"),
                    painted_status(theme, status_from_summary(&group.summary)),
                    id,
                    theme.paint(Role::Duration, format_duration(group.check_duration_ms)),
                    if reused {
                        format!("  {}", theme.paint(Role::Shared, "(shared)"))
                    } else {
                        String::new()
                    }
                );
                if !reused {
                    let member_count = group.members.len();
                    for (position, member) in group.members.iter().enumerate().rev() {
                        let mut child_branches = branches.clone();
                        child_branches.push(position + 1 == member_count);
                        match member {
                            GroupMemberRef::Group { id } => {
                                stack.push(Node::Group(id.clone(), child_branches))
                            }
                            GroupMemberRef::Suite { id } => {
                                stack.push(Node::Suite(id.clone(), child_branches))
                            }
                        }
                    }
                }
            }
            Node::Suite(id, branches) => {
                let reused = !seen_suites.insert(id.clone());
                render_suite_header(report, &id, &branches, reused, theme, output, items);
                if !reused {
                    render_suite_checks(report, &id, &branches, theme, output, items);
                }
            }
        }
    }
}

fn render_suite(
    report: &ValidationReport,
    id: &str,
    branches: &[bool],
    theme: &Theme,
    output: &mut String,
    items: &mut TreeSpacing,
) {
    render_suite_header(report, id, branches, false, theme, output, items);
    render_suite_checks(report, id, branches, theme, output, items);
}

fn render_suite_header(
    report: &ValidationReport,
    id: &str,
    branches: &[bool],
    reused: bool,
    theme: &Theme,
    output: &mut String,
    items: &mut TreeSpacing,
) {
    let suite = report
        .suites
        .iter()
        .find(|suite| suite.id == id)
        .expect("reported suite");
    items.before_item(output, theme, branches);
    let _ = writeln!(
        output,
        "{}{} {}  {}  {}  [{}]{}",
        theme.paint(Role::Border, tree::prefix(branches)),
        theme.paint(Role::Suite, "SUITE"),
        painted_status(theme, status_from_summary(&suite.summary)),
        suite.id,
        theme.paint(Role::Duration, format_duration(suite.check_duration_ms)),
        theme.paint(Role::Path, &suite.working_directory),
        if reused {
            format!("  {}", theme.paint(Role::Shared, "(shared)"))
        } else {
            String::new()
        }
    );
}

fn render_suite_checks(
    report: &ValidationReport,
    suite_id: &str,
    parent_branches: &[bool],
    theme: &Theme,
    output: &mut String,
    items: &mut TreeSpacing,
) {
    let checks = report
        .checks
        .iter()
        .filter(
            |check| matches!(&check.context, CheckContext::Suite { suite_id: id } if id == suite_id),
        )
        .collect::<Vec<_>>();
    let check_count = checks.len();
    for (position, check) in checks.into_iter().enumerate() {
        let mut branches = parent_branches.to_vec();
        branches.push(position + 1 == check_count);
        render_check_line(check, &branches, theme, output, items);
    }
}

fn render_check_line(
    check: &CheckExecutionResult,
    branches: &[bool],
    theme: &Theme,
    output: &mut String,
    items: &mut TreeSpacing,
) {
    items.before_item(output, theme, branches);
    let _ = writeln!(
        output,
        "{}{} {}  {}  {}",
        theme.paint(Role::Border, tree::prefix(branches)),
        theme.paint(Role::Check, "CHECK"),
        painted_status(theme, check.status),
        check.check_id,
        theme.paint(Role::Duration, format_duration(check.duration_ms))
    );
}

fn render_failure(check: &CheckExecutionResult, theme: &Theme, output: &mut String) {
    if check.status == Status::Pass {
        return;
    }
    if let Some(reason) = &check.reason {
        render_diagnostic("Reason:", reason, theme, output);
    }
    if !check.stderr.is_empty() {
        render_diagnostic(
            if check.stderr_truncated {
                "stderr (tail, truncated):"
            } else {
                "stderr:"
            },
            &check.stderr,
            theme,
            output,
        );
    }
    if !check.stdout.is_empty() {
        render_diagnostic(
            if check.stdout_truncated {
                "stdout (tail, truncated):"
            } else {
                "stdout:"
            },
            &check.stdout,
            theme,
            output,
        );
    }
}

/// Keeps arbitrary subprocess output inside the report frame. Compact values
/// stay beside their label, while multiline values receive a nested guide so
/// embedded line breaks cannot escape the surrounding result structure.
fn render_diagnostic(label: &str, content: &str, theme: &Theme, output: &mut String) {
    let content = content.trim_end_matches(['\r', '\n']);
    let mut lines = content
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let first = lines.next().unwrap_or_default();
    let Some(second) = lines.next() else {
        let _ = writeln!(
            output,
            "{}       {} {}",
            vertical_border(theme),
            theme.paint(Role::DiagnosticLabel, label),
            theme.paint(Role::DiagnosticContent, first)
        );
        return;
    };

    let _ = writeln!(
        output,
        "{}       {}",
        vertical_border(theme),
        theme.paint(Role::DiagnosticLabel, label)
    );
    for line in std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(lines)
    {
        let border = vertical_border(theme);
        let guide = theme.paint(Role::Border, "│");
        if line.is_empty() {
            let _ = writeln!(output, "{border}       {guide}");
        } else {
            let _ = writeln!(
                output,
                "{border}       {guide} {}",
                theme.paint(Role::DiagnosticContent, line)
            );
        }
    }
}

fn status_from_summary(summary: &crate::contracts::report::Summary) -> Status {
    if summary.fail > 0 {
        Status::Fail
    } else if summary.blocked > 0 || summary.skipped > 0 {
        Status::Blocked
    } else {
        Status::Pass
    }
}

fn painted_status(theme: &Theme, status: Status) -> String {
    let role = match status {
        Status::Pass => Role::Success,
        Status::Fail => Role::Failure,
        Status::Blocked => Role::Blocked,
        Status::Skipped => Role::Skipped,
    };
    theme.paint(role, status_label(status))
}

fn vertical_border(theme: &Theme) -> String {
    theme.paint(Role::Border, "│")
}

#[derive(Default)]
struct ItemSpacing {
    has_previous: bool,
}

impl ItemSpacing {
    fn before_item(&mut self, output: &mut String, theme: &Theme) {
        if self.has_previous {
            write_gap(output, theme, theme.item_gap());
        }
        self.has_previous = true;
    }
}

#[derive(Default)]
struct TreeSpacing {
    has_previous: bool,
}

impl TreeSpacing {
    fn before_item(&mut self, output: &mut String, theme: &Theme, branches: &[bool]) {
        if self.has_previous {
            write_tree_gap(output, theme, theme.item_gap(), branches);
        }
        self.has_previous = true;
    }
}

fn write_section_gap(output: &mut String, theme: &Theme) {
    write_gap(output, theme, theme.section_gap());
}

fn write_gap(output: &mut String, theme: &Theme, size: usize) {
    for _ in 0..size {
        let _ = writeln!(output, "{}", vertical_border(theme));
    }
}

fn write_tree_gap(output: &mut String, theme: &Theme, size: usize, branches: &[bool]) {
    let continuation = theme.paint(Role::Border, tree::continuation(branches));
    for _ in 0..size {
        let _ = writeln!(output, "{continuation}");
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::contracts::{
        config::GroupMemberRef,
        report::{
            CheckContext, CheckExecutionResult, GroupResult, Status, SuiteResult, Summary,
            ToolResult, ValidationReport, ValidationSelection, REPORT_SCHEMA_VERSION,
        },
    };
    use crate::reporting::theme::{PaletteProfile, PresentationProfile, Theme};
    use console::{measure_text_width, strip_ansi_codes};

    #[test]
    fn completed_tree_marks_shared_suites_and_counts_only_checks() {
        let passing = Summary::from_statuses([Status::Pass]);
        let group = |id: &str, members| GroupResult {
            id: id.into(),
            label: id.into(),
            description: format!("{id} group"),
            level: usize::from(id == "root") + 1,
            check_duration_ms: 25,
            summary: passing.clone(),
            members,
        };
        let report = ValidationReport {
            schema_version: REPORT_SCHEMA_VERSION,
            selection: ValidationSelection::Group { id: "root".into() },
            workspace_root: ".".into(),
            started_at_unix_ms: 1,
            duration_ms: 25,
            tools: vec![ToolResult {
                id: "tool".into(),
                program: "tool".into(),
                argv: vec!["--version".into()],
                status: Status::Pass,
                version: Some("1.0.0".into()),
                reason: None,
            }],
            groups: vec![
                group(
                    "root",
                    vec![
                        GroupMemberRef::Group { id: "left".into() },
                        GroupMemberRef::Group { id: "right".into() },
                    ],
                ),
                group(
                    "left",
                    vec![GroupMemberRef::Suite {
                        id: "shared".into(),
                    }],
                ),
                group(
                    "right",
                    vec![GroupMemberRef::Suite {
                        id: "shared".into(),
                    }],
                ),
            ],
            suites: vec![SuiteResult {
                id: "shared".into(),
                label: "Shared".into(),
                description: "Shared suite".into(),
                working_directory: ".".into(),
                check_duration_ms: 25,
                summary: passing.clone(),
                check_ids: vec!["shared.check".into()],
            }],
            checks: vec![CheckExecutionResult {
                check_id: "shared.check".into(),
                context: CheckContext::Suite {
                    suite_id: "shared".into(),
                },
                label: "Shared check".into(),
                description: "Shared check".into(),
                argv: vec!["tool".into()],
                status: Status::Pass,
                exit_code: Some(0),
                duration_ms: 25,
                timeout_seconds: 30,
                timed_out: false,
                stdout_truncated: false,
                stderr_truncated: false,
                stdout: String::new(),
                stderr: String::new(),
                reason: None,
            }],
            repository: None,
            summary: passing,
        };

        let rendered = render(&report, &Theme::plain());
        assert!(rendered.starts_with("╭─ Workspace Validator · Result"));
        assert_eq!(rendered.matches("SUITE PASS  shared").count(), 2);
        assert_eq!(rendered.matches("(shared)").count(), 1);
        assert!(rendered.contains("Counted: 1 checks + 0 repository gate"));
        assert!(rendered.contains("╰─ Validation complete"));
        for tree_line in [
            "│ GROUP PASS  root",
            "│ ├─ GROUP PASS  left",
            "│ │  └─ SUITE PASS  shared",
            "│ │     └─ CHECK PASS  shared.check",
            "│ └─ GROUP PASS  right",
            "│    └─ SUITE PASS  shared",
        ] {
            assert!(
                rendered.contains(tree_line),
                "missing tree line: {tree_line}"
            );
        }

        let styled = render(
            &report,
            &Theme::resolve(PaletteProfile::Standard, PresentationProfile::Standard),
        );
        assert!(styled.contains('\u{1b}'));
        assert_eq!(strip_ansi_codes(&styled), rendered);
        assert_eq!(
            measure_text_width(styled.lines().next().unwrap()),
            rendered.lines().next().unwrap().chars().count()
        );
        for token in ["GROUP", "SUITE", "CHECK", "PASS", "(shared)"] {
            assert!(strip_ansi_codes(&styled).contains(token));
        }

        let mut failed = report.clone();
        failed.checks[0].status = Status::Fail;
        failed.checks[0].reason = Some("command failed".into());
        failed.checks[0].stderr = "diagnostic stderr\nsecond stderr line\n".into();
        failed.checks[0].stdout = "diagnostic stdout\n\nlast stdout line\n".into();
        for palette in [
            PaletteProfile::Plain,
            PaletteProfile::Standard,
            PaletteProfile::HighContrast,
            PaletteProfile::Protanopia,
            PaletteProfile::Deuteranopia,
            PaletteProfile::Tritanopia,
            PaletteProfile::Achromatopsia,
        ] {
            for presentation in [
                PresentationProfile::Standard,
                PresentationProfile::LowVision,
            ] {
                let rendered = render(&failed, &Theme::resolve(palette, presentation));
                let stripped = strip_ansi_codes(&rendered);
                for diagnostic in [
                    "FAIL",
                    "Reason: command failed",
                    "stderr:\n│       │ diagnostic stderr\n│       │ second stderr line",
                    "stdout:\n│       │ diagnostic stdout\n│       │\n│       │ last stdout line",
                ] {
                    assert!(
                        stripped.contains(diagnostic),
                        "{palette:?} + {presentation:?}: {stripped}"
                    );
                }
            }
        }

        let regular = render(&report, &Theme::plain());
        let low_vision = render(
            &report,
            &Theme::resolve(PaletteProfile::Plain, PresentationProfile::LowVision),
        );
        assert_eq!(
            console::strip_ansi_codes(&low_vision).lines().count(),
            regular.lines().count() + 26
        );
        assert!(low_vision.contains("\u{1b}"));
        let regular_without_gaps = regular
            .lines()
            .filter(|line| !is_tree_gap(line))
            .collect::<Vec<_>>();
        let stripped_low_vision = strip_ansi_codes(&low_vision);
        let low_vision_without_gaps = stripped_low_vision
            .lines()
            .filter(|line| !is_tree_gap(line))
            .collect::<Vec<_>>();
        assert_eq!(
            low_vision_without_gaps, regular_without_gaps,
            "presentation spacing must not change result content"
        );

        let lines = stripped_low_vision.lines().collect::<Vec<_>>();
        for heading in [
            "Workspace Validator · Result",
            "Tools",
            "Execution tree",
            "Counted outcomes",
        ] {
            let heading_index = lines
                .iter()
                .position(|line| line.contains(heading))
                .expect("result heading");
            assert_eq!(&lines[heading_index + 1..heading_index + 3], &["│", "│"]);
            assert_ne!(lines[heading_index + 3], "│");
        }
        for heading in ["Tools", "Execution tree", "Counted outcomes"] {
            let heading_index = lines
                .iter()
                .position(|line| line.contains(heading))
                .expect("result section heading");
            assert_eq!(&lines[heading_index - 2..heading_index], &["│", "│"]);
            assert_ne!(lines[heading_index - 3], "│");
        }
        let footer_index = lines
            .iter()
            .position(|line| line.contains("Validation complete"))
            .expect("result footer");
        assert_eq!(&lines[footer_index - 2..footer_index], &["│", "│"]);
        assert_ne!(lines[footer_index - 3], "│");

        for (node, expected_gap) in [
            ("│ ├─ GROUP PASS  left", "│ │"),
            ("│ │  └─ SUITE PASS  shared", "│ │  │"),
            ("│ │     └─ CHECK PASS  shared.check", "│ │     │"),
            ("│ └─ GROUP PASS  right", "│ │"),
            ("│    └─ SUITE PASS  shared", "│    │"),
        ] {
            let node_index = lines
                .iter()
                .position(|line| line.contains(node))
                .expect("result tree node");
            assert_eq!(lines[node_index - 1], expected_gap);
        }
    }

    fn is_tree_gap(line: &&str) -> bool {
        line.chars().all(|character| matches!(character, '│' | ' '))
    }
}
