//! The read-only view of one directory that every reader of the disk goes through.
//!
//! Logic in `types` never calls `std::fs`, so it takes a [`FileTree`] as a generic bound and a rule
//! runs the same over the real disk and over an in-memory tree in a test.
//!
//! # Responsibilities
//!
//! - [`FileTree`]: a directory root with three reads below it, what an entry is, the names a directory holds and a file's bytes.
//! - [`EntryKind`]: what an entry is, a file, a directory, a symlink or anything else, read without following a link.
//! - `utf8_text`: a file's bytes as text, refused with the same `not valid UTF-8` as [`FileTree::read_text`].
//!
//! # Non-responsibilities
//!
//! Reading the real disk belongs to the `fs` module; writing any file belongs to the executor. The
//! in-memory tree that tests build lives here but is compiled for tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(test)]
pub mod fake;
pub mod kind;
pub mod tree;

#[doc(inline)]
pub use kind::EntryKind;
#[doc(inline)]
pub use tree::FileTree;

#[cfg(test)]
#[doc(inline)]
pub use fake::FakeFileTree;

pub(crate) use tree::utf8_text;
