//! The real adapters for one project directory.
//!
//! # Responsibilities
//!
//! - [`open`] builds the ports the commands run over: the workspace at `-C <dir>` or the current
//!   directory, with no search of the parents, and the git, binary and Lua adapters.
//! - [`scratch`] is the adapter `ext test` opens directories and creates its temporary project
//!   with. It is not part of [`Real`], because only that command needs it.
//!
//! # Non-responsibilities
//!
//! What a command does with the ports belongs to [`command`](crate::command).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::Write;
use std::path::Path;

use agentstacks::fs::{DiskScratch, StdWorkspace};
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};

use crate::command::Ports;
use crate::exit::Outcome;

/// The ports over the real disk, git, `PATH` and the airsl library.
pub(crate) type Real = Ports<StdWorkspace, GitCli, SystemBinary, AirslAdapter>;

/// Opens the project at `dir`, or at the current directory when `dir` is `None`.
///
/// # Errors
///
/// When the directory cannot be opened, prints `<dir>: <error>` to `err` and returns
/// [`Outcome::Refused`].
pub(crate) fn open(dir: Option<&Path>, err: &mut impl Write) -> Result<Real, Outcome> {
    let dir = dir.unwrap_or_else(|| Path::new("."));
    match StdWorkspace::open(dir) {
        Ok(workspace) => Ok(Ports {
            workspace,
            git: GitCli::from_env(),
            binary: SystemBinary::from_env(),
            adapter: AirslAdapter::new(),
        }),
        Err(error) => {
            let _ = writeln!(err, "{}: {error}", dir.display());
            Err(Outcome::Refused)
        }
    }
}

/// The scratch space over the real disk.
pub(crate) const fn scratch() -> DiskScratch {
    DiskScratch
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn missing_dash_c_directory_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let missing = dir.path().join("nope");
        let mut err = Vec::new();
        let got = open(Some(&missing), &mut err);
        assert!(matches!(got, Err(Outcome::Refused)));
        let text = String::from_utf8(err)?;
        assert!(
            text.starts_with(&format!("{}: ", missing.display())),
            "{text}"
        );
        assert!(text.ends_with('\n'), "{text:?}");
        Ok(())
    }

    #[test]
    fn an_existing_directory_opens_without_output() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut err = Vec::new();
        let got = open(Some(dir.path()), &mut err);
        assert!(got.is_ok());
        assert!(err.is_empty());
        Ok(())
    }

    #[test]
    fn no_dash_c_opens_the_current_directory() {
        let mut err = Vec::new();
        assert!(open(None::<&Path>, &mut err).is_ok());
    }
}
