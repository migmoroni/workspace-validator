//! Progress events emitted by execution independently of presentation.

use crate::{
    contracts::{
        config::CheckConfig,
        report::{
            CheckExecutionResult, GroupResult, OverallResult, Status, SuiteResult,
            ValidationSelection,
        },
    },
    planning::{ExecutionNodeRef, PlannedGroup, PlannedSuite},
};

/// Preflight or repository phase reported outside the validation tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressPhase {
    /// Discovery and version validation of required tools.
    Prerequisites,
    /// Repository state captured before selected checks run.
    InitialRepositorySnapshot,
    /// Repository state captured after selected checks run.
    FinalRepositorySnapshot,
}

/// Receives typed validation lifecycle events without controlling execution.
///
/// Every method has a no-op default so consumers can implement only the events
/// needed by their presentation. Reporters must not mutate planning or runtime
/// state; cancellation remains controlled by the atomic flag passed to
/// [`super::run_with_progress`].
pub trait ProgressReporter {
    /// Announces the selection before preflight starts.
    fn validation_started(&mut self, _selection: &ValidationSelection) {}
    /// Announces a preflight or repository phase.
    fn phase_started(&mut self, _phase: ProgressPhase) {}
    /// Announces completion of a preflight or repository phase.
    fn phase_finished(&mut self, _phase: ProgressPhase, _status: Status) {}
    /// Announces first execution of a group.
    fn group_started(
        &mut self,
        _path: &[ExecutionNodeRef],
        _group: &PlannedGroup,
        _immediate_total: usize,
    ) {
    }
    /// Announces reuse of a group already executed through another path.
    fn group_reused(
        &mut self,
        _path: &[ExecutionNodeRef],
        _group: &PlannedGroup,
        _result: &GroupResult,
    ) {
    }
    /// Announces completion of a group and its aggregate result.
    fn group_finished(
        &mut self,
        _path: &[ExecutionNodeRef],
        _group: &PlannedGroup,
        _result: &GroupResult,
    ) {
    }
    /// Announces first execution of a suite.
    fn suite_started(
        &mut self,
        _path: &[ExecutionNodeRef],
        _suite: &PlannedSuite,
        _check_total: usize,
    ) {
    }
    /// Announces reuse of a suite already executed through another path.
    fn suite_reused(
        &mut self,
        _path: &[ExecutionNodeRef],
        _suite: &PlannedSuite,
        _result: &SuiteResult,
    ) {
    }
    /// Announces completion of a suite and its aggregate result.
    fn suite_finished(
        &mut self,
        _path: &[ExecutionNodeRef],
        _suite: &PlannedSuite,
        _result: &SuiteResult,
    ) {
    }
    /// Announces a suite-bound check before its subprocess starts.
    fn check_started(
        &mut self,
        _path: &[ExecutionNodeRef],
        _position: usize,
        _total: usize,
        _check: &CheckConfig,
    ) {
    }
    /// Announces completion of a suite-bound check.
    fn check_finished(
        &mut self,
        _path: &[ExecutionNodeRef],
        _position: usize,
        _total: usize,
        _result: &CheckExecutionResult,
    ) {
    }
    /// Announces a direct check before its subprocess starts.
    fn direct_check_started(&mut self, _position: usize, _total: usize, _check: &CheckConfig) {}
    /// Announces completion of a direct check.
    fn direct_check_finished(
        &mut self,
        _position: usize,
        _total: usize,
        _result: &CheckExecutionResult,
    ) {
    }
    /// Announces the aggregate result after all report data is complete.
    fn validation_finished(&mut self, _result: OverallResult) {}
}

/// No-op reporter used by non-interactive and JSON consumers.
#[derive(Default)]
pub struct SilentProgress;
impl ProgressReporter for SilentProgress {}
