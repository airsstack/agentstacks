//! The adapter that reads the real disk for the ports the `types` module declares.
//!
//! `types` reaches the disk only through its ports, so one adapter module answers them with
//! `std::fs`.
//!
//! # Responsibilities
//!
//! - [`StdFileTree`]: the `FileTree` port over a real directory, reporting a symlink as a symlink
//!   and refusing any path that could leave its root.
//!
//! # Non-responsibilities
//!
//! It adds no rules about what a directory may hold: refusing symlinked entries in a walk, skipping
//! dot entries and hashing belong to `types`. It never writes: applying changes belongs to the
//! executor.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod disk;

#[doc(inline)]
pub use disk::StdFileTree;
