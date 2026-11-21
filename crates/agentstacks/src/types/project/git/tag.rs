//! A git tag a source is pinned to.
//!
//! It is a type of its own so a tag cannot be mixed up with a commit id ([`Pin`](crate::types::Pin)
//! tells the two apart by type) and a blank tag is refused once, at construction.
//!
//! # Responsibilities
//!
//! - [`GitTag`] is a non-empty string that is not only whitespace.
//!
//! # Non-responsibilities
//!
//! Whether the tag exists in the repository is decided when it is fetched.

use std::fmt;
use std::str::FromStr;

use crate::types::project::InvalidGitTag;
use crate::types::report::FieldError;

/// A git tag name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitTag(String);

impl GitTag {
    /// Parses `text` as a git tag.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidGitTag`] for an empty or whitespace-only string.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::GitTag;
    ///
    /// let tag = GitTag::new("packs-v0.1.0")?;
    /// assert_eq!(tag.as_str(), "packs-v0.1.0");
    ///
    /// assert!(GitTag::new("  ").is_err());
    /// # Ok::<(), agentstacks::types::InvalidGitTag>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidGitTag> {
        FieldError::check_not_empty(text).map_err(|_| InvalidGitTag)?;
        Ok(Self(text.to_owned()))
    }

    /// The tag as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for GitTag {
    type Err = InvalidGitTag;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl fmt::Display for GitTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for GitTag {
    type Error = InvalidGitTag;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn git_tag_accepts_a_tag() -> TestResult {
        let tag = GitTag::new("packs-v0.1.0")?;
        assert_eq!(tag.as_str(), "packs-v0.1.0");
        assert_eq!(tag.to_string(), "packs-v0.1.0");
        assert_eq!("packs-v0.1.0".parse::<GitTag>()?, tag);
        Ok(())
    }

    #[test]
    fn git_tag_refuses_empty_or_whitespace() {
        for text in ["", "  "] {
            let shown = GitTag::new(text).err().map(|e| e.to_string());
            assert_eq!(shown.as_deref(), Some("empty"));
        }
    }

    #[test]
    fn git_tag_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<GitTag, InvalidGitTag>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["v1", "  "] {
            let expected = shown(GitTag::new(text));
            assert_eq!(shown(text.parse::<GitTag>()), expected);
            assert_eq!(shown(GitTag::try_from(text)), expected);
        }
        assert!(GitTag::new("v1").is_ok());
        assert!(GitTag::new("  ").is_err());
    }
}
