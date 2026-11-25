//! The newest release of the official repository, found for `init` and `ext add`.
//!
//! # Responsibilities
//!
//! - [`newest_release`] lists the `packs-v*` tags of a repository and returns the newest one whose
//!   major version the rule of [`official_major`] allows.
//! - [`NewestError`] is why it found none: git refused, or no tag has the major version.
//!
//! # Non-responsibilities
//!
//! Which tag is newest and which major is allowed are rules in `types`. Where a refusal is
//! located, and merging the answers of many arguments for a missing git, belong to the flow that
//! calls this.

use crate::port::Git;
use crate::types::project::{GitTag, GitUrl, ProjectError};
use crate::types::report::FormatError;
use crate::types::resolve::{FetchError, latest_tag};
use crate::types::shorthand::{TAG_PREFIX, official_major, release_prefix};

/// Why no release tag was found.
#[derive(Debug)]
pub(super) enum NewestError {
    /// Git refused, or could not be run.
    Fetch(FetchError),
    /// The repository lists no release tag of the major version the `api` allows.
    NoTag(ProjectError),
}

impl From<NewestError> for FormatError {
    fn from(error: NewestError) -> Self {
        match error {
            NewestError::Fetch(error) => Self::from(error),
            NewestError::NoTag(error) => Self::from(error),
        }
    }
}

/// The newest release tag of `url`.
pub(super) fn newest_release<G: Git>(git: &G, url: &GitUrl) -> Result<GitTag, NewestError> {
    let tags = git.list_tags(url, TAG_PREFIX).map_err(NewestError::Fetch)?;
    latest_tag(&tags, TAG_PREFIX, official_major(url, TAG_PREFIX))
        .map(|(tag, _)| tag)
        .ok_or_else(|| {
            NewestError::NoTag(ProjectError::NoReleaseTag {
                prefix: release_prefix(),
                url: url.to_string(),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeGit;
    use crate::types::project::CommitId;
    use crate::types::resolve::RemoteTag;
    use crate::types::shorthand::OFFICIAL_REPOSITORY;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    fn url() -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(OFFICIAL_REPOSITORY)?)
    }

    fn listing(names: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        names
            .iter()
            .map(|name| {
                Ok(RemoteTag {
                    rev: CommitId::new(REV)?,
                    tag: GitTag::new(name)?,
                })
            })
            .collect()
    }

    #[test]
    fn newest_tag_of_the_allowed_major() -> TestResult {
        let git = FakeGit::new().tags(
            &url()?,
            listing(&[
                "packs-v1.0.0",
                "packs-v1.2.0",
                "packs-v1.10.0",
                "packs-v2.0.0",
            ])?,
        );
        let tag =
            newest_release(&git, &url()?).map_err(|error| FormatError::from(error).to_string())?;
        assert_eq!(tag.as_str(), "packs-v1.10.0");
        Ok(())
    }

    #[test]
    fn no_tag_of_the_major() -> TestResult {
        let git = FakeGit::new().tags(&url()?, listing(&["packs-v0.2.0", "packs-v2.0.0"])?);
        let error = newest_release(&git, &url()?)
            .err()
            .ok_or("expected no tag")?;
        assert!(matches!(error, NewestError::NoTag(_)), "{error:?}");
        assert_eq!(
            FormatError::from(error).to_string(),
            "no packs-v1 release tag in https://github.com/airsstack/agentstacks"
        );
        Ok(())
    }

    #[test]
    fn fetch_error_is_carried() -> TestResult {
        let git = FakeGit::new().git_missing();
        let error = newest_release(&git, &url()?)
            .err()
            .ok_or("expected a fetch error")?;
        assert!(
            matches!(error, NewestError::Fetch(FetchError::GitNotFound { .. })),
            "{error:?}"
        );
        Ok(())
    }
}
