//! The port to the executables on the machine, with a fake for tests.
//!
//! # Responsibilities
//!
//! - [`Binary`]: whether a binary is on `PATH`, and what `airsl --version` printed.
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: searching `PATH` and running `airsl` belong to the `process` module.
//! The fake that tests build lives here but is compiled for tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(test)]
mod fake;
mod seam;

#[cfg(test)]
#[doc(inline)]
pub use fake::BinaryCall;
#[cfg(test)]
#[doc(inline)]
pub use fake::FakeBinary;
#[doc(inline)]
pub use seam::Binary;
