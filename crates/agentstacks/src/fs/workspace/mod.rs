//! The adapter that gives the flows the real project directory.
//!
//! # Responsibilities
//!
//! - [`StdWorkspace`]: the `Workspace` port over a project directory on the real disk. Its work is
//!   split by job: where a path leads (`resolve`), what a target holds (`state`), the temporary
//!   file beside a target (`stage`), and installing, deleting and the project and lock writes
//!   (`swap`).
//!
//! # Non-responsibilities
//!
//! No decisions: what a step is, what changed and what the lock records belong to `types`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod project;
mod resolve;
mod stage;
mod state;
mod swap;

#[doc(inline)]
pub use project::StdWorkspace;
