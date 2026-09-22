//! Theme-independent serialization of completed validation reports.

use crate::contracts::report::ValidationReport;

/// Serializes a completed report as indented versioned JSON.
pub fn render(report: &ValidationReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::contracts::report::{
        Summary, ValidationReport, ValidationSelection, REPORT_SCHEMA_VERSION,
    };

    #[test]
    fn json_is_valid_versioned_and_contains_no_ansi() {
        let report = ValidationReport {
            schema_version: REPORT_SCHEMA_VERSION,
            selection: ValidationSelection::Group { id: "all".into() },
            workspace_root: ".".into(),
            started_at_unix_ms: 1,
            duration_ms: 2,
            tools: Vec::new(),
            groups: Vec::new(),
            suites: Vec::new(),
            checks: Vec::new(),
            repository: None,
            summary: Summary::from_statuses([]),
        };

        let rendered = render(&report).unwrap();
        assert!(!rendered.contains('\u{1b}'));
        let document: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(document["schemaVersion"], REPORT_SCHEMA_VERSION);
        assert_eq!(document["selection"]["kind"], "group");
    }
}
