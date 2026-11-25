//! Where a refusal happened: a file, plus an optional line or key path.
//!
//! A refusal is only useful when the author can find what it is about, so every diagnostic
//! carries a [`Location`]. It is its own type so the location forms below are built in one place
//! and no caller formats a position by hand.
//!
//! # Responsibilities
//!
//! - Holds a path or verbatim text, and at most one of a 1-based line or a dotted key path, so a
//!   location never holds both.
//! - Displays a path `/`-separated on every platform, then the line or key path. A
//!   [verbatim](Location::verbatim) location is shown exactly as given, for text such as
//!   `ext add "https://example.com/x.git"` that is not a path:
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
    place: Place,
    at: Position,
}

/// What a location names: a path, shown `/`-separated, or text shown as given.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    Path(PathBuf),
    Text(String),
}

/// Where in the file: nowhere, a line or a key path, never two at once. One field keeps a
/// [`Location`] small enough that a [`Diagnostic`](crate::types::Diagnostic) stays cheap to return.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Position {
    File,
    Line(NonZeroUsize),
    Key(String),
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
            place: Place::Path(path.into()),
            at: Position::File,
        }
    }

    /// A location that names text which is not a path, shown exactly as given: `//` and `/./` stay.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Location;
    ///
    /// let text = "ext add \"https://example.com/x.git\"";
    /// assert_eq!(Location::verbatim(text).to_string(), text);
    /// assert_eq!(Location::file(text).to_string(), "ext add \"https:/example.com/x.git\"");
    /// ```
    #[must_use]
    pub fn verbatim(text: impl Into<String>) -> Self {
        Self {
            place: Place::Text(text.into()),
            at: Position::File,
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
            place: Place::Path(path.into()),
            at: Position::Line(line),
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
            place: Place::Path(path.into()),
            at: Position::Key(key_path.into()),
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
            place: Place::Path(project_file.into()),
            at: key_path.map_or(Position::File, Position::Key),
        }
    }

    /// The file this location names, or `None` for a [verbatim](Self::verbatim) location, which
    /// names text and not a path.
    #[must_use]
    pub fn file_path(&self) -> Option<&Path> {
        match &self.place {
            Place::Path(path) => Some(path),
            Place::Text(_) => None,
        }
    }

    /// The 1-based line, when the location has one.
    #[must_use]
    pub const fn line(&self) -> Option<NonZeroUsize> {
        match self.at {
            Position::Line(line) => Some(line),
            Position::File | Position::Key(_) => None,
        }
    }

    /// The dotted key path, when the location has one.
    #[must_use]
    pub fn key_path(&self) -> Option<&str> {
        match &self.at {
            Position::Key(key_path) => Some(key_path),
            Position::File | Position::Line(_) => None,
        }
    }

    /// The file as a `/`-separated string: the path's components joined with `/`, each written
    /// as [`Path::display`] writes it.
    pub(crate) fn file_text(&self) -> String {
        let path = match &self.place {
            Place::Path(path) => path,
            Place::Text(text) => return text.clone(),
        };
        let mut text = String::new();
        for component in path.components() {
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
        match &self.at {
            Position::Line(line) => write!(f, ":{line}"),
            Position::Key(key_path) => write!(f, ": {key_path}"),
            Position::File => Ok(()),
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
    fn verbatim_keeps_what_a_path_would_normalise() {
        for text in [
            "ext add \"https://example.com/x.git\"",
            "ext add \"./a//b/\"",
        ] {
            let location = Location::verbatim(text);
            assert_eq!(location.to_string(), text);
            assert_eq!(location.file_path(), None);
            assert_eq!(location.line(), None);
            assert_eq!(location.key_path(), None);
        }
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
        assert_eq!(lined.file_path(), Some(Path::new("a.md")));
        assert_eq!(lined.line(), Some(line(3)?));
        assert_eq!(lined.key_path(), None);

        let keyed = Location::at_key("a.toml", "pack.api");
        assert_eq!(keyed.file_path(), Some(Path::new("a.toml")));
        assert_eq!(keyed.line(), None);
        assert_eq!(keyed.key_path(), Some("pack.api"));

        let plain = Location::file("a.lock");
        assert_eq!(plain.file_path(), Some(Path::new("a.lock")));
        assert_eq!(plain.line(), None);
        assert_eq!(plain.key_path(), None);
        Ok(())
    }
}
