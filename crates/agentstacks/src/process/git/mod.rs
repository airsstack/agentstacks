//! The `Git` port on the `git` command line.
//!
//! # Responsibilities
//!
//! - [`GitCli`]: lists the tags of a remote, fetches a tag or a commit into the cache and reads a
//!   commit's files, by running `git`.
//! - [`Snapshot`]: the files of one commit, read into memory once and served as a `FileTree`.
//! - `command` runs one `git` command the way every source needs it, and `blobs` reads object
//!   bytes through `cat-file --batch`; both serve [`GitCli`] only.
//!
//! # Non-responsibilities
//!
//! No rule about what the output means: reading tag lines and tree records, the `subdir` checks and
//! the cache layout belong to `types::resolve`, and every disk step of the cache to `fs::cache`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod blobs;
mod cli;
mod command;
mod snapshot;
#[cfg(test)]
mod testing;

#[doc(inline)]
pub use cli::GitCli;
#[doc(inline)]
pub use snapshot::Snapshot;
