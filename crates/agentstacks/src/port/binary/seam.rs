//! The port to the executables on the machine.
//!
//! Pure trait definition. No inline tests per the unit-test-mandate exemption #3: the trait body
//! has no executable logic.
//!
//! # Responsibilities
//!
//! - [`Binary`] answers whether a named binary is on `PATH` and what `airsl --version` printed.
//!
//! # Non-responsibilities
//!
//! It judges nothing. Reading a version out of the printed text and comparing it with a
//! requirement belong to the rules in `types`; searching `PATH` and running `airsl` belong to the
//! `process` adapter.

use crate::types::name::BinaryName;
use crate::types::plan::AirslAnswer;

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
}
