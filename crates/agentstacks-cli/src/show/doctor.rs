//! The doctor report as text.
//!
//! # Responsibilities
//!
//! - [`doctor_text`] prints each finding on its own line, then each advisory, or `healthy` when
//!   the report holds neither.
//!
//! # Non-responsibilities
//!
//! Finding the drift and the advisories, and choosing the exit code, belong to the doctor flow
//! and the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::app::doctor::DoctorReport;

/// Findings then advisories, one per line in the given order, lines joined by `\n` with no
/// trailing newline, or `healthy` when the report holds neither.
pub(crate) fn doctor_text(report: &DoctorReport) -> String {
    if report.healthy() {
        return "healthy".to_owned();
    }
    report
        .findings
        .iter()
        .map(ToString::to_string)
        .chain(report.advisories.iter().map(ToString::to_string))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use agentstacks::app::doctor::Advisory;
    use agentstacks::types::manifest::PackVersion;
    use agentstacks::types::plan::Finding;
    use agentstacks::types::project::SubPath;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn empty() -> DoctorReport {
        DoctorReport::default()
    }

    #[test]
    fn no_findings_prints_healthy() {
        assert_eq!(doctor_text(&empty()), "healthy");
    }

    #[test]
    fn findings_print_in_order() -> TestResult {
        let report = DoctorReport {
            findings: vec![
                Finding::Changed(SubPath::new(".claude/agents/coder.md")?),
                Finding::AirslMismatch {
                    installed: PackVersion::new("0.1.5")?,
                    pin: PackVersion::new("0.1.4")?,
                },
            ],
            advisories: Vec::new(),
        };
        assert_eq!(
            doctor_text(&report),
            "changed: .claude/agents/coder.md\nairsl: 0.1.5 installed, lock pins 0.1.4"
        );
        Ok(())
    }

    #[test]
    fn advisories_print_after_findings() -> TestResult {
        let report = DoctorReport {
            findings: vec![Finding::Changed(SubPath::new(".claude/agents/coder.md")?)],
            advisories: vec![Advisory::Unchecked {
                key: agentstacks::types::resolve::SourceKey::Pack(
                    agentstacks::types::PackName::new("core")?,
                ),
            }],
        };
        assert_eq!(
            doctor_text(&report),
            "changed: .claude/agents/coder.md\nagentstacks.toml: packs.core: could not check for updates"
        );
        Ok(())
    }
}
