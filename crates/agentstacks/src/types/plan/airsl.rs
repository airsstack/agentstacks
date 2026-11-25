//! What `airsl --version` answered.
//!
//! # Responsibilities
//!
//! - [`AirslAnswer`] is the answer: not found on `PATH`, the text it printed, a failing exit,
//!   or a spawn that failed for another reason.
//! - [`AirslAnswer::from_output`] decodes the printed bytes as UTF-8 with replacement and trims
//!   surrounding whitespace; a failing exit is unparseable whatever it printed.
//! - [`AirslAnswer::spawn_refusal`] is the refusal for a spawn that failed.
//!
//! # Non-responsibilities
//!
//! Running the binary belongs to the process adapter. Reading the version out of the text is
//! [`parse_airsl_version`](crate::types::op::parse_airsl_version).

use std::io;

use crate::types::op::OpError;
use crate::types::project::PROJECT_FILE;
use crate::types::report::{Diagnostic, Location};

/// The exit of `airsl --version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The process exited with status zero.
    Success,
    /// The process exited with a non-zero status.
    Failure,
}

/// What running `airsl --version` learned.
///
/// # Examples
///
/// ```
/// use agentstacks::types::plan::{AirslAnswer, Exit};
///
/// let answer = AirslAnswer::from_output(b"airsl 0.1.4\n", Exit::Success);
/// assert!(matches!(answer, AirslAnswer::Printed(text) if text == "airsl 0.1.4"));
/// ```
#[derive(Debug)]
pub enum AirslAnswer {
    /// There is no `airsl` on `PATH`.
    NotFound,
    /// It exited with status zero and printed this text, trimmed.
    Printed(String),
    /// It exited with a non-zero status and printed this text, trimmed.
    Failed(String),
    /// It exists but could not be started.
    SpawnFailed(io::Error),
}

impl AirslAnswer {
    /// The answer for a run that printed `stdout` and exited with `status`.
    #[must_use]
    pub fn from_output(stdout: &[u8], status: Exit) -> Self {
        let text = String::from_utf8_lossy(stdout).trim().to_owned();
        match status {
            Exit::Success => Self::Printed(text),
            Exit::Failure => Self::Failed(text),
        }
    }

    /// The refusal at `agentstacks.toml` for a spawn that failed; `None` for any other answer.
    #[must_use]
    pub fn spawn_refusal(&self) -> Option<Diagnostic> {
        let Self::SpawnFailed(error) = self else {
            return None;
        };
        // `io::Error` is not `Clone`; the copy keeps the kind and the text.
        let copy = io::Error::new(error.kind(), error.to_string());
        Some(Diagnostic::new(
            Location::file(PROJECT_FILE),
            OpError::AirslSpawn(copy),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airsl_answer_decodes_and_trims() {
        let printed = AirslAnswer::from_output(b"airsl 0.1.4\n", Exit::Success);
        assert!(matches!(&printed, AirslAnswer::Printed(text) if text == "airsl 0.1.4"));
        let lossy = AirslAnswer::from_output(b"airsl \xFF\n", Exit::Success);
        assert!(matches!(&lossy, AirslAnswer::Printed(text) if text == "airsl \u{FFFD}"));
        let failed = AirslAnswer::from_output(b"airsl 0.1.4\n", Exit::Failure);
        assert!(matches!(&failed, AirslAnswer::Failed(text) if text == "airsl 0.1.4"));
    }

    #[test]
    fn spawn_refusal_only_for_a_failed_spawn() {
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let refusal = AirslAnswer::SpawnFailed(error).spawn_refusal();
        assert_eq!(
            refusal.map(|d| d.to_string()).as_deref(),
            Some("agentstacks.toml: airsl: permission denied")
        );
        assert!(AirslAnswer::NotFound.spawn_refusal().is_none());
        assert!(
            AirslAnswer::Printed(String::new())
                .spawn_refusal()
                .is_none()
        );
    }
}
