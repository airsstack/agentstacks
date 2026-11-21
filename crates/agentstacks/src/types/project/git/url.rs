//! The URL of a git repository.
//!
//! It is a type of its own because the URL ends up on a `git` command line: refusing a leading `-`
//! at construction means no later code can pass an option-shaped value by mistake.
//!
//! # Responsibilities
//!
//! - [`GitUrl`] is a non-empty string that does not start with `-`, so it can never be read as an
//!   option when it is passed to git.
//!
//! # Non-responsibilities
//!
//! Whether the URL is reachable, or even a URL, is decided when it is fetched.

use std::fmt;
use std::str::FromStr;

use crate::types::project::InvalidGitUrl;
use crate::types::report::FieldError;

/// A git URL that cannot be taken for a command-line option.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GitUrl(String);

impl GitUrl {
    /// Parses `text` as a git URL.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidGitUrl::Empty`] for an empty or whitespace-only string, and
    /// [`InvalidGitUrl::LeadingDash`] for one that starts with `-`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{GitUrl, InvalidGitUrl};
    ///
    /// let url = GitUrl::new("https://github.com/airsstack/agentstacks")?;
    /// assert_eq!(url.as_str(), "https://github.com/airsstack/agentstacks");
    ///
    /// // A value that git would read as an option is refused.
    /// assert_eq!(GitUrl::new("--upload-pack=x"), Err(InvalidGitUrl::LeadingDash));
    /// # Ok::<(), InvalidGitUrl>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidGitUrl> {
        FieldError::check_not_empty(text).map_err(|_| InvalidGitUrl::Empty)?;
        if text.starts_with('-') {
            return Err(InvalidGitUrl::LeadingDash);
        }
        Ok(Self(text.to_owned()))
    }

    /// The URL as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for GitUrl {
    type Err = InvalidGitUrl;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl fmt::Display for GitUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn git_url_accepts_the_documented_url() -> TestResult {
        let text = "https://github.com/airsstack/agentstacks";
        let url = GitUrl::new(text)?;
        assert_eq!(url.as_str(), text);
        assert_eq!(url.to_string(), text);
        assert_eq!(text.parse::<GitUrl>()?, url);
        Ok(())
    }

    #[test]
    fn git_url_refuses_a_leading_dash_or_empty() {
        for text in ["-x", "--upload-pack=x"] {
            let shown = GitUrl::new(text).err().map(|e| e.to_string());
            assert_eq!(shown.as_deref(), Some("URL must not start with \"-\""));
        }
        for text in ["", " \t"] {
            let shown = GitUrl::new(text).err().map(|e| e.to_string());
            assert_eq!(shown.as_deref(), Some("empty"));
        }
    }

    #[test]
    fn git_url_keeps_a_dash_that_is_not_leading() -> TestResult {
        assert_eq!(GitUrl::new("a-b")?.as_str(), "a-b");
        Ok(())
    }
}
