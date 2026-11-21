//! The messages of the project file's sources with a wording of their own.
//!
//! The generic shapes (`required`, `expected <type>, got <type>`, `empty`, `unknown key`) belong to
//! [`FieldError`]; every other refusal a source can meet is one variant here, so a caller can
//! match on it and the wording lives in one place.
//!
//! It is separate from [`ProjectError`](crate::types::ProjectError) because the lock reads sources
//! too and shares these messages, whereas `ProjectError` belongs to the project file alone.
//!
//! # Responsibilities
//!
//! - [`SourceError`] has a variant per source refusal, each displaying exactly the message the
//!   author reads after the location.
//! - It wraps the error of each source value (URL, tag, commit id, sub path, local path) and
//!   [`FieldError`] transparently, so an empty string reads `empty` as it does everywhere else.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it, and which key raises which refusal belongs to the readers of the sources.

use crate::types::project::InvalidCommitId;
use crate::types::project::InvalidGitTag;
use crate::types::project::InvalidGitUrl;
use crate::types::project::InvalidLocalPath;
use crate::types::project::InvalidSubPath;
use crate::types::report::FieldError;

/// A refusal of a pack or extension source in `agentstacks.toml` or `agentstacks.lock`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitUrl, SourceError};
///
/// let refusal = GitUrl::new("--upload-pack=x").err().map(SourceError::from);
/// assert_eq!(
///     refusal.map(|error| error.to_string()).as_deref(),
///     Some("URL must not start with \"-\"")
/// );
/// assert_eq!(SourceError::GitOrPath.to_string(), "needs exactly one of git, path");
/// assert_eq!(SourceError::TagOrRev.to_string(), "needs exactly one of tag, rev");
/// assert_eq!(
///     SourceError::GitOnlyFields.to_string(),
///     "tag, rev and subdir apply only to git sources"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// A `git` URL that is blank or that git would read as one of its own options.
    #[error(transparent)]
    GitUrl(#[from] InvalidGitUrl),
    /// A blank `tag`.
    #[error(transparent)]
    GitTag(#[from] InvalidGitTag),
    /// A `rev` that is blank or not 40 or 64 lowercase hex characters.
    #[error(transparent)]
    CommitId(#[from] InvalidCommitId),
    /// A `subdir` that is blank, absolute or has a `..` segment.
    #[error(transparent)]
    SubPath(#[from] InvalidSubPath),
    /// A `path` that is blank, absolute or climbs above the project directory.
    #[error(transparent)]
    LocalPath(#[from] InvalidLocalPath),
    /// An entry with both or neither of `git` and `path`.
    #[error("needs exactly one of git, path")]
    GitOrPath,
    /// A git entry with both or neither of `tag` and `rev`.
    #[error("needs exactly one of tag, rev")]
    TagOrRev,
    /// A `tag`, `rev` or `subdir` on a path entry.
    #[error("tag, rev and subdir apply only to git sources")]
    GitOnlyFields,
    /// One of the generic field message shapes.
    #[error(transparent)]
    Field(#[from] FieldError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_error_messages_are_the_refusal_texts() {
        let cases = [
            (
                SourceError::from(InvalidGitUrl::LeadingDash),
                "URL must not start with \"-\"",
            ),
            (SourceError::from(InvalidGitTag), "empty"),
            (
                SourceError::from(InvalidCommitId::NotACommitId),
                "not a commit id",
            ),
            (
                SourceError::from(InvalidSubPath::OutsideRepository),
                "must stay inside the repository",
            ),
            (
                SourceError::from(InvalidLocalPath::AbsolutePath),
                "absolute paths are not portable",
            ),
            (
                SourceError::from(InvalidLocalPath::OutsideProject),
                "must stay inside the project",
            ),
            (SourceError::GitOrPath, "needs exactly one of git, path"),
            (SourceError::TagOrRev, "needs exactly one of tag, rev"),
            (
                SourceError::GitOnlyFields,
                "tag, rev and subdir apply only to git sources",
            ),
            (SourceError::Field(FieldError::Empty), "empty"),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
