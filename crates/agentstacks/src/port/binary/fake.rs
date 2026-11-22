//! An in-memory [`Binary`] for tests.
//!
//! It lets a flow look up binaries and `airsl` without a `PATH` or a process, and makes the
//! answers a machine cannot be made to give on demand, such as `airsl` failing to start, one
//! builder call.
//!
//! # Responsibilities
//!
//! - [`FakeBinary`] holds the names that are on `PATH` and what `airsl --version` answers: not
//!   found by default, or the text it printed with a zero exit, the text it printed with a
//!   non-zero exit, or a spawn failure.
//! - [`BinaryCall`] is one entry of the question log that [`FakeBinary::asked`] returns.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only. How the real `PATH` and `airsl` behave is proven by the tests
//! of the `process` module.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io;

use crate::port::binary::seam::Binary;
use crate::types::name::BinaryName;
use crate::types::plan::{AirslAnswer, Exit};

/// One question asked of a [`FakeBinary`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryCall {
    /// [`Binary::on_path`] of a name.
    OnPath(BinaryName),
    /// [`Binary::airsl_version`].
    AirslVersion,
}

/// What the configured `airsl` does when asked its version.
#[derive(Debug, Clone)]
enum Airsl {
    Prints(String),
    ExitsWithFailure(String),
    FailsToStart,
}

/// An in-memory `PATH`.
#[derive(Debug, Default)]
pub struct FakeBinary {
    names: BTreeSet<BinaryName>,
    airsl: Option<Airsl>,
    asked: RefCell<Vec<BinaryCall>>,
}

impl FakeBinary {
    /// A `PATH` holding nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts `name` on `PATH`.
    #[must_use]
    pub fn with(mut self, name: BinaryName) -> Self {
        self.names.insert(name);
        self
    }

    /// Puts `airsl` on `PATH`, printing `text` and exiting with status zero.
    #[must_use]
    pub fn airsl(mut self, text: &str) -> Self {
        self.airsl = Some(Airsl::Prints(text.to_owned()));
        self
    }

    /// Puts `airsl` on `PATH`, printing `text` and exiting with a non-zero status.
    #[must_use]
    pub fn airsl_exit_failure(mut self, text: &str) -> Self {
        self.airsl = Some(Airsl::ExitsWithFailure(text.to_owned()));
        self
    }

    /// Puts `airsl` on `PATH`, but running it fails with [`FakeBinary::denied`].
    #[must_use]
    pub fn airsl_spawn_failure(mut self) -> Self {
        self.airsl = Some(Airsl::FailsToStart);
        self
    }

    /// The error a spawn failure carries.
    #[must_use]
    pub fn denied() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "permission denied")
    }

    /// Every question so far, in order.
    #[must_use]
    pub fn asked(&self) -> Vec<BinaryCall> {
        self.asked.borrow().clone()
    }
}

impl Binary for FakeBinary {
    fn on_path(&self, name: &BinaryName) -> bool {
        self.asked
            .borrow_mut()
            .push(BinaryCall::OnPath(name.clone()));
        self.names.contains(name) || (name.as_str() == "airsl" && self.airsl.is_some())
    }

    fn airsl_version(&self) -> AirslAnswer {
        self.asked.borrow_mut().push(BinaryCall::AirslVersion);
        match &self.airsl {
            None => AirslAnswer::NotFound,
            Some(Airsl::Prints(text)) => AirslAnswer::from_output(text.as_bytes(), Exit::Success),
            Some(Airsl::ExitsWithFailure(text)) => {
                AirslAnswer::from_output(text.as_bytes(), Exit::Failure)
            }
            Some(Airsl::FailsToStart) => AirslAnswer::SpawnFailed(Self::denied()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::plan::AirslAnswer;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn name(text: &str) -> Result<BinaryName, Box<dyn std::error::Error>> {
        Ok(BinaryName::new(text)?)
    }

    #[test]
    fn on_path_answers_configured_names() -> TestResult {
        let binary = FakeBinary::new().with(name("rg")?);
        assert!(binary.on_path(&name("rg")?));
        assert!(!binary.on_path(&name("node")?));
        Ok(())
    }

    #[test]
    fn airsl_version_is_not_found_by_default() {
        assert!(matches!(
            FakeBinary::new().airsl_version(),
            AirslAnswer::NotFound
        ));
    }

    #[test]
    fn airsl_version_prints_the_configured_text() -> TestResult {
        let binary = FakeBinary::new().airsl("airsl 0.1.4");
        assert!(matches!(
            binary.airsl_version(),
            AirslAnswer::Printed(text) if text == "airsl 0.1.4"
        ));
        assert!(binary.on_path(&name("airsl")?));
        Ok(())
    }

    #[test]
    fn airsl_exit_failure_is_the_unparseable_answer() -> TestResult {
        let binary = FakeBinary::new().airsl_exit_failure("airsl 0.1.4");
        assert!(matches!(
            binary.airsl_version(),
            AirslAnswer::Failed(text) if text == "airsl 0.1.4"
        ));
        assert!(binary.on_path(&name("airsl")?));
        Ok(())
    }

    #[test]
    fn airsl_spawn_failure_carries_the_io_error() -> TestResult {
        let binary = FakeBinary::new().airsl_spawn_failure();
        let AirslAnswer::SpawnFailed(error) = binary.airsl_version() else {
            return Err("a spawn failure".into());
        };
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), FakeBinary::denied().to_string());
        assert!(binary.on_path(&name("airsl")?));
        Ok(())
    }

    #[test]
    fn questions_are_logged() -> TestResult {
        let binary = FakeBinary::new();
        binary.on_path(&name("rg")?);
        binary.on_path(&name("node")?);
        let _ = binary.airsl_version();
        assert_eq!(
            binary.asked(),
            [
                BinaryCall::OnPath(name("rg")?),
                BinaryCall::OnPath(name("node")?),
                BinaryCall::AirslVersion,
            ]
        );
        Ok(())
    }
}
