//! The newest release of the official repository, found for `init` and `ext add`.
//!
//! # Responsibilities
//!
//! - [`newest_release`] lists the tags of a repository under a stream prefix and returns the
//!   newest one of the allowed major, when one is given.
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

/// The newest release tag of `url` under `prefix`, limited to `major` when one is given.
pub(super) fn newest_release<G: Git>(
    git: &G,
    url: &GitUrl,
    prefix: &str,
    major: Option<u64>,
) -> Result<GitTag, NewestError> {
    let tags = git.list_tags(url, prefix).map_err(NewestError::Fetch)?;
    latest_tag(&tags, prefix, major)
        .map(|(tag, _)| tag)
        .ok_or_else(|| {
            NewestError::NoTag(ProjectError::NoReleaseTag {
                prefix: major.map_or_else(|| prefix.to_owned(), |major| format!("{prefix}{major}")),
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
        let tag = newest_release(&git, &url()?, "packs-v", Some(1))
            .map_err(|error| FormatError::from(error).to_string())?;
        assert_eq!(tag.as_str(), "packs-v1.10.0");
        Ok(())
    }

    #[test]
    fn no_tag_of_the_major() -> TestResult {
        let git = FakeGit::new().tags(&url()?, listing(&["packs-v0.2.0", "packs-v2.0.0"])?);
        let error = newest_release(&git, &url()?, "packs-v", Some(1))
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
        let error = newest_release(&git, &url()?, "packs-v", Some(1))
            .err()
            .ok_or("expected a fetch error")?;
        assert!(
            matches!(error, NewestError::Fetch(FetchError::GitNotFound { .. })),
            "{error:?}"
        );
        Ok(())
    }

    #[test]
    fn prerelease_never_wins_a_stream() -> TestResult {
        let git = FakeGit::new().tags(
            &url()?,
            listing(&["packs-core-v1.1.0-rc.1", "packs-core-v1.0.0"])?,
        );
        let tag = newest_release(&git, &url()?, "packs-core-v", Some(1))
            .map_err(|error| FormatError::from(error).to_string())?;
        assert_eq!(tag.as_str(), "packs-core-v1.0.0");
        Ok(())
    }

    #[test]
    fn third_party_skips_major_filter() -> TestResult {
        let other = GitUrl::new("https://github.com/airsstack/agentstacks.git")?;
        let git = FakeGit::new().tags(
            &other,
            listing(&["packs-core-v1.0.0", "packs-core-v9.0.0"])?,
        );
        let tag = newest_release(&git, &other, "packs-core-v", None)
            .map_err(|error| FormatError::from(error).to_string())?;
        assert_eq!(tag.as_str(), "packs-core-v9.0.0");
        Ok(())
    }

    #[test]
    fn lookalike_prefix_does_not_match() -> TestResult {
        let git = FakeGit::new().tags(
            &url()?,
            listing(&["packs-core-v1.0.1", "packs-core-v-extra-v1.0.0"])?,
        );
        let tag = newest_release(&git, &url()?, "packs-core-v", Some(1))
            .map_err(|error| FormatError::from(error).to_string())?;
        assert_eq!(tag.as_str(), "packs-core-v1.0.1");
        Ok(())
    }
}
