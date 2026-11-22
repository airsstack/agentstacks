//! The process exit code of a command.
//!
//! # Responsibilities
//!
//! - [`Outcome`] is how a command ended, and [`Outcome::code`] is the exit code for it.
//!
//! # Non-responsibilities
//!
//! A usage error exits 2 by clap itself, so it has no outcome here.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::process::ExitCode;

/// How a command ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The command did what was asked, or the author declined: exit 0.
    Success,
    /// A refusal, a finding or a missing terminal: exit 1.
    Refused,
}

impl Outcome {
    /// The exit code: 0 for [`Outcome::Success`], 1 for [`Outcome::Refused`].
    pub(crate) const fn code(self) -> ExitCode {
        match self {
            Self::Success => ExitCode::SUCCESS,
            Self::Refused => ExitCode::FAILURE,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::Outcome;

    #[test]
    fn success_is_zero_and_refused_is_one() {
        assert_eq!(Outcome::Success.code(), ExitCode::from(0));
        assert_eq!(Outcome::Refused.code(), ExitCode::from(1));
    }
}
