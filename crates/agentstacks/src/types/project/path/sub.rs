//! A relative path that only descends: a directory inside a repository, or a file the lock records
//! inside the project.
//!
//! It is separate from [`LocalPath`](crate::types::LocalPath) because the rules differ: the
//! repository and the lock never need a `..` segment, so none is accepted at all.
//!
//! # Responsibilities
//!
//! - [`SubPath::new`] parses the text: relative, with no `..` segment. It is the only constructor,
//!   so a value that exists already holds that rule. It does not refuse a `\` or agentstacks' own
//!   files, which the lock parse refuses for a file path; the operation path rule keeps those out
//!   before a lock is built.
//! - [`SubPath::normalized`] is the text with every empty and `.` segment removed, so `./a`, `a//`
//!   and `a` compare as one path. The decision is lexical; the file system is never read.
//!
//! # Non-responsibilities
//!
//! A project directory that may climb with `..` and come back is
//! [`LocalPath`](crate::types::LocalPath). Resolving symlinks and checking that the path exists
//! belong to the code that reads it.

use std::fmt;
use std::str::FromStr;

use crate::types::project::InvalidSubPath;
use crate::types::report::FieldError;

/// A relative path with no `..` segment, as the author or the lock wrote it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubPath(String);

impl SubPath {
    /// Parses `text` as a path that only descends.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidSubPath::Empty`] for an empty or whitespace-only string, and
    /// [`InvalidSubPath::OutsideRepository`] for a leading `/` or any `..` segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{InvalidSubPath, SubPath};
    ///
    /// let path = SubPath::new("./packs//core")?;
    /// assert_eq!(path.normalized(), "packs/core");
    ///
    /// assert_eq!(SubPath::new("../core"), Err(InvalidSubPath::OutsideRepository));
    /// assert_eq!(SubPath::new("/core"), Err(InvalidSubPath::OutsideRepository));
    /// # Ok::<(), InvalidSubPath>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidSubPath> {
        FieldError::check_not_empty(text).map_err(|_| InvalidSubPath::Empty)?;
        if text.starts_with('/') || text.split('/').any(|segment| segment == "..") {
            return Err(InvalidSubPath::OutsideRepository);
        }
        Ok(Self(text.to_owned()))
    }

    /// The path as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path with every empty and `.` segment removed, joined by `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::SubPath;
    ///
    /// assert_eq!(SubPath::new("a//./b/")?.normalized(), "a/b");
    /// # Ok::<(), agentstacks::types::InvalidSubPath>(())
    /// ```
    #[must_use]
    pub fn normalized(&self) -> String {
        self.0
            .split('/')
            .filter(|segment| !matches!(*segment, "" | "."))
            .collect::<Vec<_>>()
            .join("/")
    }
}

impl fmt::Display for SubPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SubPath {
    type Err = InvalidSubPath;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for SubPath {
    type Error = InvalidSubPath;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn shown(result: Result<SubPath, InvalidSubPath>) -> Option<String> {
        result.err().map(|error| error.to_string())
    }

    #[test]
    fn sub_path_accepts_relative_paths() -> TestResult {
        for text in ["packs/core", "extensions/claude", "a..b/c", "./a"] {
            let path = SubPath::new(text)?;
            assert_eq!(path.as_str(), text);
            assert_eq!(path.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn sub_path_refuses_escape_and_leading_slash() {
        for text in ["../x", "a/../b", "/x", ".."] {
            assert_eq!(
                shown(SubPath::new(text)).as_deref(),
                Some("must stay inside the repository"),
                "{text}"
            );
        }
        assert_eq!(shown(SubPath::new("")).as_deref(), Some("empty"));
        assert_eq!(shown(SubPath::new("  ")).as_deref(), Some("empty"));
    }

    #[test]
    fn sub_path_normalized_drops_empty_and_dot_segments() -> TestResult {
        for (text, expected) in [
            ("a", "a"),
            ("./a", "a"),
            ("a//b", "a/b"),
            ("a/", "a"),
            ("./a/./b/", "a/b"),
            (".claude/x", ".claude/x"),
        ] {
            assert_eq!(SubPath::new(text)?.normalized(), expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn sub_path_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<SubPath, InvalidSubPath>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["a/b", "../core"] {
            let expected = shown(SubPath::new(text));
            assert_eq!(shown(text.parse::<SubPath>()), expected);
            assert_eq!(shown(SubPath::try_from(text)), expected);
        }
        assert!(SubPath::new("a/b").is_ok());
        assert!(SubPath::new("../core").is_err());
    }
}
