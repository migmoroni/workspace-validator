//! Shared terminal tree geometry for execution and completed results.

/// Builds the visible prefix for one node from its position at each ancestry
/// level. Each boolean reports whether that node is its parent's last child.
pub(super) fn prefix(branches: &[bool]) -> String {
    let mut prefix = String::from("│ ");
    let Some((current, ancestors)) = branches.split_last() else {
        return prefix;
    };
    prefix.reserve(branches.len() * 3);
    for is_last in ancestors {
        prefix.push_str(if *is_last { "   " } else { "│  " });
    }
    prefix.push_str(if *current { "└─ " } else { "├─ " });
    prefix
}

/// Builds the connector-only row shown between spaced tree nodes.
pub(super) fn continuation(branches: &[bool]) -> String {
    let Some((_current, ancestors)) = branches.split_last() else {
        return "│".into();
    };
    let mut continuation = String::from("│ ");
    continuation.reserve(branches.len() * 3);
    for is_last in ancestors {
        continuation.push_str(if *is_last { "   " } else { "│  " });
    }
    continuation.push('│');
    continuation
}

#[cfg(test)]
mod tests {
    use super::{continuation, prefix};

    #[test]
    fn prefix_preserves_open_ancestor_branches() {
        assert_eq!(prefix(&[]), "│ ");
        assert_eq!(prefix(&[false]), "│ ├─ ");
        assert_eq!(prefix(&[true]), "│ └─ ");
        assert_eq!(prefix(&[false, true]), "│ │  └─ ");
        assert_eq!(prefix(&[true, false]), "│    ├─ ");
    }

    #[test]
    fn continuation_connects_spaced_hierarchy_items() {
        assert_eq!(continuation(&[]), "│");
        assert_eq!(continuation(&[false]), "│ │");
        assert_eq!(continuation(&[true]), "│ │");
        assert_eq!(continuation(&[false, true]), "│ │  │");
        assert_eq!(continuation(&[true, false]), "│    │");
    }
}
