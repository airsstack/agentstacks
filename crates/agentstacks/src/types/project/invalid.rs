//! The refusals returned when a git URL, tag, commit id or path cannot be held.
//!
//! Each of the five source values ([`GitUrl`](crate::types::GitUrl), [`GitTag`](crate::types::GitTag),
//! [`CommitId`](crate::types::CommitId), [`SubPath`](crate::types::SubPath),
//! [`LocalPath`](crate::types::LocalPath)) refuses with its own small error type. They are small
//! closed enums with fixed wording and share one reason to change, so they sit together apart from
//! the constructors that return them.
//!
//! # Responsibilities
//!
//! - [`InvalidGitUrl`]: a blank URL, or one starting with `-`.
//! - [`InvalidGitTag`]: a blank tag.
//! - [`InvalidCommitId`]: a blank `rev`, or one that is not 40 or 64 lowercase hex characters.
//! - [`InvalidSubPath`]: a blank path, or one that is absolute or has a `..` segment.
//! - [`InvalidLocalPath`]: a blank path, an absolute one, or one climbing above the project directory.
//!
//! Each variant displays the message the author reads after the location.
//!
//! # Non-responsibilities
//!
//! Whether a URL is reachable, a tag or commit exists, or a directory exists is decided when it
//! is fetched or read.

/// A text that cannot be a [`GitUrl`](crate::types::GitUrl).
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitUrl, InvalidGitUrl};
///
/// assert_eq!(GitUrl::new("--upload-pack=x"), Err(InvalidGitUrl::LeadingDash));
/// assert_eq!(GitUrl::new("  "), Err(InvalidGitUrl::Empty));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidGitUrl {
    /// An empty or whitespace-only URL.
    #[error("empty")]
    Empty,
    /// A URL that starts with `-`, which git would read as one of its own options.
    #[error("URL must not start with \"-\"")]
    LeadingDash,
}

/// A text that cannot be a [`GitTag`](crate::types::GitTag): it is empty or only whitespace.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, InvalidGitTag};
///
/// assert_eq!(GitTag::new("  "), Err(InvalidGitTag));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("empty")]
pub struct InvalidGitTag;

/// A text that cannot be a [`CommitId`](crate::types::CommitId).
///
/// # Examples
///
/// ```
/// use agentstacks::types::{CommitId, InvalidCommitId};
///
/// assert_eq!(CommitId::new("0123abc"), Err(InvalidCommitId::NotACommitId));
/// assert_eq!(CommitId::new(""), Err(InvalidCommitId::Empty));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidCommitId {
    /// An empty or whitespace-only `rev`.
    #[error("empty")]
    Empty,
    /// A `rev` that is not 40 or 64 lowercase hex characters.
    #[error("not a commit id")]
    NotACommitId,
}

/// A text that cannot be a [`SubPath`](crate::types::SubPath).
///
/// A project path that may climb with `..` and come back has its own refusal, [`InvalidLocalPath`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::{InvalidSubPath, SubPath};
///
/// assert_eq!(SubPath::new("../core"), Err(InvalidSubPath::OutsideRepository));
/// assert_eq!(SubPath::new(""), Err(InvalidSubPath::Empty));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidSubPath {
    /// An empty or whitespace-only path.
    #[error("empty")]
    Empty,
    /// A path that is absolute or has a `..` segment.
    #[error("must stay inside the repository")]
    OutsideRepository,
}

/// A text that cannot be a [`LocalPath`](crate::types::LocalPath).
///
/// # Examples
///
/// ```
/// use agentstacks::types::{InvalidLocalPath, LocalPath};
///
/// assert_eq!(LocalPath::new("../outside"), Err(InvalidLocalPath::OutsideProject));
/// assert_eq!(LocalPath::new("/etc"), Err(InvalidLocalPath::AbsolutePath));
/// assert_eq!(LocalPath::new(""), Err(InvalidLocalPath::Empty));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidLocalPath {
    /// An empty or whitespace-only path.
    #[error("empty")]
    Empty,
    /// A path that is absolute.
    #[error("absolute paths are not portable")]
    AbsolutePath,
    /// A path whose `..` segments climb above the project directory.
    #[error("must stay inside the project")]
    OutsideProject,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_git_url_displays_each_message() {
        assert_eq!(InvalidGitUrl::Empty.to_string(), "empty");
        assert_eq!(
            InvalidGitUrl::LeadingDash.to_string(),
            "URL must not start with \"-\""
        );
    }

    #[test]
    fn invalid_git_tag_displays_empty() {
        assert_eq!(InvalidGitTag.to_string(), "empty");
    }

    #[test]
    fn invalid_commit_id_displays_each_message() {
        assert_eq!(InvalidCommitId::Empty.to_string(), "empty");
        assert_eq!(InvalidCommitId::NotACommitId.to_string(), "not a commit id");
    }

    #[test]
    fn invalid_sub_path_displays_each_message() {
        assert_eq!(InvalidSubPath::Empty.to_string(), "empty");
        assert_eq!(
            InvalidSubPath::OutsideRepository.to_string(),
            "must stay inside the repository"
        );
    }

    #[test]
    fn invalid_local_path_displays_each_message() {
        assert_eq!(InvalidLocalPath::Empty.to_string(), "empty");
        assert_eq!(
            InvalidLocalPath::AbsolutePath.to_string(),
            "absolute paths are not portable"
        );
        assert_eq!(
            InvalidLocalPath::OutsideProject.to_string(),
            "must stay inside the project"
        );
    }
}
