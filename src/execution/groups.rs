//! Iterative traversal and memoization of ordered groups.

use super::{progress::ProgressReporter, suites, RunCollections};
use crate::{
    config::ValidatedConfig,
    contracts::{
        config::GroupMemberRef,
        report::{GroupResult, Status, Summary},
    },
    planning::{ExecutionNodeRef, PlannedGroup, ValidationPlan},
};
use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicBool, Arc},
};

struct Frame {
    id: String,
    path: Vec<ExecutionNodeRef>,
    next: usize,
    started: bool,
}

pub(super) fn run(
    validated: &ValidatedConfig,
    plan: &ValidationPlan,
    root: &str,
    tools: &BTreeMap<String, Status>,
    cancelled: &Arc<AtomicBool>,
    progress: &mut impl ProgressReporter,
    collections: &mut RunCollections,
) {
    let planned: BTreeMap<_, _> = plan
        .groups
        .iter()
        .map(|group| (group.id.clone(), group))
        .collect();
    let mut stack = vec![Frame {
        id: root.into(),
        path: vec![ExecutionNodeRef::Group { id: root.into() }],
        next: 0,
        started: false,
    }];
    while let Some(frame) = stack.last_mut() {
        let group = planned[&frame.id];
        if !frame.started {
            if let Some(result) = collections.group_results.get(&frame.id) {
                progress.group_reused(&frame.path, group, result);
                stack.pop();
                continue;
            }
            progress.group_started(&frame.path, group, group.members.len());
            frame.started = true;
        }
        if frame.next < group.members.len() {
            let member = group.members[frame.next].clone();
            frame.next += 1;
            match member {
                GroupMemberRef::Group { id } => {
                    let mut path = frame.path.clone();
                    path.push(ExecutionNodeRef::Group { id: id.clone() });
                    if let Some(result) = collections.group_results.get(&id) {
                        progress.group_reused(&path, planned[&id], result);
                    } else {
                        stack.push(Frame {
                            id,
                            path,
                            next: 0,
                            started: false,
                        });
                    }
                }
                GroupMemberRef::Suite { id } => {
                    let mut path = frame.path.clone();
                    path.push(ExecutionNodeRef::Suite { id: id.clone() });
                    suites::run(
                        suites::SuiteRunContext {
                            validated,
                            plan,
                            tools,
                            cancelled,
                        },
                        &id,
                        &path,
                        progress,
                        collections,
                    );
                }
            }
        } else {
            let result = build_result(group, plan, collections);
            progress.group_finished(&frame.path, group, &result);
            collections.group_results.insert(group.id.clone(), result);
            stack.pop();
        }
    }
}

fn build_result(
    group: &PlannedGroup,
    plan: &ValidationPlan,
    collections: &RunCollections,
) -> GroupResult {
    let concrete = plan.descendant_check_executions[&group.id]
        .iter()
        .filter_map(|key| collections.check_results.get(key))
        .collect::<Vec<_>>();
    GroupResult {
        id: group.id.clone(),
        label: group.label.clone(),
        description: group.description.clone(),
        level: group.level,
        check_duration_ms: concrete.iter().map(|result| result.duration_ms).sum(),
        summary: Summary::from_statuses(concrete.iter().map(|result| result.status)),
        members: group.members.clone(),
    }
}
