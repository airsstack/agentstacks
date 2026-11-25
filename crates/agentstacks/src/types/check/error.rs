//! The refusals of the steps of `ext test` that no other rule owns.
//!
//! # Responsibilities
//!
//! - [`CheckError`] words the two refusals of the run itself: `airsl` missing from `PATH`, and
//!   `airsl test` that could not be started.
//!
//! # Non-responsibilities
//!
//! Where each refusal is shown, and what the run does next, belong to the flow and the command
//! line.

use std::io;

/// A refusal of the `ext test` run itself.
///
/// # Examples
///
/// ```
/// use agentstacks::types::check::CheckError;
///
/// assert_eq!(
///     CheckError::AirslMissing.to_string(),
///     "airsl not found on PATH (needed for the Lua tests and the hooks)"
/// );
/// ```
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    /// `airsl` is not on `PATH`, so neither the Lua tests nor the hooks can run.
    #[error("airsl not found on PATH (needed for the Lua tests and the hooks)")]
    AirslMissing,
    /// `airsl` is on `PATH` but `airsl test` could not be started.
    #[error("airsl test: {0}")]
    AirslTest(#[source] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airsl_test_shows_the_io_error() {
        let error = CheckError::AirslTest(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied",
        ));
        assert_eq!(error.to_string(), "airsl test: permission denied");
    }
}
