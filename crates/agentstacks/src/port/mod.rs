//! The traits the flows and the rules read the outside through, each with a fake for tests.
//!
//! Logic in `types` and `app` never calls `std::fs`, so it takes a port as a generic bound and a
//! rule runs the same over the real disk and over an in-memory tree in a test.
//!
//! # Responsibilities
//!
//! - [`FileTree`]: a directory root with three reads below it, what an entry is, the names a
//!   directory holds and a file's bytes.
//! - [`EntryKind`]: what an entry is, a file, a directory, a symlink or anything else, read without
//!   following a link.
//! - [`Adapter`]: the engine that runs one tool extension, to open it and dispatch `values` and
//!   `plan`.
//! - [`Binary`]: whether a binary is on `PATH`, and what `airsl --version` printed.
//! - [`Git`]: the remote side of a git source, its tags, a tag's commit and a commit's files.
//! - [`Workspace`]: the project directory, the two project files, the state of a project path, the
//!   tree behind a local source and the writes apply makes.
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: reading the real disk belongs to the `fs` module, and a rule over a
//! port belongs to `types`. The in-memory tree that tests build lives here but is compiled for
//! tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod adapter;
pub mod binary;
#[cfg(test)]
pub mod fake;
pub mod git;
pub mod kind;
pub mod tree;
pub mod workspace;

#[doc(inline)]
pub use adapter::Adapter;
#[doc(inline)]
pub use binary::Binary;
#[doc(inline)]
pub use git::Git;
#[doc(inline)]
pub use kind::EntryKind;
#[doc(inline)]
pub use tree::FileTree;
#[doc(inline)]
pub use workspace::Workspace;

#[cfg(test)]
#[doc(inline)]
pub use adapter::AdapterCall;
#[cfg(test)]
#[doc(inline)]
pub use adapter::FakeAdapter;
#[cfg(test)]
#[doc(inline)]
pub use adapter::ScriptStep;
#[cfg(test)]
#[doc(inline)]
pub use adapter::ToolScript;
#[cfg(test)]
#[doc(inline)]
pub use binary::BinaryCall;
#[cfg(test)]
#[doc(inline)]
pub use binary::FakeBinary;
#[cfg(test)]
#[doc(inline)]
pub use fake::FakeFileTree;
#[cfg(test)]
#[doc(inline)]
pub use git::FakeGit;
#[cfg(test)]
#[doc(inline)]
pub use git::GitCall;
#[cfg(test)]
#[doc(inline)]
pub use workspace::FakeWorkspace;
#[cfg(test)]
#[doc(inline)]
pub use workspace::WorkspaceCall;
