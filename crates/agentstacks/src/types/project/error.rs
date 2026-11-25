//! The messages of `agentstacks.toml` with a wording of their own.
//!
//! The generic shapes belong to [`FieldError`](crate::types::FieldError) and the source messages to
//! [`SourceError`](crate::types::SourceError); every other refusal the project file can meet is one
//! variant here.
//!
//! It is a file of its own because these refusals belong to the project file alone, while the
//! source messages are shared with the lock; keeping them apart keeps each wording in one place.
//!
//! # Responsibilities
//!
//! - [`ProjectError`] has a variant per project-file refusal, each displaying exactly the message
//!   the author reads after the location.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it.

use crate::types::report::LegacyProject;
use crate::types::report::UnsupportedApi;

/// A refusal of `agentstacks.toml` as a whole, of an entry's name, or of the arguments `init`
/// writes it from.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ProjectError;
///
/// assert_eq!(ProjectError::NothingToInstall.to_string(), "nothing to install");
/// assert_eq!(ProjectError::InvalidName.to_string(), "not a valid name");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectError {
    /// An `api` this agentstacks does not speak.
    #[error("api {0}")]
    UnsupportedApi(#[from] UnsupportedApi),
    /// An `api` written for the `0.x` releases, which only `update` goes on with.
    #[error(transparent)]
    LegacyProject(#[from] LegacyProject),
    /// A file that lists no tool or no pack.
    #[error("nothing to install")]
    NothingToInstall,
    /// A `[tools.*]` or `[packs.*]` key that is not a valid name.
    #[error("not a valid name")]
    InvalidName,
    /// A tag rewrite found a tag other than the one it expected: the file changed after `update`
    /// listed the new tags.
    #[error("tag changed during update (run update again)")]
    TagChanged,
    /// A tag rewrite found no `tag` at the entry it was asked to change.
    #[error("no tag to update (run update again)")]
    NoTagToUpdate,
    /// An `api` rewrite found no integer `api` that is the legacy one, because the `api` is
    /// missing, not an integer or another number: the file changed after `update` read it.
    #[error("api changed during update (run update again)")]
    ApiChanged,
    /// `init` found an `agentstacks.toml` already there.
    #[error("already exists (edit it and run apply)")]
    AlreadyExists,
    /// The official repository lists no release tag that starts with `prefix`.
    #[error("no {prefix} release tag in {url}")]
    NoReleaseTag {
        /// The tag prefix searched for.
        prefix: String,
        /// The repository that was listed.
        url: String,
    },
    /// Two `init` arguments of one option resolve to the same `key`; `first` is the flag text of
    /// the argument that gave it first.
    #[error("{noun} \"{key}\" is already given by {first}")]
    AlreadyGiven {
        /// `pack` or `tool`.
        noun: &'static str,
        /// The shared key.
        key: String,
        /// The earlier argument as written, flag included.
        first: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_error_messages_are_the_refusal_texts() {
        assert_eq!(
            ProjectError::from(UnsupportedApi::new(2)).to_string(),
            "api 2 not supported (this agentstacks speaks 1)"
        );
        assert_eq!(
            ProjectError::from(LegacyProject).to_string(),
            "api 0 is from agentstacks 0.x (run agentstacks update)"
        );
        assert_eq!(
            ProjectError::NothingToInstall.to_string(),
            "nothing to install"
        );
        assert_eq!(ProjectError::InvalidName.to_string(), "not a valid name");
        assert_eq!(
            ProjectError::TagChanged.to_string(),
            "tag changed during update (run update again)"
        );
        assert_eq!(
            ProjectError::NoTagToUpdate.to_string(),
            "no tag to update (run update again)"
        );
        assert_eq!(
            ProjectError::ApiChanged.to_string(),
            "api changed during update (run update again)"
        );
        assert_eq!(
            ProjectError::AlreadyExists.to_string(),
            "already exists (edit it and run apply)"
        );
        assert_eq!(
            ProjectError::NoReleaseTag {
                prefix: "packs-v".to_owned(),
                url: "https://x/y".to_owned()
            }
            .to_string(),
            "no packs-v release tag in https://x/y"
        );
        assert_eq!(
            ProjectError::AlreadyGiven {
                noun: "pack",
                key: "core".to_owned(),
                first: "--pack \"core\"".to_owned()
            }
            .to_string(),
            "pack \"core\" is already given by --pack \"core\""
        );
    }
}
