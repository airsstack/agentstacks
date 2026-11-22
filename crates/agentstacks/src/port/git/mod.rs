//! The port to the remote side of a git source, with a fake for tests.
//!
//! # Responsibilities
//!
//! - [`Git`]: list the tags of a remote, fetch a tag to its commit, and read a commit's files as a
//!   tree.
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: running `git` belongs to the `process` module. The fake that tests
//! build lives here but is compiled for tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(test)]
mod fake;
mod seam;

#[cfg(test)]
#[doc(inline)]
pub use fake::FakeGit;
#[cfg(test)]
#[doc(inline)]
pub use fake::GitCall;
#[doc(inline)]
pub use seam::Git;
