//! The port to the project directory, with a fake for tests.
//!
//! # Responsibilities
//!
//! - [`Workspace`]: the project files, the state of a project path, the tree behind a local source
//!   and the writes apply makes.
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: the real disk belongs to the `fs` module. The in-memory project that
//! tests build lives here but is compiled for tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(test)]
mod fake;
mod seam;

#[cfg(test)]
#[doc(inline)]
pub use fake::FakeWorkspace;
#[cfg(test)]
#[doc(inline)]
pub use fake::WorkspaceCall;
#[doc(inline)]
pub use seam::Workspace;
