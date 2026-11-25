//! The port to a temporary project and to directories outside the project, with a fake for tests.
//!
//! # Responsibilities
//!
//! - [`Scratch`]: a directory anywhere on the machine opened as a read-only [`FileTree`], and a
//!   temporary project created from a set of files as a [`Workspace`].
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: the real disk and the temporary directory belong to the `fs` module.
//! The in-memory scratch that tests build lives here and is compiled for tests and under the
//! `testing` feature.
//!
//! [`FileTree`]: crate::port::FileTree
//! [`Workspace`]: crate::port::Workspace
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(any(test, feature = "testing"))]
mod fake;
mod seam;

#[cfg(any(test, feature = "testing"))]
#[doc(inline)]
pub use fake::FakeScratch;
#[cfg(any(test, feature = "testing"))]
#[doc(inline)]
pub use fake::ScratchCall;
#[doc(inline)]
pub use seam::Scratch;
