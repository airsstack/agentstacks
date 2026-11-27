//! The doctor rules: one locked file, and the airsl binary, against the lock.
//!
//! # Responsibilities
//!
//! - [`file_finding`] compares a lock entry with where its path leads on disk.
//! - [`airsl_finding`] compares the version the lock pins with what `airsl --version` answered.
//!
//! # Non-responsibilities
//!
//! Reading the lock and the disk, an I/O failure while reading an entry, and the order of the
//! findings belong to the doctor flow.

use crate::types::lock::FileEntry;
use crate::types::manifest::PackVersion;
use crate::types::op::parse_airsl_version;
use crate::types::plan::{AirslAnswer, DiskState, Finding, Resolution};

/// The finding for the file `entry` records, given where its path leads; `None` when it is
/// healthy.
///
/// A symlink to a regular file inside the project counts as that file.
#[must_use]
pub fn file_finding(entry: &FileEntry, resolution: &Resolution) -> Option<Finding> {
    let path = entry.path().clone();
    match resolution {
        Resolution::Outside { .. } => Some(Finding::Outside(path)),
        Resolution::Inside { state, .. } => match state {
            DiskState::Absent => Some(Finding::Missing(path)),
            DiskState::NotRegular => Some(Finding::NotRegular(path)),
            DiskState::File(file) if file.hash() == entry.hash() => None,
            DiskState::File(_) => Some(Finding::Changed(path)),
        },
    }
}

/// The finding for the airsl binary, given the version the lock pins (or `None` when it pins
/// none); `None` when the installed version is acceptable.
#[must_use]
pub fn airsl_finding(pin: Option<&PackVersion>, answer: AirslAnswer) -> Option<Finding> {
    let text = match answer {
        AirslAnswer::NotFound => {
            return Some(pin.map_or_else(
                || Finding::AirslMissing,
                |pin| Finding::AirslNotFound { pin: pin.clone() },
            ));
        }
        AirslAnswer::SpawnFailed(error) => return Some(Finding::AirslSpawn(error)),
        AirslAnswer::Failed(text) => text,
        AirslAnswer::Printed(text) => match parse_airsl_version(&text) {
            Some(installed) => {
                return match pin {
                    Some(pin) if &installed == pin => None,
                    Some(pin) => Some(Finding::AirslMismatch {
                        installed,
                        pin: pin.clone(),
                    }),
                    None => None,
                };
            }
            None => text,
        },
    };
    Some(Finding::AirslUnparsed {
        text: text.trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;
    use crate::types::lock::Owner;
    use crate::types::plan::{DiskFile, Link};
    use crate::types::project::SubPath;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn entry() -> Result<FileEntry, Box<dyn std::error::Error>> {
        Ok(FileEntry::new(
            SubPath::new(".claude/x")?,
            ContentHash::of_bytes(b"locked"),
            Owner::new("core")?,
        ))
    }

    fn found(resolution: &Resolution) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(file_finding(&entry()?, resolution).map(|finding| finding.to_string()))
    }

    fn inside(link: Link, state: DiskState) -> Resolution {
        Resolution::Inside { link, state }
    }

    fn file(bytes: &[u8]) -> DiskState {
        DiskState::File(DiskFile::new(bytes.to_vec(), 0o644))
    }

    #[test]
    fn drift_missing_file() -> TestResult {
        assert_eq!(
            found(&inside(Link::Direct, DiskState::Absent))?.as_deref(),
            Some("missing: .claude/x")
        );
        Ok(())
    }

    #[test]
    fn drift_not_a_regular_file() -> TestResult {
        assert_eq!(
            found(&inside(Link::Direct, DiskState::NotRegular))?.as_deref(),
            Some("not a regular file: .claude/x")
        );
        Ok(())
    }

    #[test]
    fn drift_outside_the_project() -> TestResult {
        for link in [Link::Symlink, Link::Direct] {
            assert_eq!(
                found(&Resolution::Outside { link })?.as_deref(),
                Some("outside the project: .claude/x")
            );
        }
        Ok(())
    }

    #[test]
    fn drift_symlink_inside_counts_as_regular() -> TestResult {
        assert_eq!(found(&inside(Link::Symlink, file(b"locked")))?, None);
        Ok(())
    }

    #[test]
    fn drift_changed_file() -> TestResult {
        assert_eq!(
            found(&inside(Link::Direct, file(b"edited")))?.as_deref(),
            Some("changed: .claude/x")
        );
        Ok(())
    }

    #[test]
    fn drift_matching_file_is_healthy() -> TestResult {
        assert_eq!(found(&inside(Link::Direct, file(b"locked")))?, None);
        Ok(())
    }

    fn pin() -> Result<PackVersion, Box<dyn std::error::Error>> {
        Ok(PackVersion::new("0.1.4")?)
    }

    fn airsl(answer: AirslAnswer) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(airsl_finding(Some(&pin()?), answer).map(|finding| finding.to_string()))
    }

    #[test]
    fn airsl_not_found_on_path() -> TestResult {
        assert_eq!(
            airsl(AirslAnswer::NotFound)?.as_deref(),
            Some(
                "airsl: not found on PATH (lock pins 0.1.4) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            )
        );
        Ok(())
    }

    #[test]
    fn airsl_version_unparsed() -> TestResult {
        assert_eq!(
            airsl(AirslAnswer::Printed("oops".to_owned()))?.as_deref(),
            Some("airsl: --version printed \"oops\", expected \"airsl <version>\"")
        );
        assert_eq!(
            airsl(AirslAnswer::Failed("airsl 0.1.4".to_owned()))?.as_deref(),
            Some("airsl: --version printed \"airsl 0.1.4\", expected \"airsl <version>\"")
        );
        Ok(())
    }

    #[test]
    fn airsl_version_mismatch() -> TestResult {
        assert_eq!(
            airsl(AirslAnswer::Printed("airsl 0.1.5".to_owned()))?.as_deref(),
            Some("airsl: 0.1.5 installed, lock pins 0.1.4")
        );
        Ok(())
    }

    #[test]
    fn airsl_missing_without_pin() {
        assert_eq!(
            airsl_finding(None, AirslAnswer::NotFound)
                .map(|f| f.to_string())
                .as_deref(),
            Some(
                "airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            )
        );
    }

    #[test]
    fn airsl_parseable_without_pin_is_healthy() {
        assert!(airsl_finding(None, AirslAnswer::Printed("airsl 0.1.5".to_owned())).is_none());
    }

    #[test]
    fn airsl_version_match_is_healthy() -> TestResult {
        assert_eq!(airsl(AirslAnswer::Printed("airsl 0.1.4".to_owned()))?, None);
        Ok(())
    }

    #[test]
    fn airsl_spawn_failure_is_reported() -> TestResult {
        let denied = || Error::new(ErrorKind::PermissionDenied, "permission denied");
        let answer = AirslAnswer::SpawnFailed(denied());
        let refusal = answer.spawn_refusal().map(|d| d.to_string());
        let finding = airsl_finding(Some(&pin()?), answer);
        assert!(matches!(finding, Some(Finding::AirslSpawn(_))));
        let expected = "agentstacks.toml: airsl: permission denied";
        assert_eq!(finding.map(|f| f.to_string()).as_deref(), Some(expected));
        assert_eq!(refusal.as_deref(), Some(expected));
        Ok(())
    }

    #[test]
    fn lock_io_finding_text() {
        let error = Error::new(ErrorKind::PermissionDenied, "permission denied");
        assert_eq!(
            Finding::LockIo(error).to_string(),
            "agentstacks.lock: permission denied"
        );
    }
}
