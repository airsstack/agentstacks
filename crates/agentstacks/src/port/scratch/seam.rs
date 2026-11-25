//! The port to a temporary project and to directories outside the project.
//!
//! Pure trait definition with no executable logic, so it carries no inline tests.
//!
//! # Responsibilities
//!
//! - [`Scratch`] opens a directory outside the project for reading and creates a temporary
//!   project holding a given set of files.
//!
//! # Non-responsibilities
//!
//! It decides nothing. Which files the project holds and what a refusal says belong to the rules in
//! `types`; the real disk and the temporary directory belong to the `fs` adapter.

use std::io;
use std::path::Path;

use crate::port::tree::FileTree;
use crate::port::workspace::Workspace;
use crate::types::check::StagedFile;

/// Directories outside the project, and a temporary project beside them.
pub trait Scratch {
    /// The read-only tree of a directory opened with [`Scratch::open`].
    type Tree: FileTree;
    /// The temporary project [`Scratch::create`] builds.
    type Project: Workspace;

    /// Opens the directory `dir` as a tree.
    ///
    /// `dir` is the path to read, resolved against the project directory. `label` is the path as
    /// the user wrote it: the tree's [`FileTree::root`] is `label`, so every refusal that names the
    /// tree shows the path as written.
    ///
    /// # Errors
    ///
    /// The I/O failure of looking at `dir`, such as `NotFound` when it does not exist.
    fn open(&self, dir: &Path, label: &Path) -> io::Result<Self::Tree>;

    /// Creates a temporary project holding `files`.
    ///
    /// Each file is written at its path below the project root, with the directories above it. The
    /// project lasts as long as the returned value.
    ///
    /// # Errors
    ///
    /// The I/O failure of creating the project or of writing a file.
    fn create(&self, files: &[StagedFile]) -> io::Result<Self::Project>;
}
