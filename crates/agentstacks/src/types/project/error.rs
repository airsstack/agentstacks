//! The messages of `agentstacks.toml` with a wording of their own.
//!
//! The generic shapes belong to [`FieldError`](crate::types::FieldError) and the source messages to
//! [`SourceError`]; every other refusal the project file can meet is one
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

use crate::types::project::{GitOption, SourceError};
use crate::types::report::AlreadyGiven;
use crate::types::report::LegacyProject;
use crate::types::report::UnsupportedApi;

/// A refusal of `agentstacks.toml` as a whole, of an entry's name, or of the arguments `init` and
/// `ext add` write it from.
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
    /// `ext add` found `tools.<name>` already in the file.
    #[error("tools.{name} already exists (edit it there)")]
    ToolExists {
        /// The tool name that is already listed.
        name: String,
    },
    /// `ext add` found the file different from the one it read first, or no longer a project file
    /// it can add a tool to.
    #[error("changed during ext add (run it again)")]
    ChangedDuringAdd,
    /// `ext add` was given a git URL with neither `--tag` nor `--rev`.
    #[error("a git URL needs --tag or --rev")]
    GitNeedsPin,
    /// `ext add` was given both `--tag` and `--rev`.
    #[error("--tag and --rev cannot be used together")]
    PinConflict,
    /// `ext add` was given a git-only option with a path or an official name.
    #[error("{flag} applies only to a git URL")]
    FlagNeedsGit {
        /// The option that was given.
        flag: GitOption,
    },
    /// `ext add` was given an option value that cannot be held.
    #[error("{flag}: {error}")]
    FlagValue {
        /// The option that was given.
        flag: GitOption,
        /// Why the value is refused.
        error: SourceError,
    },
    /// `init` found an `agentstacks.toml` already there.
    #[error("already exists (edit it and run apply)")]
    AlreadyExists,
    /// The official repository lists no release tag of the prefix and major that `prefix` names.
    #[error("no {prefix} release tag in {url}")]
    NoReleaseTag {
        /// The tag prefix searched for.
        prefix: String,
        /// The repository that was listed.
        url: String,
    },
    /// Two `init` arguments of one option resolve to the same `key`; `first` is the flag text of
    /// the argument that gave it first.
    #[error(transparent)]
    AlreadyGiven(#[from] AlreadyGiven),
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
            ProjectError::ToolExists {
                name: "claude".to_owned()
            }
            .to_string(),
            "tools.claude already exists (edit it there)"
        );
        assert_eq!(
            ProjectError::ChangedDuringAdd.to_string(),
            "changed during ext add (run it again)"
        );
        assert_eq!(
            ProjectError::GitNeedsPin.to_string(),
            "a git URL needs --tag or --rev"
        );
        assert_eq!(
            ProjectError::PinConflict.to_string(),
            "--tag and --rev cannot be used together"
        );
        assert_eq!(
            ProjectError::FlagNeedsGit {
                flag: GitOption::Tag
            }
            .to_string(),
            "--tag applies only to a git URL"
        );
        assert_eq!(
            ProjectError::FlagValue {
                flag: GitOption::Rev,
                error: SourceError::from(crate::types::InvalidCommitId::NotACommitId)
            }
            .to_string(),
            "--rev: not a commit id"
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
            ProjectError::from(AlreadyGiven::new("pack", "core", "--pack \"core\"")).to_string(),
            "pack \"core\" is already given by --pack \"core\""
        );
    }
}
