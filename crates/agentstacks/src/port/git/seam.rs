//! The port to the remote side of a git source.
//!
//! Pure trait definition. No inline tests per the unit-test-mandate exemption #3: the trait body
//! has no executable logic.
//!
//! # Responsibilities
//!
//! - [`Git`] lists the tags of a remote, fetches a tag to the commit it names, and gives the files
//!   of a commit as a tree.
//!
//! # Non-responsibilities
//!
//! It chooses nothing. Which tag is the newest and whether a tree matches the lock belong to the
//! rules in `types`; running `git` and keeping the cache belong to the `process` adapter.

use crate::port::tree::FileTree;
use crate::types::project::{CommitId, GitTag, GitUrl, Pin, SubPath};
use crate::types::resolve::{FetchError, RemoteTag};

/// A git remote, reached through its URL.
///
/// Every failure is a [`FetchError`] without a location: the flow adds `agentstacks.toml` and the
/// key of the entry that named the source. An implementation reports:
///
/// - git not on `PATH` as [`FetchError::GitNotFound`] with an empty `needed_by` on every call. The
///   flow merges the answers of every source into one refusal.
/// - git exiting with a failure as [`FetchError::GitFailed`], with the subcommand and git's stderr.
/// - git failing to start for another reason as [`FetchError::GitSpawn`].
/// - git output that cannot be read (a tag line, a tree record, a commit id, a blob frame) as
///   [`FetchError::GitUnreadable`], with the subcommand and the first line of the output.
/// - a failed cache operation as [`FetchError::CacheIo`], with the path involved.
/// - a `subdir` that is not a directory in the source as [`FetchError::SubdirNotFound`] or
///   [`FetchError::SubdirIsFile`], and an unset or empty `HOME` as [`FetchError::HomeUnset`].
pub trait Git {
    /// The files of one commit, read into memory once.
    type Tree: FileTree;

    /// The tags of `url` whose name starts with `prefix`, with the commit each names, in the order
    /// git lists them (byte order of the tag name). Empty when no tag matches.
    ///
    /// # Errors
    ///
    /// The [`FetchError`] for the failure, as listed on [`Git`].
    fn list_tags(&self, url: &GitUrl, prefix: &str) -> Result<Vec<RemoteTag>, FetchError>;

    /// Fetches `tag` into the cache and returns the commit it names. An annotated tag gives the
    /// commit under it.
    ///
    /// # Errors
    ///
    /// The [`FetchError`] for the failure, as listed on [`Git`].
    fn fetch_tag(&self, url: &GitUrl, tag: &GitTag) -> Result<CommitId, FetchError>;

    /// The files under `subdir` of commit `rev`, or of the whole commit when `subdir` is `None`.
    ///
    /// The commit is fetched first only when the cache does not hold it; a commit that is cached
    /// needs no network. A symlink in the source is an entry of kind
    /// [`EntryKind::Symlink`](crate::port::EntryKind::Symlink) and a submodule an entry of kind
    /// [`EntryKind::Other`](crate::port::EntryKind::Other). `at` is the pin the source names, the
    /// word a `subdir` refusal uses for where the directory was not found.
    ///
    /// # Errors
    ///
    /// The [`FetchError`] for the failure, as listed on [`Git`].
    fn tree(
        &self,
        url: &GitUrl,
        rev: &CommitId,
        subdir: Option<&SubPath>,
        at: &Pin,
    ) -> Result<Self::Tree, FetchError>;
}
