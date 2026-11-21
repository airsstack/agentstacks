//! The lockfile `agentstacks.lock`, read from text, checked as a whole and written back as text.
//!
//! The lock pins what an install resolved to, so one strict value type serves the reader and the
//! writer and a lock built in memory is held to the rules of one read from disk.
//!
//! # Responsibilities
//!
//! - [`Lockfile`]: the checked lock, parsed from text or built from [`LockContents`], with every list kept sorted.
//! - [`LockContents`]: the typed entries a lock is built from, in any order.
//! - [`LockedEntry`]: one locked tool or pack, with its version, origin and tree hash.
//! - [`LockedSource`]: where a locked entry came from, a git commit or a project directory.
//! - [`FileEntry`]: one file an install wrote, with its path, hash and owner.
//! - [`MergeEntry`]: one merge an install made into a file it does not own.
//! - [`Owner`]: the name of the pack or tool that wrote a file or merge.
//! - [`MergeFile`]: the file a merge changed, as the lock records it.
//! - [`MergeKey`]: the locator of the key a merge changed inside its file.
//! - [`LockError`]: the refusals of the lock with a wording of their own.
//! - [`write_lock()`]: the lock's text, with a fixed field order and one newline at the end.
//! - [`check_airsl_presence()`]: refuses a lock whose `[airsl]` section disagrees with what the locked packs need.
//!
//! # Non-responsibilities
//!
//! A missing lock is not an error here, because the reader takes text. Comparing the lock with
//! `agentstacks.toml`, or a locked hash with the content on disk, belongs to the stage that plans
//! and applies an install. Writing the text to disk belongs to the executor.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod airsl;
pub mod entry;
pub mod error;
pub mod file;
pub mod merge;
pub mod owner;
pub mod write;

#[cfg(test)]
mod testing;

#[doc(inline)]
pub use airsl::check_airsl_presence;
#[doc(inline)]
pub use entry::FileEntry;
#[doc(inline)]
pub use entry::LockedEntry;
#[doc(inline)]
pub use entry::LockedSource;
#[doc(inline)]
pub use entry::MergeEntry;
#[doc(inline)]
pub use error::LockError;
#[doc(inline)]
pub use file::LockContents;
#[doc(inline)]
pub use file::Lockfile;
#[doc(inline)]
pub use merge::MergeFile;
#[doc(inline)]
pub use merge::MergeKey;
#[doc(inline)]
pub use owner::Owner;
#[doc(inline)]
pub use write::write_lock;
