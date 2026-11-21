//! Where a refusal happened: a file, plus an optional line or key path.
//!
//! A refusal is only useful when the author can find what it is about, so every diagnostic
//! carries a [`Location`]. It is its own type so the location forms below are built in one place
//! and no caller formats a position by hand.
//!
//! # Responsibilities
//!
//! - Holds a file and at most one of a 1-based line or a dotted key path, so a location never
//!   holds both.
//! - Displays the file `/`-separated on every platform, then the line or key path:
//!
//!   | Location | Displays as |
//!   |---|---|
//!   | line | `<file>:<line>` |
//!   | key path | `<file>: <key.path>` |
//!   | file only | `<file>` |
//!   | set level, with key | `<project file>: <key.path>` |
//!   | set level, no key | `<project file>` |
//!
//! # Non-responsibilities
//!
//! Finding the line or key path belongs to the parsers; this type only carries and displays them.

use std::fmt;
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};

/// A file plus an optional line or key path.
///
/// Fields are private; the constructors guarantee a location never holds both a line and a key
/// path.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::Location;
///
/// let line = NonZeroUsize::new(3).ok_or("line numbers start at 1")?;
/// assert_eq!(Location::at_line("packs/core/pack.toml", line).to_string(), "packs/core/pack.toml:3");
/// assert_eq!(
///     Location::at_key("packs/core/pack.toml", "hooks[2].grant").to_string(),
///     "packs/core/pack.toml: hooks[2].grant",
/// );
/// assert_eq!(Location::file("packs/core/pack.toml").to_string(), "packs/core/pack.toml");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    file: PathBuf,
    line: Option<NonZeroUsize>,
    key_path: Option<String>,
}

