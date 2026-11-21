//! One located refusal: where it happened and what was wrong.
//!
//! A diagnostic is its own type so every parser and check reports a refusal the same way, and so
//! the message a test compares against is the diagnostic's own `Display`.
//!
//! # Responsibilities
//!
//! - [`Diagnostic`] pairs a [`Location`] with a [`FormatError`].
//! - Its `Display` is the exact message: the location, `": "`, then the error, for example
//!   `packs/core/pack.toml: hooks[2].grant: unknown key`.
//!
//! # Non-responsibilities
//!
//! Collecting and ordering diagnostics belongs to [`Diagnostics`](crate::types::Diagnostics).

use std::fmt;

use crate::types::report::FormatError;
use crate::types::report::location::Location;

/// A refusal at a [`Location`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Diagnostic, FieldError, Location};
///
/// let diagnostic = Diagnostic::new(Location::at_key("pack.toml", "name"), FieldError::Required);
/// assert_eq!(diagnostic.to_string(), "pack.toml: name: required");
/// ```
#[derive(Debug)]
pub struct Diagnostic {
    location: Location,
    error: FormatError,
}

impl Diagnostic {
    /// A diagnostic for `error` at `location`; `error` is any type that converts into a
    /// [`FormatError`].
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{Diagnostic, FieldError, Location};
    ///
    /// let diagnostic = Diagnostic::new(Location::file("pack.toml"), FieldError::UnknownKey);
    /// assert_eq!(diagnostic.location().file_path(), std::path::Path::new("pack.toml"));
    /// assert_eq!(diagnostic.error().to_string(), "unknown key");
    /// ```
    #[must_use]
    pub fn new(location: Location, error: impl Into<FormatError>) -> Self {
        Self {
            location,
            error: error.into(),
        }
    }

    /// Where the refusal happened.
    #[must_use]
    pub const fn location(&self) -> &Location {
        &self.location
    }

    /// What was wrong.
    #[must_use]
    pub const fn error(&self) -> &FormatError {
        &self.error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.error)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::{FieldError, TypeWord};

    #[test]
    fn required_at_key_path() {
        let diagnostic = Diagnostic::new(
            Location::at_key("agentstacks.lock", "pack[1].rev"),
            FieldError::Required,
        );
        assert_eq!(
            diagnostic.to_string(),
            "agentstacks.lock: pack[1].rev: required"
        );
    }

    #[test]
    fn required_api_in_empty_project_file() {
        let diagnostic = Diagnostic::new(
            Location::at_key("agentstacks.toml", "api"),
            FieldError::Required,
        );
        assert_eq!(diagnostic.to_string(), "agentstacks.toml: api: required");
    }

    #[test]
    fn empty_at_key_path() {
        let diagnostic = Diagnostic::new(
            Location::at_key("agentstacks.lock", "merge[0].key"),
            FieldError::Empty,
        );
        assert_eq!(
            diagnostic.to_string(),
            "agentstacks.lock: merge[0].key: empty"
        );
    }

    #[test]
    fn unknown_key_in_pack_manifest() {
        let diagnostic = Diagnostic::new(
            Location::at_key("packs/core/pack.toml", "hooks[2].grant"),
            FieldError::UnknownKey,
        );
        assert_eq!(
            diagnostic.to_string(),
            "packs/core/pack.toml: hooks[2].grant: unknown key"
        );
    }

    #[test]
    fn unknown_key_in_extension_manifest() {
        let diagnostic = Diagnostic::new(
            Location::at_key("extension.toml", "extension.foo"),
            FieldError::UnknownKey,
        );
        assert_eq!(
            diagnostic.to_string(),
            "extension.toml: extension.foo: unknown key"
        );
    }

    #[test]
    fn wrong_type_at_key_path() {
        let diagnostic = Diagnostic::new(
            Location::at_key("agentstacks.toml", "api"),
            FieldError::WrongType {
                expected: TypeWord::Integer,
                got: TypeWord::Datetime,
            },
        );
        assert_eq!(
            diagnostic.to_string(),
            "agentstacks.toml: api: expected integer, got datetime"
        );
    }

    #[test]
    fn line_location_joins_with_colon_space() -> Result<(), Box<dyn std::error::Error>> {
        let line = NonZeroUsize::new(68).ok_or("test line must be non-zero")?;
        let diagnostic = Diagnostic::new(
            Location::at_line("packs/sdlc/skills/plan/SKILL.md", line),
            FieldError::Empty,
        );
        assert_eq!(
            diagnostic.to_string(),
            "packs/sdlc/skills/plan/SKILL.md:68: empty"
        );
        Ok(())
    }

    #[test]
    fn io_error_at_file() {
        let io = Error::new(ErrorKind::PermissionDenied, "denied");
        let diagnostic = Diagnostic::new(Location::file("packs/core/pack.toml"), io);
        assert_eq!(diagnostic.to_string(), "packs/core/pack.toml: denied");
    }

    #[test]
    fn accessors_return_parts() {
        let diagnostic = Diagnostic::new(
            Location::at_key("agentstacks.toml", "api"),
            FieldError::Required,
        );
        assert_eq!(diagnostic.location().to_string(), "agentstacks.toml: api");
        assert_eq!(diagnostic.error().to_string(), "required");
    }
}
