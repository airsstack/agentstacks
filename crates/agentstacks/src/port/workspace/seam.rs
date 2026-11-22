//! The port to the project directory the flows plan against and apply to.
//!
//! Pure trait definition. No inline tests per the unit-test-mandate exemption #3: the trait body
//! has no executable logic.
//!
//! # Responsibilities
//!
//! - [`Workspace`] is the project as the flows see it: the two project files, the state of any
//!   project-relative path, the tree behind a local source directory, and the writes apply makes.
//!
//! # Non-responsibilities
//!
//! It decides nothing. Whether a file changed, which step it gets and what the lock records belong
//! to the rules in `types`; reading and writing a real disk belongs to the `fs` adapter.

use std::io;

use crate::port::tree::FileTree;
use crate::types::op::OpPath;
use crate::types::plan::Resolution;
use crate::types::project::LocalPath;

/// The project directory, reached through project-relative paths.
///
/// Every path is an [`OpPath`], already normalised. Every failure is the platform's own
/// [`io::Error`], so a flow reports `<path>: <the error's Display>`.
///
/// An implementation keeps these rules:
///
/// - Paths compare by their bytes, so `A.md` and `a.md` are two paths even where the disk folds
///   case.
/// - A path is followed through symlinks. [`Workspace::resolve`] reports where it leads and what
///   is there. The lock records the path as it was proposed, never the path it leads to.
/// - [`Workspace::stage`], [`Workspace::swap`] and [`Workspace::delete`] look at where the path
///   leads when they run. One that leads outside the project is refused with an error that
///   displays as `resolves outside the project`, and nothing outside the project is written.
/// - A file is installed in two steps: [`Workspace::stage`] writes a temporary file named
///   `.<name>.agentstacks-tmp` beside the canonical target, and [`Workspace::swap`] renames it onto
///   the target. A file that already exists keeps its permission mode.
/// - Nothing outside these methods writes.
pub trait Workspace {
    /// The tree a local source directory is read through.
    type Tree: FileTree;

    /// The text of `agentstacks.toml`.
    ///
    /// # Errors
    ///
    /// The platform's `NotFound` when the file does not exist, an error displayed as `not valid
    /// UTF-8` when its bytes are not UTF-8, or the I/O failure of the read.
    fn project_text(&self) -> io::Result<String>;

    /// The text of `agentstacks.lock`, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// An error displayed as `not valid UTF-8` when its bytes are not UTF-8, or the I/O failure of
    /// the read.
    fn lock_text(&self) -> io::Result<Option<String>>;

    /// Where `path` leads and what is there.
    ///
    /// The answer says whether the path itself is a symlink and, when its target is inside the
    /// project, whether that target is absent, a regular file or something else. A regular file
    /// comes with its bytes, hash and permission mode. A symlink that leads nowhere is something
    /// else, not absent.
    ///
    /// # Errors
    ///
    /// The I/O failure of looking at the path.
    fn resolve(&self, path: &OpPath) -> io::Result<Resolution>;

    /// The tree of the local source directory `dir`, put through the same resolution as a path.
    ///
    /// `None` when the directory leads outside the project.
    ///
    /// # Errors
    ///
    /// The I/O failure of looking at the directory.
    fn source_tree(&self, dir: &LocalPath) -> io::Result<Option<Self::Tree>>;

    /// Writes `bytes` to the temporary file beside the canonical target of `path`, creating the
    /// directories above it and replacing a leftover temporary file.
    ///
    /// # Errors
    ///
    /// The refusal for a path that leads outside the project, or the I/O failure of the write.
    fn stage(&self, path: &OpPath, bytes: &[u8]) -> io::Result<()>;

    /// Removes the temporary file of `path`. One that is already gone is not an error.
    ///
    /// # Errors
    ///
    /// The I/O failure of the removal.
    fn discard(&self, path: &OpPath) -> io::Result<()>;

    /// Renames the temporary file of `path` onto its canonical target, writing through a symlink
    /// and leaving the link in place. A file that already exists keeps its permission mode.
    ///
    /// # Errors
    ///
    /// The refusal for a path that leads outside the project, or the I/O failure of the rename.
    fn swap(&self, path: &OpPath) -> io::Result<()>;

    /// Removes the file at `path`. The directories it leaves empty stay.
    ///
    /// # Errors
    ///
    /// The refusal for a path that leads outside the project, or the I/O failure of the removal.
    fn delete(&self, path: &OpPath) -> io::Result<()>;

    /// Writes `agentstacks.lock` through the temporary file `.agentstacks.lock.agentstacks-tmp` and
    /// a rename, keeping the permission mode of a lock that exists.
    ///
    /// # Errors
    ///
    /// The I/O failure of the write.
    fn write_lock(&self, text: &str) -> io::Result<()>;
}
