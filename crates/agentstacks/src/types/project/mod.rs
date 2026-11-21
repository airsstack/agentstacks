//! The project file `agentstacks.toml`, checked field by field into one value, and the source values it is built from.
//!
//! Both the project file and the lock say where a pack or tool comes from, so the source rules
//! live in one place that both read.
//!
//! # Responsibilities
//!
//! - [`ProjectFile`]: parses the whole file and returns the tools and packs it lists, or every refusal at once.
//! - [`PackEntry`]: one `[packs.<name>]` entry, its source plus the `options` table kept raw.
//! - [`Source`]: where a pack or tool comes from, a git repository with a pin or a directory in the project.
//! - [`SourceFields`]: the checked source keys (`git`, `path`, `tag`, `rev`, `subdir`) of one entry, read under a pin rule.
//! - [`Pin`]: what a git source is fixed to, a tag or a commit.
//! - [`PinRule`]: which pin keys a git source must carry, so one reader serves the project file and the lock.
//! - [`GitUrl`]: a repository URL that cannot be taken for a command-line option.
//! - [`GitTag`]: a non-empty git tag name.
//! - [`CommitId`]: a full-length lowercase hex commit id.
//! - [`SubPath`]: a relative path that only descends, inside a repository or the project.
//! - [`LocalPath`]: a relative path inside the project whose `..` segments never climb out of it.
//! - [`InvalidGitUrl`], [`InvalidGitTag`], [`InvalidCommitId`], [`InvalidSubPath`],
//!   [`InvalidLocalPath`]: the refusal each of those constructors returns.
//! - [`SourceError`]: the refusals of the source values, wrapping those above.
//! - [`ProjectError`]: the refusals of the project file with a wording of their own.
//!
//! # Non-responsibilities
//!
//! Rules that need fetched content (an entry's key matching the name inside its source, a pack
//! requiring a pack the file does not list, option typing) belong to the checks over the loaded
//! packs. Whether a URL, tag or path exists is decided when it is fetched or read.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod entry;
pub mod error;
pub mod file;
pub mod git;
pub mod invalid;
pub mod path;
pub mod pin;
pub mod source;

#[cfg(test)]
mod testing;

#[doc(inline)]
pub use entry::PackEntry;
#[doc(inline)]
pub use error::ProjectError;
#[doc(inline)]
pub use file::ProjectFile;
#[doc(inline)]
pub use git::CommitId;
#[doc(inline)]
pub use git::GitTag;
#[doc(inline)]
pub use git::GitUrl;
#[doc(inline)]
pub use invalid::InvalidCommitId;
#[doc(inline)]
pub use invalid::InvalidGitTag;
#[doc(inline)]
pub use invalid::InvalidGitUrl;
#[doc(inline)]
pub use invalid::InvalidLocalPath;
#[doc(inline)]
pub use invalid::InvalidSubPath;
#[doc(inline)]
pub use path::LocalPath;
#[doc(inline)]
pub use path::SubPath;
#[doc(inline)]
pub use pin::Pin;
#[doc(inline)]
pub use pin::PinRule;
#[doc(inline)]
pub use source::Source;
#[doc(inline)]
pub use source::SourceError;
#[doc(inline)]
pub use source::SourceFields;
