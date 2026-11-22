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
//! tests build lives here and is compiled for tests and under the `testing` feature.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(any(test, feature = "testing"))]
mod fake;
mod seam;

#[cfg(any(test, feature = "testing"))]
#[doc(inline)]
pub use fake::FakeWorkspace;
#[cfg(any(test, feature = "testing"))]
#[doc(inline)]
pub use fake::WorkspaceCall;
#[doc(inline)]
pub use seam::Workspace;
