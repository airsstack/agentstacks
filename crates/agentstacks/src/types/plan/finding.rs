//! One finding of the drift check.
//!
//! # Responsibilities
//!
//! - [`Finding`] is one thing the check found wrong: the lock missing or unreadable, a locked
//!   file that is missing, not a regular file, changed or outside the project, or the airsl
//!   binary missing, unreadable or at another version than the lock pins. Its `Display` is the
//!   text shown.
//!
//! # Non-responsibilities
//!
//! Making the findings belongs to the drift rules, and their order belongs to the flow.
//! A finding holds an I/O error, so it is compared by its displayed text.

use crate::types::manifest::PackVersion;
use crate::types::project::SubPath;
use crate::types::report::Diagnostic;

/// One finding.
///
/// # Examples
///
/// ```
/// use agentstacks::types::SubPath;
/// use agentstacks::types::plan::Finding;
///
/// assert_eq!(Finding::Changed(SubPath::new(".claude/x")?).to_string(), "changed: .claude/x");
/// # Ok::<(), agentstacks::types::InvalidSubPath>(())
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Finding {
    /// There is no lock.
    #[error("agentstacks.lock: not found (run apply)")]
    LockMissing,
    /// The lock could not be read.
    #[error("agentstacks.lock: {0}")]
    LockIo(std::io::Error),
    /// The lock is refused.
    #[error("{0}")]
    Lock(Diagnostic),
    /// A locked file is missing.
    #[error("missing: {0}")]
    Missing(SubPath),
    /// A locked path is not a regular file.
    #[error("not a regular file: {0}")]
    NotRegular(SubPath),
    /// A locked file differs from what the lock records.
    #[error("changed: {0}")]
    Changed(SubPath),
    /// A locked path resolves outside the project.
    #[error("outside the project: {0}")]
    Outside(SubPath),
    /// A locked file could not be read.
    #[error("{path}: {error}")]
    Io {
        /// The path.
        path: SubPath,
        /// Why reading failed.
        error: std::io::Error,
    },
    /// The lock pins airsl and none is on `PATH`.
    #[error(
        "airsl: not found on PATH (lock pins {pin}) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
    )]
    AirslNotFound {
        /// The pinned version.
        pin: PackVersion,
    },
    /// No lock pins airsl and none is on `PATH`.
    #[error(
        "airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
    )]
    AirslMissing,
    /// `airsl --version` printed text that is not `airsl <version>`.
    #[error("airsl: --version printed \"{text}\", expected \"airsl <version>\"")]
    AirslUnparsed {
        /// What it printed, trimmed.
        text: String,
    },
    /// The installed airsl is not the version the lock pins.
    #[error("airsl: {installed} installed, lock pins {pin}")]
    AirslMismatch {
        /// The installed version.
        installed: PackVersion,
        /// The pinned version.
        pin: PackVersion,
    },
    /// `airsl` could not be started.
    #[error("agentstacks.toml: airsl: {0}")]
    AirslSpawn(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;
    use crate::types::lock::Lockfile;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn denied() -> Error {
        Error::new(ErrorKind::PermissionDenied, "permission denied")
    }

    #[test]
    fn finding_texts_match_the_spec() -> TestResult {
        let path = || SubPath::new(".claude/x");
        let old = PackVersion::new("0.1.4")?;
        let new = PackVersion::new("0.1.5")?;
        let cases = [
            (
                Finding::LockMissing,
                "agentstacks.lock: not found (run apply)",
            ),
            (
                Finding::LockIo(denied()),
                "agentstacks.lock: permission denied",
            ),
            (Finding::Missing(path()?), "missing: .claude/x"),
            (
                Finding::NotRegular(path()?),
                "not a regular file: .claude/x",
            ),
            (Finding::Changed(path()?), "changed: .claude/x"),
            (Finding::Outside(path()?), "outside the project: .claude/x"),
            (
                Finding::Io {
                    path: path()?,
                    error: denied(),
                },
                ".claude/x: permission denied",
            ),
            (
                Finding::AirslNotFound { pin: old.clone() },
                "airsl: not found on PATH (lock pins 0.1.4) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)",
            ),
            (
                Finding::AirslUnparsed {
                    text: "oops".to_owned(),
                },
                "airsl: --version printed \"oops\", expected \"airsl <version>\"",
            ),
            (
                Finding::AirslMismatch {
                    installed: new,
                    pin: old,
                },
                "airsl: 0.1.5 installed, lock pins 0.1.4",
            ),
            (
                Finding::AirslSpawn(denied()),
                "agentstacks.toml: airsl: permission denied",
            ),
        ];
        for (finding, text) in cases {
            assert_eq!(finding.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn lock_finding_shows_the_lock_refusal() -> TestResult {
        let text = "api = 1\n\n[[pack]]\nname = \"core\"\nversion = \"1.0.0\"\nbranch = \"main\"\n";
        let refusals = Lockfile::parse(std::path::Path::new("agentstacks.lock"), text)
            .err()
            .ok_or("the lock was accepted")?;
        let wanted = "agentstacks.lock: pack[0].branch: unknown key";
        let sorted = refusals.into_sorted();
        let shown: Vec<String> = sorted.iter().map(ToString::to_string).collect();
        let refusal = sorted
            .into_iter()
            .find(|d| d.to_string() == wanted)
            .ok_or_else(|| format!("no unknown-key refusal in {shown:?}"))?;
        assert_eq!(Finding::Lock(refusal).to_string(), wanted);
        Ok(())
    }
}
