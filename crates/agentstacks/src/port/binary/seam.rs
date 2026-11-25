//! The port to the executables on the machine.
//!
//! Pure trait definition with no executable logic, so it carries no inline tests.
//!
//! # Responsibilities
//!
//! - [`Binary`] answers whether a named binary is on `PATH`, what `airsl --version` printed and how
//!   `airsl test` ended in a directory.
//! - [`LuaTestRun`] is what one `airsl test` run ended with.
//!
//! # Non-responsibilities
//!
//! It judges nothing. Reading a version out of the printed text and comparing it with a
//! requirement belong to the rules in `types`; searching `PATH` and running `airsl` belong to the
//! `process` adapter.

use std::io;
use std::path::Path;

use crate::types::name::BinaryName;
use crate::types::plan::AirslAnswer;

/// How one `airsl test` run ended.
///
/// # Examples
///
/// ```
/// use agentstacks::port::binary::LuaTestRun;
///
/// let run = LuaTestRun { success: true, stdout: "2 passed\n".to_owned(), stderr: String::new() };
/// assert!(run.success);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaTestRun {
    /// Whether `airsl` exited with status zero.
    pub success: bool,
    /// What it printed on standard output, as lossy UTF-8.
    pub stdout: String,
    /// What it printed on standard error, as lossy UTF-8.
    pub stderr: String,
}

/// The binaries a pack or an extension needs.
pub trait Binary {
    /// Whether `name` is found on `PATH`.
    ///
    /// A binary is found when an entry of `PATH` holds a regular file of that name with an
    /// execute bit. Empty `PATH` entries are skipped and an unset `PATH` finds nothing. A symlink
    /// counts when it leads to an executable regular file.
    fn on_path(&self, name: &BinaryName) -> bool;

    /// What running `airsl --version` learned.
    ///
    /// - `airsl` not on `PATH` is [`AirslAnswer::NotFound`].
    /// - It ran and exited with status zero: [`AirslAnswer::Printed`] with its standard output as
    ///   lossy UTF-8, trimmed.
    /// - It ran and exited with a non-zero status: [`AirslAnswer::Failed`] with that output, which
    ///   the version rule counts as unparseable.
    /// - It was found but could not be started: [`AirslAnswer::SpawnFailed`] with the I/O error.
    fn airsl_version(&self) -> AirslAnswer;

    /// Runs `airsl test` over the directory `dir` and reports how it ended.
    ///
    /// A run that finds a failing test is `Ok` with `success` false, so the caller shows what was
    /// printed.
    ///
    /// # Errors
    ///
    /// The I/O error of starting `airsl`, including `airsl` not being on `PATH`.
    fn airsl_test(&self, dir: &Path) -> io::Result<LuaTestRun>;
}
