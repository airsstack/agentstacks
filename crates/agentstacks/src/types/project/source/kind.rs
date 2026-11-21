//! Where a pack or an extension comes from.
//!
//! It is separate from the crate-internal `SourceFields` because the fields keep everything as read (including the
//! `tag` written beside a `rev`), while the source is the resolved value the rest of the program
//! uses.
//!
//! # Responsibilities
//!
//! - [`Source`] is a git repository pinned to a tag or a commit, with an optional directory inside
//!   it, or a directory inside the project.
//! - `Source::from_fields` (crate-internal) converts read `SourceFields` into a source. It cannot fail, because
//!   the fields were checked when they were read.
//!
//! # Non-responsibilities
//!
//! Fetching a source and checking what it contains belong to the code that installs it.

use crate::types::project::GitUrl;
use crate::types::project::LocalPath;
use crate::types::project::Pin;
use crate::types::project::SourceFields;
use crate::types::project::SubPath;

/// A pack or extension source.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, GitUrl, LocalPath, Pin, Source, SubPath};
///
/// let git = Source::Git {
///     url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
///     pin: Pin::Tag(GitTag::new("packs-v0.1.0")?),
///     subdir: Some(SubPath::new("extensions/claude")?),
/// };
/// let local = Source::Path(LocalPath::new("packs/team-lint")?);
/// assert_ne!(git, local);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A git repository.
    Git {
        /// The repository URL.
        url: GitUrl,
        /// The tag or commit the source is pinned to.
        pin: Pin,
        /// The directory inside the repository, when the source is not its root.
        subdir: Option<SubPath>,
    },
    /// A directory inside the project.
    Path(LocalPath),
}

impl Source {
    /// The source `fields` describe, keeping the pin the fields resolved (a `rev` wins over a
    /// `tag` written beside it).
    #[must_use]
    pub(in crate::types) fn from_fields(fields: SourceFields) -> Self {
        match fields {
            SourceFields::Git {
                url, pin, subdir, ..
            } => Self::Git { url, pin, subdir },
            SourceFields::Path(path) => Self::Path(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::project::CommitId;
    use crate::types::project::GitTag;
    use crate::types::project::PinRule;
    use crate::types::project::testing::read_fields;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn source_from_fields_pins_a_tag_or_a_rev() -> TestResult {
        let url = GitUrl::new("https://example.org/r")?;
        let tag = GitTag::new("v1")?;
        let rev = CommitId::new(HEX40)?;
        let subdir = SubPath::new("packs/core")?;

        let by_tag = Source::from_fields(SourceFields::Git {
            url: url.clone(),
            pin: Pin::Tag(tag.clone()),
            tag: Some(tag.clone()),
            subdir: Some(subdir.clone()),
        });
        assert_eq!(
            by_tag,
            Source::Git {
                url: url.clone(),
                pin: Pin::Tag(tag),
                subdir: Some(subdir),
            }
        );

        let by_rev = Source::from_fields(SourceFields::Git {
            url: url.clone(),
            pin: Pin::Rev(rev.clone()),
            tag: None,
            subdir: None,
        });
        assert_eq!(
            by_rev,
            Source::Git {
                url,
                pin: Pin::Rev(rev),
                subdir: None,
            }
        );

        let path = LocalPath::new("packs/team-lint")?;
        assert_eq!(
            Source::from_fields(SourceFields::Path(path.clone())),
            Source::Path(path)
        );
        Ok(())
    }

    #[test]
    fn source_from_fields_prefers_rev_over_tag() -> TestResult {
        let entry: toml::Table =
            format!("git = \"https://example.org/r\"\ntag = \"v1\"\nrev = \"{HEX40}\"\n")
                .parse()?;
        let fields = read_fields(
            &entry,
            Path::new("agentstacks.lock"),
            "pack[1]",
            PinRule::RevRequired,
        )
        .map_err(|diagnostics| {
            let shown: Vec<String> = diagnostics.into_iter().map(|d| d.to_string()).collect();
            shown.join("; ")
        })?;
        assert_eq!(
            Source::from_fields(fields),
            Source::Git {
                url: GitUrl::new("https://example.org/r")?,
                pin: Pin::Rev(CommitId::new(HEX40)?),
                subdir: None,
            }
        );
        Ok(())
    }
}
