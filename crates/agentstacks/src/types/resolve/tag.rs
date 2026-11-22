//! Splitting a tag into prefix and version, and choosing the latest tag.
//!
//! It is separate from reading `ls-remote` because the rules compare values only, so they run
//! over tags from any source.
//!
//! # Responsibilities
//!
//! - [`TagVersion::split`] splits a tag at its version: the version starts at the leftmost ASCII
//!   digit whose remainder is a semantic version, and the prefix is everything before it.
//! - [`latest_tag`] picks the highest released version among the tags that share a prefix, by
//!   semver precedence and not by the order git lists them.
//!
//! # Non-responsibilities
//!
//! Listing the tags and writing the chosen one into `agentstacks.toml` belong to the flows.

use std::cmp::Ordering;

use crate::types::manifest::PackVersion;
use crate::types::project::GitTag;
use crate::types::resolve::RemoteTag;

/// A tag split into the text before its version and the version.
///
/// # Examples
///
/// ```
/// use agentstacks::types::GitTag;
/// use agentstacks::types::resolve::TagVersion;
///
/// let split = TagVersion::split(&GitTag::new("packs-v0.1.0")?).ok_or("no version")?;
/// assert_eq!(split.prefix, "packs-v");
/// assert_eq!(split.version.to_string(), "0.1.0");
/// assert!(TagVersion::split(&GitTag::new("main-latest")?).is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagVersion {
    /// Everything before the version.
    pub prefix: String,
    /// The version the tag ends in.
    pub version: PackVersion,
}

impl TagVersion {
    /// Splits `tag` at the leftmost ASCII digit whose remainder parses as a semantic version, or
    /// returns `None` when no position does.
    #[must_use]
    pub fn split(tag: &GitTag) -> Option<Self> {
        let text = tag.as_str();
        text.char_indices()
            .filter(|(_, ch)| ch.is_ascii_digit())
            .find_map(|(at, _)| {
                PackVersion::new(&text[at..]).ok().map(|version| Self {
                    prefix: text[..at].to_owned(),
                    version,
                })
            })
    }
}

/// How `a` ranks against `b` by semver precedence, which ignores build metadata.
pub(super) fn precedence(a: &PackVersion, b: &PackVersion) -> Ordering {
    a.as_version().cmp_precedence(b.as_version())
}

/// The highest released tag that starts with `prefix` and continues with a semantic version.
///
/// Tags with another prefix, a suffix that is not semver, or a pre-release are ignored. The tags
/// rank by semver precedence; among tags of equal precedence (they differ only in build
/// metadata) the greater tag in byte order wins. `None` when no tag qualifies.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{CommitId, GitTag};
/// use agentstacks::types::resolve::{RemoteTag, latest_tag};
///
/// let rev = CommitId::new("90e2a535db371753890122ed088c81bf3867be7a")?;
/// let tags = ["packs-v0.9.0", "packs-v0.10.0", "packs-v0.2.0"]
///     .iter()
///     .map(|tag| Ok(RemoteTag { rev: rev.clone(), tag: GitTag::new(tag)? }))
///     .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
/// let (latest, version) = latest_tag(&tags, "packs-v").ok_or("no tag")?;
/// assert_eq!(latest.as_str(), "packs-v0.10.0");
/// assert_eq!(version.to_string(), "0.10.0");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn latest_tag(tags: &[RemoteTag], prefix: &str) -> Option<(GitTag, PackVersion)> {
    tags.iter()
        .filter_map(|remote| {
            let suffix = remote.tag.as_str().strip_prefix(prefix)?;
            let version = PackVersion::new(suffix).ok()?;
            version
                .as_version()
                .pre
                .is_empty()
                .then(|| (remote.tag.clone(), version))
        })
        .max_by(|(tag_a, a), (tag_b, b)| {
            precedence(a, b).then_with(|| tag_a.as_str().cmp(tag_b.as_str()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::project::CommitId;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const NEW: &str = "90e2a535db371753890122ed088c81bf3867be7a";

    fn remote(names: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        names
            .iter()
            .map(|name| {
                Ok(RemoteTag {
                    rev: CommitId::new(NEW)?,
                    tag: GitTag::new(name)?,
                })
            })
            .collect()
    }

    fn latest(names: &[&str], prefix: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(latest_tag(&remote(names)?, prefix).map(|(tag, _)| tag.to_string()))
    }

    #[test]
    fn split_packs_v() -> TestResult {
        let split = TagVersion::split(&GitTag::new("packs-v0.1.0")?).ok_or("no split")?;
        assert_eq!(split.prefix, "packs-v");
        assert_eq!(split.version, PackVersion::new("0.1.0")?);
        Ok(())
    }

    #[test]
    fn split_takes_the_leftmost_digit_that_parses() -> TestResult {
        let split = TagVersion::split(&GitTag::new("x-1.2.3-4.5.6")?).ok_or("no split")?;
        assert_eq!(split.prefix, "x-");
        assert_eq!(split.version, PackVersion::new("1.2.3-4.5.6")?);
        Ok(())
    }

    #[test]
    fn split_without_a_version_is_none() -> TestResult {
        assert_eq!(TagVersion::split(&GitTag::new("main-latest")?), None);
        Ok(())
    }

    #[test]
    fn latest_is_by_semver_not_lexical() -> TestResult {
        let names = [
            "packs-v0.1.0",
            "packs-v0.10.0",
            "packs-v0.2.0",
            "packs-v0.9.0",
        ];
        assert_eq!(latest(&names, "packs-v")?.as_deref(), Some("packs-v0.10.0"));
        let (_, version) = latest_tag(&remote(&names)?, "packs-v").ok_or("no tag")?;
        assert_eq!(version, PackVersion::new("0.10.0")?);
        Ok(())
    }

    #[test]
    fn latest_skips_pre_releases() -> TestResult {
        let names = ["packs-v0.3.0-rc.1", "packs-v0.2.0"];
        assert_eq!(latest(&names, "packs-v")?.as_deref(), Some("packs-v0.2.0"));
        Ok(())
    }

    #[test]
    fn latest_skips_other_prefixes_and_non_semver() -> TestResult {
        let names = ["other-v9.9.9", "packs-vnext", "packs-v0.1.0"];
        assert_eq!(latest(&names, "packs-v")?.as_deref(), Some("packs-v0.1.0"));
        Ok(())
    }

    #[test]
    fn latest_breaks_build_metadata_ties_by_tag_bytes() -> TestResult {
        for names in [
            ["packs-v1.0.0+a", "packs-v1.0.0+b"],
            ["packs-v1.0.0+b", "packs-v1.0.0+a"],
        ] {
            assert_eq!(
                latest(&names, "packs-v")?.as_deref(),
                Some("packs-v1.0.0+b")
            );
        }
        Ok(())
    }

    #[test]
    fn latest_of_nothing_is_none() -> TestResult {
        assert_eq!(latest(&[], "packs-v")?, None);
        assert_eq!(latest(&["packs-v1.0.0-rc.1"], "packs-v")?, None);
        Ok(())
    }
}
