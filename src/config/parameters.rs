//! Pure discovery and expansion of structured argument placeholders.

use crate::contracts::config::ParameterValue;
use std::collections::{BTreeMap, BTreeSet};

/// Returns the name represented by an exact `{name}` argument placeholder.
pub(crate) fn placeholder_name(argument: &str) -> Option<&str> {
    let name = argument.strip_prefix('{')?.strip_suffix('}')?;
    valid_name(name).then_some(name)
}

/// Discovers every placeholder declared implicitly by a check argument vector.
pub(crate) fn placeholder_names(arguments: &[String]) -> BTreeSet<String> {
    arguments
        .iter()
        .filter_map(|argument| placeholder_name(argument).map(str::to_owned))
        .collect()
}

/// Expands placeholders without shell parsing or recursive interpolation.
pub(crate) fn expand(
    arguments: &[String],
    parameters: &BTreeMap<String, ParameterValue>,
) -> Vec<String> {
    let mut expanded = Vec::new();
    for argument in arguments {
        let Some(name) = placeholder_name(argument) else {
            expanded.push(argument.clone());
            continue;
        };
        match parameters.get(name) {
            Some(ParameterValue::Single(value)) => expanded.push(value.clone()),
            Some(ParameterValue::Multiple(values)) => expanded.extend(values.iter().cloned()),
            None => {}
        }
    }
    expanded
}

/// Applies the same identifier grammar used by the public configuration IDs.
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.bytes().enumerate().all(|(index, byte)| {
            (index > 0 || byte.is_ascii_lowercase())
                && (byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-'))
        })
        && !value.ends_with(['.', '_', '-'])
        && !value.as_bytes().windows(2).any(|window| {
            matches!(window[0], b'.' | b'_' | b'-') && matches!(window[1], b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_only_complete_valid_placeholders() {
        let arguments = [
            "{target}".into(),
            "--flag={target}".into(),
            "{invalid name}".into(),
            "{profile.name}".into(),
        ];
        assert_eq!(
            placeholder_names(&arguments),
            BTreeSet::from(["profile.name".into(), "target".into()])
        );
    }

    #[test]
    fn expands_single_multiple_repeated_and_absent_values_literally() {
        let arguments = vec![
            "test".into(),
            "{target}".into(),
            "{missing}".into(),
            "{target}".into(),
            "{runner}".into(),
        ];
        let parameters = BTreeMap::from([
            ("target".into(), ParameterValue::Single("crate one".into())),
            (
                "runner".into(),
                ParameterValue::Multiple(vec!["--".into(), "{target}".into()]),
            ),
        ]);
        assert_eq!(
            expand(&arguments, &parameters),
            ["test", "crate one", "crate one", "--", "{target}"]
        );
    }
}
