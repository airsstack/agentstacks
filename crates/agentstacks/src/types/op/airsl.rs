//! What the core says about the `airsl` binary it finds, or does not find, on `PATH`.
//!
//! # Responsibilities
//!
//! - [`parse_airsl_version`] reads the installed version from `airsl --version`: exactly
//!   `airsl <semver>`, after trimming surrounding whitespace. This is the one copy of that rule.
//! - [`airsl_not_found`] refuses a missing binary and names the packs that need it, and
//!   [`airsl_unparseable`] refuses output that is not `airsl <version>`.
//!
//! The refusal for a version outside the packs' range is
//! [`AirslRange::installed_refusal`](crate::types::AirslRange::installed_refusal).
//!
//! # Non-responsibilities
//!
//! Running the binary, decoding its bytes and treating a failing exit as unparseable belong to
//! the process adapter and the flow that calls it.

use std::path::Path;

use crate::types::manifest::PackVersion;
use crate::types::op::OpError;
use crate::types::report::{Diagnostic, Location};
use crate::types::set::AirslRange;

/// The version `airsl --version` printed, when it printed exactly `airsl <semver>`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::parse_airsl_version;
///
/// let version = parse_airsl_version("airsl 0.1.4\n");
/// assert_eq!(version.map(|v| v.to_string()).as_deref(), Some("0.1.4"));
/// assert!(parse_airsl_version("airsl-dev").is_none());
/// ```
#[must_use]
pub fn parse_airsl_version(text: &str) -> Option<PackVersion> {
    let version = text.trim().strip_prefix("airsl ")?;
    PackVersion::new(version).ok()
}

/// The refusal at the project file that `airsl` is needed and not on `PATH`.
#[must_use]
pub fn airsl_not_found(project_file: &Path, range: &AirslRange) -> Diagnostic {
    let packs = range.needs().iter().map(|(pack, _)| pack.clone()).collect();
    Diagnostic::new(
        Location::set_level(project_file, None),
        OpError::AirslNotFound { packs },
    )
}

/// The refusal at the project file that `airsl --version` printed `text`, trimmed.
#[must_use]
pub fn airsl_unparseable(project_file: &Path, text: &str) -> Diagnostic {
    Diagnostic::new(
        Location::set_level(project_file, None),
        OpError::AirslVersionText {
            text: text.trim().to_owned(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::manifest::AirslReq;
    use crate::types::name::PackName;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const PROJECT: &str = "agentstacks.toml";

    fn range(needs: &[(&str, &str)]) -> Result<AirslRange, Box<dyn std::error::Error>> {
        let mut listed = Vec::new();
        for (pack, req) in needs {
            listed.push((PackName::new(pack)?, AirslReq::new(req)?));
        }
        Ok(AirslRange::new(listed).ok_or("a pack declares a range")?)
    }

    #[test]
    fn airsl_version_parses_the_probed_output() {
        for text in ["airsl 0.1.4", "airsl 0.1.4\n", "  airsl 0.1.4  \n"] {
            let parsed = parse_airsl_version(text).map(|version| version.to_string());
            assert_eq!(parsed.as_deref(), Some("0.1.4"), "{text:?}");
        }
    }

    #[test]
    fn airsl_version_refuses_other_text() {
        for text in [
            "airsl-dev",
            "0.1.4",
            "airsl",
            "airsl x.y",
            "airsl 0.1.4 extra",
            "",
        ] {
            assert_eq!(parse_airsl_version(text), None, "{text:?}");
        }
    }

    #[test]
    fn airsl_not_found_names_the_needing_packs() -> TestResult {
        let needs = range(&[("sdlc", "<0.3"), ("core", ">=0.1.4")])?;
        assert_eq!(
            airsl_not_found(Path::new(PROJECT), &needs).to_string(),
            "agentstacks.toml: airsl not found on PATH (needed by core, sdlc)"
        );
        Ok(())
    }

    #[test]
    fn airsl_unparseable_quotes_the_trimmed_text() {
        assert_eq!(
            airsl_unparseable(Path::new(PROJECT), "  airsl-dev\n").to_string(),
            "agentstacks.toml: airsl --version printed \"airsl-dev\", expected \"airsl <version>\""
        );
    }

    #[test]
    fn airsl_out_of_range_reuses_the_built_refusal() -> TestResult {
        let needs = range(&[("core", ">=0.1.4")])?;
        let installed = parse_airsl_version("airsl 0.1.3").ok_or("a version")?;
        let refusal = needs
            .installed_refusal(Path::new(PROJECT), &installed)
            .ok_or("0.1.3 is below the range")?;
        assert_eq!(
            refusal.to_string(),
            "agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4"
        );
        Ok(())
    }
}
