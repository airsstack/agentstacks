//! The one error type a located diagnostic carries.
//!
//! Every module's error enum joins here through a single `#[from]` variant, so a diagnostic holds
//! one type whatever produced it.
//!
//! # Responsibilities
//!
//! - [`FormatError`] has a variant per module error enum, each displaying its inner error
//!   unchanged.
//! - [`FormatError::Io`] is the only I/O variant. An I/O failure is located at the path that
//!   failed ([`Location::file`](crate::types::Location::file)), so it reads `<path>: <the std::io::Error
//!   Display>`.
//!
//! # Usage
//!
//! `std::io::Error` does not implement `PartialEq`, so neither does [`FormatError`]; compare
//! displayed text rather than error values.
//!
//! # Non-responsibilities
//!
//! The messages themselves belong to the module error enums this one wraps.

use crate::types::extension::ExtensionError;
use crate::types::frontmatter::FrontmatterError;
use crate::types::lock::LockError;
use crate::types::manifest::ManifestError;
use crate::types::pack::PackError;
use crate::types::project::ProjectError;
use crate::types::project::SourceError;
use crate::types::report::FieldError;
use crate::types::set::SetError;
use crate::types::tokens::TokenError;
use crate::types::tree::WalkError;

/// An error a located diagnostic carries.
///
/// # Examples
///
/// Any module error converts in, and the message is the inner error's own:
///
/// ```
/// use agentstacks::types::{FieldError, FormatError};
///
/// let error = FormatError::from(FieldError::Required);
/// assert_eq!(error.to_string(), "required");
/// ```
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// One of the generic field message shapes.
    #[error(transparent)]
    Field(#[from] FieldError),
    /// An extension message with its own wording.
    #[error(transparent)]
    Extension(#[from] ExtensionError),
    /// A frontmatter message with its own wording.
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
    /// A lockfile message with its own wording.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// A manifest message with its own wording.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// A pack directory message with its own wording.
    #[error(transparent)]
    Pack(#[from] PackError),
    /// A project file message with its own wording.
    #[error(transparent)]
    Project(#[from] ProjectError),
    /// A source message with its own wording.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// A whole-set message with its own wording. Boxed: its variants are larger than every other
    /// error here, and every `Result` that returns a diagnostic would grow with it.
    #[error(transparent)]
    Set(Box<SetError>),
    /// A TOML syntax error, in the parser's own words.
    #[error("{0}")]
    TomlSyntax(String),
    /// A token the scanner refused.
    #[error(transparent)]
    Token(#[from] TokenError),
    /// An I/O failure on the file the diagnostic is located at.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A directory entry the walker refuses.
    #[error(transparent)]
    Walk(#[from] WalkError),
}

impl From<SetError> for FormatError {
    fn from(error: SetError) -> Self {
        Self::Set(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn field_error_converts_and_displays_unchanged() {
        let error = FormatError::from(FieldError::UnknownKey);
        assert_eq!(error.to_string(), "unknown key");
    }

    #[test]
    fn format_error_walk_displays_the_walk_message() {
        for walk in [
            WalkError::Symlink,
            WalkError::SpecialFile,
            WalkError::NotUtf8,
        ] {
            assert_eq!(FormatError::from(walk).to_string(), walk.to_string());
        }
    }

    #[test]
    fn source_and_project_errors_convert_and_display_unchanged() {
        assert_eq!(
            FormatError::from(SourceError::GitOrPath).to_string(),
            "needs exactly one of git, path"
        );
        assert_eq!(
            FormatError::from(ProjectError::NothingToInstall).to_string(),
            "nothing to install"
        );
    }

    #[test]
    fn extension_error_converts_and_displays_unchanged() {
        assert_eq!(
            FormatError::from(ExtensionError::NotAnExtension).to_string(),
            "not an extension (no extension.toml)"
        );
    }

    #[test]
    fn lock_error_converts_and_displays_unchanged() {
        assert_eq!(
            FormatError::from(LockError::AirslMissing).to_string(),
            "[airsl] missing, but locked packs require airsl"
        );
    }

    #[test]
    fn pack_error_converts_and_displays_unchanged() {
        assert_eq!(
            FormatError::from(PackError::NotAPack).to_string(),
            "not a pack (no pack.toml)"
        );
    }

    #[test]
    fn set_error_converts_and_displays_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let error = SetError::SourcePack {
            name: crate::types::name::PackName::new("kore")?,
        };
        assert_eq!(
            FormatError::from(error).to_string(),
            "source contains pack \"kore\""
        );
        Ok(())
    }

    #[test]
    fn token_error_converts_and_displays_unchanged() {
        let error = FormatError::from(TokenError::Unterminated);
        assert_eq!(error.to_string(), "unterminated token");
    }

    #[test]
    fn io_error_converts_and_displays_its_own_message() {
        let io = Error::new(ErrorKind::PermissionDenied, "denied");
        let expected = io.to_string();
        let error = FormatError::from(io);
        assert_eq!(error.to_string(), expected);
        assert_eq!(error.to_string(), "denied");
    }
}
