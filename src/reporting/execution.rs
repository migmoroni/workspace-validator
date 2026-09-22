//! Interactive presentation of active validation execution.
//!
//! This module observes typed progress events emitted by the operational
//! executor. It owns only terminal presentation state and never starts tools,
//! checks, or repository operations.

use super::format::{format_duration, format_selection, format_summary, status_label};
use super::theme::{Role, Theme};
use super::tree;
use crate::{
    contracts::{
        config::CheckConfig,
        report::{
            CheckExecutionResult, GroupResult, OverallResult, Status, SuiteResult,
            ValidationSelection,
        },
    },
    execution::progress::{ProgressPhase, ProgressReporter},
    planning::{ExecutionNodeRef, PlannedGroup, PlannedSuite},
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

struct NodeLine {
    bar: ProgressBar,
    label: String,
    /// Whether each visible branch, from the root down, is its parent's last child.
    branches: Vec<bool>,
}

/// Interactive terminal presentation for hierarchical validation progress.
///
/// Bars are keyed by full membership path rather than suite ID so a reused
/// suite can advance each visible parent while execution remains memoized.
pub struct TerminalExecutionReporter {
    theme: Theme,
    multi: MultiProgress,
    nodes: BTreeMap<String, NodeLine>,
    header: Option<ProgressBar>,
    phase: Option<ProgressBar>,
    direct: Option<ProgressBar>,
    started: Option<Instant>,
    has_execution_item: bool,
}

impl TerminalExecutionReporter {
    /// Creates a reporter using one resolved theme for the complete run.
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            multi: MultiProgress::new(),
            nodes: BTreeMap::new(),
            header: None,
            phase: None,
            direct: None,
            started: None,
            has_execution_item: false,
        }
    }

    /// Reports whether indicatif is presenting an interactive execution block.
    pub(crate) fn is_visible(&self) -> bool {
        !self.multi.is_hidden()
    }

    fn key(path: &[ExecutionNodeRef]) -> String {
        path.iter()
            .map(|node| match node {
                ExecutionNodeRef::Group { id } => format!("g:{id}"),
                ExecutionNodeRef::Suite { id } => format!("s:{id}"),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn spinner(&self, message: String) -> ProgressBar {
        let bar = self.multi.add(ProgressBar::new_spinner());
        bar.set_style(active_spinner_style(&self.theme));
        bar.set_prefix(self.theme.paint(Role::Border, "│ "));
        bar.set_message(message);
        bar.enable_steady_tick(Duration::from_millis(80));
        bar
    }

    /// Derives a tree position from the parent's current progress before the
    /// child advances it. Presentation therefore follows declared suite order
    /// without leaking graph traversal concerns back into execution.
    fn branches_for(&self, path: &[ExecutionNodeRef]) -> Vec<bool> {
        if path.len() <= 1 {
            return Vec::new();
        }
        let parent_key = Self::key(&path[..path.len() - 1]);
        let Some(parent) = self.nodes.get(&parent_key) else {
            return vec![false; path.len() - 1];
        };
        let mut branches = parent.branches.clone();
        let is_last = parent
            .bar
            .length()
            .is_none_or(|total| parent.bar.position() + 1 >= total);
        branches.push(is_last);
        branches
    }

    fn advance_parent(&self, path: &[ExecutionNodeRef]) {
        if path.len() <= 1 {
            return;
        }
        if let Some(parent) = self.nodes.get(&Self::key(&path[..path.len() - 1])) {
            parent.bar.inc(1);
        }
    }

    fn begin_stage(&mut self) {
        self.begin_item_with_gap(self.theme.execution_gap(), "│");
    }

    fn begin_tree_item(&mut self, branches: &[bool]) {
        self.begin_item_with_gap(
            tree_gap_size(&self.theme, branches),
            &tree::continuation(branches),
        );
    }

    fn begin_item_with_gap(&mut self, size: usize, gap: &str) {
        if self.has_execution_item {
            add_gap_lines(&self.multi, &self.theme, size, gap);
        }
        self.has_execution_item = true;
    }
}
impl Default for TerminalExecutionReporter {
    fn default() -> Self {
        Self::new(Theme::plain())
    }
}

impl ProgressReporter for TerminalExecutionReporter {
    fn validation_started(&mut self, selection: &ValidationSelection) {
        self.started = Some(Instant::now());
        self.has_execution_item = false;
        let header = self.multi.add(ProgressBar::new_spinner());
        header.set_style(ProgressStyle::with_template("{wide_msg}").unwrap());
        header.set_message(self.theme.frame_rule(
            '╭',
            &format!(
                "Workspace Validator · Execution · {}",
                format_selection(selection)
            ),
            Role::Heading,
        ));
        header.finish();
        self.header = Some(header);
        add_gap_lines(&self.multi, &self.theme, self.theme.execution_gap(), "│");
    }

    fn phase_started(&mut self, phase: ProgressPhase) {
        self.begin_stage();
        self.phase = Some(self.spinner(phase_label(phase).into()));
    }

    fn phase_finished(&mut self, phase: ProgressPhase, status: Status) {
        if let Some(bar) = self.phase.take() {
            let duration = format_duration(bar.elapsed().as_millis() as u64);
            finish_line(
                &bar,
                self.theme.paint(Role::Border, "│ "),
                status,
                format!(
                    "{}  {}",
                    phase_label(phase),
                    self.theme.paint(Role::Duration, duration)
                ),
                &self.theme,
            );
        }
    }

    fn group_started(&mut self, path: &[ExecutionNodeRef], group: &PlannedGroup, total: usize) {
        let branches = self.branches_for(path);
        self.begin_tree_item(&branches);
        let bar = self.multi.add(ProgressBar::new(total as u64));
        bar.set_style(active_suite_style(&self.theme));
        bar.set_prefix(self.theme.paint(Role::Border, tree::prefix(&branches)));
        bar.set_message(format!(
            "{} {}",
            self.theme.paint(Role::Group, "GROUP"),
            group.label
        ));
        bar.enable_steady_tick(Duration::from_millis(80));
        self.nodes.insert(
            Self::key(path),
            NodeLine {
                bar,
                label: group.label.clone(),
                branches,
            },
        );
    }

    fn group_reused(
        &mut self,
        path: &[ExecutionNodeRef],
        group: &PlannedGroup,
        result: &GroupResult,
    ) {
        let branches = self.branches_for(path);
        self.begin_tree_item(&branches);
        let bar = self.multi.add(ProgressBar::new_spinner());
        finish_line(
            &bar,
            self.theme.paint(Role::Border, tree::prefix(&branches)),
            summary_status(result.summary.result),
            format!(
                "{} {}  {} · {}",
                self.theme.paint(Role::Group, "GROUP"),
                group.label,
                self.theme.paint(Role::Shared, "reused"),
                format_summary(&result.summary)
            ),
            &self.theme,
        );
        self.advance_parent(path);
    }

    fn group_finished(
        &mut self,
        path: &[ExecutionNodeRef],
        group: &PlannedGroup,
        result: &GroupResult,
    ) {
        if let Some(line) = self.nodes.remove(&Self::key(path)) {
            finish_line(
                &line.bar,
                self.theme.paint(Role::Border, tree::prefix(&line.branches)),
                summary_status(result.summary.result),
                format!(
                    "{} {}  {} · {}",
                    self.theme.paint(Role::Group, "GROUP"),
                    group.label,
                    format_summary(&result.summary),
                    self.theme
                        .paint(Role::Duration, format_duration(result.check_duration_ms))
                ),
                &self.theme,
            );
        }
        self.advance_parent(path);
    }

    fn suite_started(&mut self, path: &[ExecutionNodeRef], suite: &PlannedSuite, total: usize) {
        let branches = self.branches_for(path);
        self.begin_tree_item(&branches);
        let bar = self.multi.add(ProgressBar::new(total as u64));
        bar.set_style(active_suite_style(&self.theme));
        bar.set_prefix(self.theme.paint(Role::Border, tree::prefix(&branches)));
        bar.set_message(format!(
            "{} {}",
            self.theme.paint(Role::Suite, "SUITE"),
            suite.label
        ));
        bar.enable_steady_tick(Duration::from_millis(80));
        self.nodes.insert(
            Self::key(path),
            NodeLine {
                bar,
                label: suite.label.clone(),
                branches,
            },
        );
    }

    fn suite_reused(
        &mut self,
        path: &[ExecutionNodeRef],
        suite: &PlannedSuite,
        result: &SuiteResult,
    ) {
        let branches = self.branches_for(path);
        let summary = &result.summary;
        self.begin_tree_item(&branches);
        let bar = self.multi.add(ProgressBar::new_spinner());
        finish_line(
            &bar,
            self.theme.paint(Role::Border, tree::prefix(&branches)),
            summary_status(summary.result),
            format!(
                "{} {}  {} · {}",
                self.theme.paint(Role::Suite, "SUITE"),
                suite.label,
                self.theme.paint(Role::Shared, "reused"),
                format_summary(summary)
            ),
            &self.theme,
        );
        // Reused suites do not create another bar, but they still complete one
        // unit of work in the parent path that referenced them.
        self.advance_parent(path);
    }

    fn check_started(
        &mut self,
        path: &[ExecutionNodeRef],
        _position: usize,
        _total: usize,
        check: &CheckConfig,
    ) {
        if let Some(line) = self.nodes.get(&Self::key(path)) {
            line.bar
                .set_message(active_check_message(&line.label, &check.label, &self.theme));
        }
    }

    fn check_finished(
        &mut self,
        path: &[ExecutionNodeRef],
        _position: usize,
        _total: usize,
        _result: &CheckExecutionResult,
    ) {
        if let Some(line) = self.nodes.get(&Self::key(path)) {
            line.bar.inc(1);
        }
    }

    fn suite_finished(
        &mut self,
        path: &[ExecutionNodeRef],
        suite: &PlannedSuite,
        result: &SuiteResult,
    ) {
        if let Some(line) = self.nodes.remove(&Self::key(path)) {
            let summary = &result.summary;
            let duration = result.check_duration_ms;
            finish_line(
                &line.bar,
                self.theme.paint(Role::Border, tree::prefix(&line.branches)),
                summary_status(summary.result),
                format!(
                    "{} {}  {} · {}",
                    self.theme.paint(Role::Suite, "SUITE"),
                    suite.label,
                    format_summary(summary),
                    self.theme.paint(Role::Duration, format_duration(duration))
                ),
                &self.theme,
            );
        }
        // A child suite advances its parent only after its final aggregate is
        // available, keeping nested bars aligned with execution semantics.
        self.advance_parent(path);
    }

    fn direct_check_started(&mut self, position: usize, total: usize, check: &CheckConfig) {
        self.begin_stage();
        self.direct = Some(self.spinner(format!(
            "[{position}/{total}] {}",
            self.theme.paint(Role::Check, &check.label)
        )));
    }

    fn direct_check_finished(
        &mut self,
        _position: usize,
        _total: usize,
        result: &CheckExecutionResult,
    ) {
        if let Some(bar) = self.direct.take() {
            finish_line(
                &bar,
                self.theme.paint(Role::Border, "│ "),
                result.status,
                format!(
                    "{}  {}",
                    self.theme.paint(Role::Check, &result.label),
                    self.theme
                        .paint(Role::Duration, format_duration(result.duration_ms))
                ),
                &self.theme,
            );
        }
    }

    fn validation_finished(&mut self, result: OverallResult) {
        if let Some(bar) = self.phase.take() {
            bar.finish_and_clear();
        }
        if let Some(bar) = self.direct.take() {
            bar.finish_and_clear();
        }
        for (_, line) in std::mem::take(&mut self.nodes) {
            line.bar.finish_and_clear();
        }
        self.header = None;
        let duration = self
            .started
            .take()
            .map(|started| format_duration(started.elapsed().as_millis() as u64))
            .unwrap_or_else(|| "0 ms".into());
        let status = summary_status(result);
        add_gap_lines(&self.multi, &self.theme, self.theme.execution_gap(), "│");
        let footer = self.multi.add(ProgressBar::new_spinner());
        footer.set_style(completed_frame_style());
        footer.set_message(self.theme.frame_rule(
            '╰',
            &format!("Execution {} · {duration}", status_label(status)),
            status_role(status),
        ));
        // The completed execution remains in scrollback as a concise run log.
        // The detailed report is printed immediately below by the CLI.
        footer.finish();
    }
}

fn active_spinner_style(theme: &Theme) -> ProgressStyle {
    ProgressStyle::with_template(&format!(
        "{{prefix}}{} {{wide_msg}}  {}",
        styled_placeholder("spinner", &theme.indicatif_modifier(Role::ActiveSpinner)),
        styled_placeholder("elapsed_precise", &theme.indicatif_modifier(Role::Duration))
    ))
    .unwrap()
}

fn active_suite_style(theme: &Theme) -> ProgressStyle {
    let complete = theme.indicatif_modifier(Role::ProgressComplete);
    let pending = theme.indicatif_modifier(Role::ProgressPending);
    ProgressStyle::with_template(&format!(
        "{{prefix}}{} {} {{pos:>2}}/{{len:<2}} {{wide_msg}}  {}",
        styled_placeholder("spinner", &theme.indicatif_modifier(Role::ActiveSpinner)),
        bar_placeholder(theme.progress_width(), &complete, &pending),
        styled_placeholder("elapsed_precise", &theme.indicatif_modifier(Role::Duration))
    ))
    .unwrap()
    .progress_chars("━╸─")
}

fn completed_style(theme: &Theme, status: Status) -> ProgressStyle {
    ProgressStyle::with_template(&format!(
        "{{prefix}}{} {{wide_msg}}",
        styled_placeholder("spinner", &theme.indicatif_modifier(status_role(status)))
    ))
    .unwrap()
    .tick_strings(&["", status_label(status)])
}

fn styled_placeholder(field: &str, modifier: &str) -> String {
    if modifier.is_empty() {
        format!("{{{field}}}")
    } else {
        format!("{{{field}:{modifier}}}")
    }
}

fn bar_placeholder(width: usize, complete: &str, pending: &str) -> String {
    if complete.is_empty() && pending.is_empty() {
        format!("{{bar:{width}}}")
    } else {
        format!(
            "{{bar:{width}{}/{}}}",
            complete,
            pending.trim_start_matches('.')
        )
    }
}

fn add_gap_lines(multi: &MultiProgress, theme: &Theme, size: usize, content: &str) {
    for _ in 0..size {
        let gap = multi.add(ProgressBar::new_spinner());
        gap.set_style(completed_frame_style());
        gap.set_message(theme.paint(Role::Border, content));
        gap.finish();
    }
}

fn completed_frame_style() -> ProgressStyle {
    ProgressStyle::with_template("{wide_msg}").unwrap()
}

fn finish_line(bar: &ProgressBar, prefix: String, status: Status, message: String, theme: &Theme) {
    bar.disable_steady_tick();
    bar.set_prefix(prefix);
    bar.set_message(message);
    bar.set_style(completed_style(theme, status));
    bar.finish();
}

fn tree_gap_size(theme: &Theme, branches: &[bool]) -> usize {
    if branches.is_empty() {
        theme.execution_gap()
    } else {
        theme.item_gap()
    }
}

fn active_check_message(suite_label: &str, check_label: &str, theme: &Theme) -> String {
    if suite_label.eq_ignore_ascii_case(check_label) {
        theme.paint(Role::Check, suite_label)
    } else {
        format!("{suite_label} › {}", theme.paint(Role::Check, check_label))
    }
}

fn phase_label(phase: ProgressPhase) -> &'static str {
    match phase {
        ProgressPhase::Prerequisites => "Checking required tools",
        ProgressPhase::InitialRepositorySnapshot => "Capturing initial repository state",
        ProgressPhase::FinalRepositorySnapshot => "Checking repository integrity",
    }
}

fn summary_status(result: OverallResult) -> Status {
    match result {
        OverallResult::Pass => Status::Pass,
        OverallResult::Fail => Status::Fail,
        OverallResult::Blocked => Status::Blocked,
    }
}

fn status_role(status: Status) -> Role {
    match status {
        Status::Pass => Role::Success,
        Status::Fail => Role::Failure,
        Status::Blocked => Role::Blocked,
        Status::Skipped => Role::Skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_check_message, active_spinner_style, bar_placeholder, completed_style,
        styled_placeholder, tree_gap_size, TerminalExecutionReporter,
    };
    use crate::contracts::report::Status;
    use crate::reporting::theme::{PaletteProfile, PresentationProfile, Theme};
    use console::strip_ansi_codes;

    #[test]
    fn low_vision_separates_stages_more_than_nested_tree_items() {
        let low_vision = Theme::resolve(PaletteProfile::Plain, PresentationProfile::LowVision);
        assert_eq!(tree_gap_size(&low_vision, &[]), 2);
        assert_eq!(tree_gap_size(&low_vision, &[false]), 1);
        assert_eq!(tree_gap_size(&low_vision, &[false, true]), 1);

        let regular = Theme::plain();
        assert_eq!(tree_gap_size(&regular, &[]), 0);
        assert_eq!(tree_gap_size(&regular, &[false]), 0);
    }

    #[test]
    fn active_check_message_does_not_repeat_identical_labels() {
        let plain = Theme::plain();
        assert_eq!(
            active_check_message("Testes Rust", "Testes Rust", &plain),
            "Testes Rust"
        );
        assert_eq!(
            active_check_message("Svelte", "Análise estática Svelte", &plain),
            "Svelte › Análise estática Svelte"
        );

        let standard = Theme::resolve(PaletteProfile::Standard, PresentationProfile::Standard);
        assert_eq!(
            strip_ansi_codes(&active_check_message(
                "Svelte",
                "Análise estática Svelte",
                &standard
            )),
            "Svelte › Análise estática Svelte"
        );
    }

    #[test]
    fn default_reporter_and_plain_templates_have_no_style_modifiers() {
        let reporter = TerminalExecutionReporter::default();
        assert_eq!(reporter.theme.palette_profile(), PaletteProfile::Plain);
        assert_eq!(
            reporter.theme.presentation_profile(),
            PresentationProfile::Standard
        );
        let standard = TerminalExecutionReporter::new(Theme::resolve(
            PaletteProfile::Standard,
            PresentationProfile::LowVision,
        ));
        assert_eq!(standard.theme.palette_profile(), PaletteProfile::Standard);
        assert_eq!(
            standard.theme.presentation_profile(),
            PresentationProfile::LowVision
        );
        let _ = active_spinner_style(&Theme::plain());
        assert_eq!(styled_placeholder("spinner", ""), "{spinner}");
        assert_eq!(bar_placeholder(14, "", ""), "{bar:14}");
        assert_eq!(bar_placeholder(22, ".bold", ".dim"), "{bar:22.bold/dim}");
    }

    #[test]
    fn completed_progress_keeps_explicit_status_labels() {
        let theme = Theme::resolve(PaletteProfile::Standard, PresentationProfile::LowVision);
        for (status, label) in [
            (Status::Pass, "PASS"),
            (Status::Fail, "FAIL"),
            (Status::Blocked, "BLOCKED"),
            (Status::Skipped, "SKIPPED"),
        ] {
            assert_eq!(completed_style(&theme, status).get_final_tick_str(), label);
        }
    }
}
