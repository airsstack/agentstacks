//! The adapters that reach the real disk for the ports the `port` module declares.
//!
//! The inside reaches the disk only through those ports, so this module answers them with
//! `std::fs`. It is the only module that reads or writes the project's files and the cache; the
//! `process` adapter reads file metadata only to look a binary up on `PATH`.
//!
//! # Responsibilities
//!
//! - [`StdFileTree`]: the `FileTree` port over a real directory, reporting a symlink as a symlink
//!   and refusing any path that could leave its root. It only reads.
//! - [`StdWorkspace`]: the `Workspace` port over the project directory: where a path leads, what
//!   is there, and the writes apply makes (a temporary file beside the target, a rename, a delete,
//!   the lock).
//! - [`DiskScratch`]: the `Scratch` port. It opens a directory outside the project as a
//!   [`StdFileTree`] and creates a temporary project directory that its [`ScratchProject`] removes
//!   on drop.
//! - `Cache`: the git cache directory, where a fetched revision is kept, so the `process` adapter
//!   that drives `git` leaves every disk step to this module. It is the one thing from `fs` that
//!   `process` uses.
//!
//! # Non-responsibilities
//!
//! They add no rules about what a directory may hold or what a step is: refusing symlinked entries
//! in a walk, hashing, and deciding what to write or remove belong to `types` and the flows.
//! Running `git` belongs to `process`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod cache;
pub mod disk;
pub mod scratch;
pub mod workspace;

pub(crate) use cache::Cache;
#[doc(inline)]
pub use disk::StdFileTree;
#[doc(inline)]
pub use scratch::{DiskScratch, ScratchProject};
#[doc(inline)]
pub use workspace::StdWorkspace;
