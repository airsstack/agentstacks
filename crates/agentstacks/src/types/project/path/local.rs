//! A directory inside the project, as `path` names a pack or tool source.
//!
//! It is separate from [`SubPath`](crate::types::SubPath) because the rules differ: a `..` segment
//! is allowed here as long as the path never climbs out of the project.
//!
//! # Responsibilities
//!
//! - [`LocalPath::new`] parses the text: relative, and its `..` segments never climb above the
//!   project directory. It is the only constructor, so a value that exists already holds the rule.
//!   The project directory itself (`.`) is a valid path. The decision is lexical, on the declared
//!   string; the file system is never read, so a symlink is not seen here.
//!
//! # Non-responsibilities
//!
//! A path that may never use `..` is [`SubPath`](crate::types::SubPath). Resolving symlinks and
//! checking that the directory exists belong to the code that reads it.

use std::fmt;
use std::str::FromStr;

use crate::types::project::InvalidLocalPath;
use crate::types::report::FieldError;

/// A relative path that stays inside the project, as the author wrote it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalPath(String);

impl LocalPath {
    /// Parses `text` as a path inside the project directory.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLocalPath::Empty`] for an empty or whitespace-only string,
    /// [`InvalidLocalPath::AbsolutePath`] for a leading `/`, and
    /// [`InvalidLocalPath::OutsideProject`] when the `..` segments climb above the project
    /// directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{InvalidLocalPath, LocalPath};
    ///
    /// // A `..` may come back down, and `.` names the project directory itself.
    /// assert_eq!(LocalPath::new("packs/../tools/lint")?.as_str(), "packs/../tools/lint");
    /// assert!(LocalPath::new(".").is_ok());
    ///
    /// assert_eq!(LocalPath::new("../outside"), Err(InvalidLocalPath::OutsideProject));
    /// assert_eq!(LocalPath::new("/etc"), Err(InvalidLocalPath::AbsolutePath));
    /// # Ok::<(), InvalidLocalPath>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidLocalPath> {
        FieldError::check_not_empty(text).map_err(|_| InvalidLocalPath::Empty)?;
        if text.starts_with('/') {
            return Err(InvalidLocalPath::AbsolutePath);
        }
        let mut depth = 0_usize;
        for segment in text.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or(InvalidLocalPath::OutsideProject)?;
                }
                _ => depth += 1,
            }
        }
        Ok(Self(text.to_owned()))
    }

    /// The path as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for LocalPath {
    type Err = InvalidLocalPath;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for LocalPath {
    type Error = InvalidLocalPath;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn shown(result: Result<LocalPath, InvalidLocalPath>) -> Option<String> {
        result.err().map(|error| error.to_string())
    }

    #[test]
    fn local_path_accepts_paths_that_stay_inside() -> TestResult {
        for text in ["packs/team-lint", "a/../b", "./a", "a//b"] {
            let path = LocalPath::new(text)?;
            assert_eq!(path.as_str(), text);
            assert_eq!(path.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn local_path_accepts_the_project_root() -> TestResult {
        for text in [".", "./", "packs/..", "a/b/../.."] {
            assert_eq!(LocalPath::new(text)?.as_str(), text, "{text}");
        }
        Ok(())
    }

    #[test]
    fn local_path_refuses_absolute_and_climbing_paths() {
        assert_eq!(
            shown(LocalPath::new("/abs")).as_deref(),
            Some("absolute paths are not portable")
        );
        for text in ["../x", "a/../../x", ".."] {
            assert_eq!(
                shown(LocalPath::new(text)).as_deref(),
                Some("must stay inside the project"),
                "{text}"
            );
        }
        assert_eq!(shown(LocalPath::new("")).as_deref(), Some("empty"));
        assert_eq!(shown(LocalPath::new("  ")).as_deref(), Some("empty"));
    }

    #[test]
    fn local_path_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<LocalPath, InvalidLocalPath>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["packs/x", "/etc"] {
            let expected = shown(LocalPath::new(text));
            assert_eq!(shown(text.parse::<LocalPath>()), expected);
            assert_eq!(shown(LocalPath::try_from(text)), expected);
        }
        assert!(LocalPath::new("packs/x").is_ok());
        assert!(LocalPath::new("/etc").is_err());
    }
}