impl Location {
    /// A location naming only a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Location;
    ///
    /// let location = Location::file("agentstacks.toml");
    /// assert_eq!(location.line(), None);
    /// assert_eq!(location.key_path(), None);
    /// ```
    #[must_use]
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            file: path.into(),
            line: None,
            key_path: None,
        }
    }

    /// A location at a 1-based line of a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::Location;
    ///
    /// let line = NonZeroUsize::new(7).ok_or("line numbers start at 1")?;
    /// let location = Location::at_line("SKILL.md", line);
    /// assert_eq!(location.line(), Some(line));
    /// assert_eq!(location.to_string(), "SKILL.md:7");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn at_line(path: impl Into<PathBuf>, line: NonZeroUsize) -> Self {
        Self {
            file: path.into(),
            line: Some(line),
            key_path: None,
        }
    }

    /// A location at a dotted key path inside a file, such as `hooks[2].grants.read[0]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Location;
    ///
    /// let location = Location::at_key("pack.toml", "hooks[2].grants.read[0]");
    /// assert_eq!(location.key_path(), Some("hooks[2].grants.read[0]"));
    /// assert_eq!(location.to_string(), "pack.toml: hooks[2].grants.read[0]");
    /// ```
    #[must_use]
    pub fn at_key(path: impl Into<PathBuf>, key_path: impl Into<String>) -> Self {
        Self {
            file: path.into(),
            line: None,
            key_path: Some(key_path.into()),
        }
    }

    /// The location of a refusal about a whole set of packs: the project file at `project_file`,
    /// exactly as the caller names it, with the key path of the entry it concerns when there is
    /// one.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Location;
    ///
    /// let with_key = Location::set_level("agentstacks.toml", Some("packs[1]".to_owned()));
    /// assert_eq!(with_key.to_string(), "agentstacks.toml: packs[1]");
    /// assert_eq!(Location::set_level("agentstacks.toml", None).to_string(), "agentstacks.toml");
    /// ```
    #[must_use]
    pub fn set_level(project_file: impl Into<PathBuf>, key_path: Option<String>) -> Self {
        Self {
            file: project_file.into(),
            line: None,
            key_path,
        }
    }

    /// The file this location names.
    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.file
    }

    /// The 1-based line, when the location has one.
    #[must_use]
    pub const fn line(&self) -> Option<NonZeroUsize> {
        self.line
    }

    /// The dotted key path, when the location has one.
    #[must_use]
    pub fn key_path(&self) -> Option<&str> {
        self.key_path.as_deref()
    }

    /// The file as a `/`-separated string: the path's components joined with `/`, each written
    /// as [`Path::display`] writes it.
    pub(crate) fn file_text(&self) -> String {
        let mut text = String::new();
        for component in self.file.components() {
            if matches!(component, Component::RootDir) {
                text.push('/');
                continue;
            }
            if !text.is_empty() && !text.ends_with('/') {
                text.push('/');
            }
            text.push_str(&component.as_os_str().display().to_string());
        }
        text
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.file_text())?;
        match (self.line, &self.key_path) {
            (Some(line), _) => write!(f, ":{line}"),
            (None, Some(key_path)) => write!(f, ": {key_path}"),
            (None, None) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn line(n: usize) -> Result<NonZeroUsize, Box<dyn std::error::Error>> {
        Ok(NonZeroUsize::new(n).ok_or("test line must be non-zero")?)
    }

    #[test]
    fn at_line_displays_file_colon_line() -> TestResult {
        let location = Location::at_line("packs/sdlc/skills/plan/SKILL.md", line(68)?);
        assert_eq!(location.to_string(), "packs/sdlc/skills/plan/SKILL.md:68");
        Ok(())
    }

    #[test]
    fn at_key_displays_file_then_key_path() {
        let location = Location::at_key("packs/core/pack.toml", "hooks[2].grant");
        assert_eq!(location.to_string(), "packs/core/pack.toml: hooks[2].grant");
    }

    #[test]
    fn file_only_displays_the_path() {
        let location = Location::file("agentstacks.lock");
        assert_eq!(location.to_string(), "agentstacks.lock");
    }

    #[test]
    fn file_displays_slash_separated() {
        let path = Path::new("packs").join("core").join("pack.toml");
        let location = Location::file(path);
        assert_eq!(location.to_string(), "packs/core/pack.toml");
        assert_eq!(location.file_text(), "packs/core/pack.toml");
    }

    #[cfg(unix)]
    #[test]
    fn redundant_separators_and_current_dir_components_are_normalised() {
        for raw in ["a//b/pack.toml", "a/./b/pack.toml", "a/b//pack.toml"] {
            let location = Location::file(raw);
            assert_eq!(location.file_text(), "a/b/pack.toml", "{raw}");
            assert_eq!(location.to_string(), "a/b/pack.toml", "{raw}");
        }
    }

    #[test]
    fn absolute_file_has_a_single_leading_slash() {
        let location = Location::file(Path::new("/").join("tmp").join("x").join("pack.toml"));
        assert_eq!(location.file_text(), "/tmp/x/pack.toml");
    }

    #[test]
    fn set_level_with_key_displays_project_file_then_key() {
        let location = Location::set_level(
            "agentstacks.toml",
            Some("packs.core.options.archive_keep".to_owned()),
        );
        assert_eq!(
            location.to_string(),
            "agentstacks.toml: packs.core.options.archive_keep"
        );
    }

    #[test]
    fn set_level_without_key_displays_project_file() {
        let location = Location::set_level("agentstacks.toml", None);
        assert_eq!(location.to_string(), "agentstacks.toml");
    }

    #[test]
    fn set_level_uses_the_callers_project_file_path() {
        let keyed =
            Location::set_level("/work/app/agentstacks.toml", Some("packs.core".to_owned()));
        assert_eq!(keyed.to_string(), "/work/app/agentstacks.toml: packs.core");
        let plain = Location::set_level("/work/app/agentstacks.toml", None);
        assert_eq!(plain.to_string(), "/work/app/agentstacks.toml");
        let parsed = Location::at_key("/work/app/agentstacks.toml", "packs.core");
        assert_eq!(keyed, parsed);
    }

    #[test]
    fn accessors_return_constructed_parts() -> TestResult {
        let lined = Location::at_line("a.md", line(3)?);
        assert_eq!(lined.file_path(), Path::new("a.md"));
        assert_eq!(lined.line(), Some(line(3)?));
        assert_eq!(lined.key_path(), None);

        let keyed = Location::at_key("a.toml", "pack.api");
        assert_eq!(keyed.file_path(), Path::new("a.toml"));
        assert_eq!(keyed.line(), None);
        assert_eq!(keyed.key_path(), Some("pack.api"));

        let plain = Location::file("a.lock");
        assert_eq!(plain.file_path(), Path::new("a.lock"));
        assert_eq!(plain.line(), None);
        assert_eq!(plain.key_path(), None);
        Ok(())
    }
}
