//! Pure formatting shared by execution and result presentation.

use crate::contracts::report::{Status, Summary, ValidationSelection};

pub(super) fn format_selection(selection: &ValidationSelection) -> String {
    match selection {
        ValidationSelection::Group { id } => format!("group {id}"),
        ValidationSelection::Suite { id } => format!("suite {id}"),
        ValidationSelection::Check { id } => format!("check {id}"),
    }
}

pub(super) fn format_duration(milliseconds: u64) -> String {
    if milliseconds < 1_000 {
        format!("{milliseconds} ms")
    } else if milliseconds < 60_000 {
        format!("{:.2} s", milliseconds as f64 / 1_000.0)
    } else {
        let minutes = milliseconds / 60_000;
        let seconds = (milliseconds % 60_000) as f64 / 1_000.0;
        format!("{minutes} min {seconds:.1} s")
    }
}

pub(super) fn format_summary(summary: &Summary) -> String {
    if summary.fail == 0 && summary.blocked == 0 && summary.skipped == 0 {
        return format!(
            "{} {}",
            summary.pass,
            if summary.pass == 1 { "check" } else { "checks" }
        );
    }
    let mut parts = Vec::new();
    for (count, label) in [
        (summary.pass, "pass"),
        (summary.fail, "fail"),
        (summary.blocked, "blocked"),
        (summary.skipped, "skipped"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {label}"));
        }
    }
    parts.join(" · ")
}

pub(super) fn status_label(status: Status) -> &'static str {
    match status {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Blocked => "BLOCKED",
        Status::Skipped => "SKIPPED",
    }
}

#[cfg(test)]
mod tests {
    use super::{format_duration, format_summary, status_label};
    use crate::contracts::report::{Status, Summary};

    #[test]
    fn formats_short_and_long_durations() {
        assert_eq!(format_duration(25), "25 ms");
        assert_eq!(format_duration(1_250), "1.25 s");
        assert_eq!(format_duration(65_400), "1 min 5.4 s");
    }

    #[test]
    fn summary_text_omits_zero_statuses() {
        assert_eq!(
            format_summary(&Summary::from_statuses([Status::Pass, Status::Pass])),
            "2 checks"
        );
        assert_eq!(
            format_summary(&Summary::from_statuses([
                Status::Pass,
                Status::Fail,
                Status::Skipped,
            ])),
            "1 pass · 1 fail · 1 skipped"
        );
    }

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(status_label(Status::Pass), "PASS");
        assert_eq!(status_label(Status::Fail), "FAIL");
        assert_eq!(status_label(Status::Blocked), "BLOCKED");
        assert_eq!(status_label(Status::Skipped), "SKIPPED");
    }
}
