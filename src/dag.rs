//! Generic, iterative dependency-graph operations.

use std::collections::{BTreeMap, BTreeSet};

/// Validates references and rejects cycles in a dependency graph.
pub fn validate_graph(
    ids: &BTreeSet<String>,
    dependencies: &BTreeMap<String, Vec<String>>,
    label: &str,
) -> Result<(), String> {
    for (id, required) in dependencies {
        for dependency in required {
            if !ids.contains(dependency) {
                return Err(format!(
                    "{label} {id} references missing dependency {dependency}"
                ));
            }
        }
    }
    // Iterative color-state DFS keeps deeply nested user configuration stack
    // safe while retaining the active path needed for a useful cycle message.
    let mut state: BTreeMap<String, u8> = ids.iter().map(|id| (id.clone(), 0)).collect();
    for start in ids {
        if state[start] != 0 {
            continue;
        }
        let mut stack = vec![(start.clone(), 0usize)];
        let mut path = Vec::new();
        while let Some((id, index)) = stack.pop() {
            if index == 0 {
                state.insert(id.clone(), 1);
                path.push(id.clone());
            }
            let children = dependencies.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            if index < children.len() {
                stack.push((id.clone(), index + 1));
                let child = &children[index];
                match state[child] {
                    0 => stack.push((child.clone(), 0)),
                    1 => {
                        let at = path.iter().position(|v| v == child).unwrap_or(0);
                        let mut cycle = path[at..].to_vec();
                        cycle.push(child.clone());
                        return Err(format!("cycle in {label} graph: {}", cycle.join(" -> ")));
                    }
                    _ => {}
                }
            } else {
                state.insert(id.clone(), 2);
                path.pop();
            }
        }
    }
    Ok(())
}

/// Returns the transitive dependency closure in dependency-first order.
pub fn dependency_order(target: &str, dependencies: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    // The expansion marker emulates recursive post-order traversal so every
    // dependency appears before the node that requires it.
    let mut stack = vec![(target.to_string(), false)];
    while let Some((id, expanded)) = stack.pop() {
        if expanded {
            result.push(id);
            continue;
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        stack.push((id.clone(), true));
        if let Some(children) = dependencies.get(&id) {
            for child in children.iter().rev() {
                stack.push((child.clone(), false));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{dependency_order, validate_graph};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn deeply_nested_graphs_are_stack_safe() {
        let count = 10_000;
        let ids: BTreeSet<_> = (0..count).map(|index| format!("n{index}")).collect();
        let dependencies: BTreeMap<_, _> = (0..count)
            .map(|index| {
                let values = if index == 0 {
                    vec![]
                } else {
                    vec![format!("n{}", index - 1)]
                };
                (format!("n{index}"), values)
            })
            .collect();
        validate_graph(&ids, &dependencies, "fixture").unwrap();
        assert_eq!(
            dependency_order(&format!("n{}", count - 1), &dependencies).len(),
            count
        );
    }
}
