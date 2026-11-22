//! The doctor report as text.
//!
//! # Responsibilities
//!
//! - [`doctor_text`] prints each finding on its own line, or `healthy` when there are none.
//!
//! # Non-responsibilities
//!
//! Finding the drift and choosing the exit code belong to the doctor flow and the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::types::plan::Finding;

/// One finding per line in the given order, lines joined by `\n` with no trailing newline, or
/// `healthy` when `findings` is empty.
pub(crate) fn doctor_text(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "healthy".to_owned();
    }
    findings
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use agentstacks::types::manifest::PackVersion;
    use agentstacks::types::project::SubPath;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn no_findings_prints_healthy() {
        assert_eq!(doctor_text(&[]), "healthy");
    }

    #[test]
    fn findings_print_in_order() -> TestResult {
        let findings = [
            Finding::Changed(SubPath::new(".claude/agents/coder.md")?),
            Finding::AirslMismatch {
                installed: PackVersion::new("0.1.5")?,
                pin: PackVersion::new("0.1.4")?,
            },
        ];
        assert_eq!(
            doctor_text(&findings),
            "changed: .claude/agents/coder.md\nairsl: 0.1.5 installed, lock pins 0.1.4"
        );
        Ok(())
    }
}
