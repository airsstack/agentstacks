//! The messages of a tool extension's directory and manifest that have a wording of their own.
//!
//! The generic shapes (`required`, `expected <type>, got <type>`, `empty`, `unknown key`) belong to
//! [`FieldError`](crate::types::FieldError); every other refusal an extension can meet is one
//! variant here, so a caller can match on it and the wording lives in one place.
//!
//! It is a file of its own because the entry path, the manifest and the loader all raise these
//! refusals, so none of them can own the wording.
//!
//! # Responsibilities
//!
//! - [`ExtensionError`] has a variant per extension refusal, each displaying exactly the message
//!   the author reads after the location.
//! - It wraps the name and version errors transparently, so a bad extension name or version reads
//!   the same as a bad one anywhere else.
//! - It words the symlink refusal for an extension, which the directory walker words for a pack.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it, and which field raises which refusal belongs to the types that read the manifest. An I/O
//! failure and the walker's other refusals are not variants here:
//! [`FormatError`](crate::types::FormatError) carries them.

use crate::types::manifest::InvalidVersion;
use crate::types::name::InvalidName;
use crate::types::report::DoesNotExist;
use crate::types::report::NotLuaFile;
use crate::types::report::UnsupportedApi;

/// A refusal of an extension directory or its `extension.toml`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ExtensionError;
///
/// assert_eq!(
///     ExtensionError::NotAnExtension.to_string(),
///     "not an extension (no extension.toml)"
/// );
/// assert_eq!(
///     ExtensionError::EntryOutside { text: "../adapter.lua".to_owned() }.to_string(),
///     "\"../adapter.lua\" must stay inside the extension"
/// );
/// assert_eq!(
///     ExtensionError::Symlink.to_string(),
///     "symlinks are not allowed in an extension"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExtensionError {
    /// The directory holds no `extension.toml`.
    #[error("not an extension (no extension.toml)")]
    NotAnExtension,
    /// A name that is not a valid extension name.
    #[error(transparent)]
    Name(#[from] InvalidName),
    /// A `version` that is not semver.
    #[error(transparent)]
    Version(#[from] InvalidVersion),
    /// An `api` this agentstacks does not speak.
    #[error(transparent)]
    UnsupportedApi(#[from] UnsupportedApi),
    /// An `entry` that is absolute or climbs out of the extension.
    #[error("\"{text}\" must stay inside the extension")]
    EntryOutside {
        /// The refused path, as written.
        text: String,
    },
    /// An `entry` that is not a `.lua` file.
    #[error(transparent)]
    NotLuaFile(#[from] NotLuaFile),
    /// An `entry` that names no file of the extension.
    #[error(transparent)]
    DoesNotExist(#[from] DoesNotExist),
    /// A symlink inside the extension directory.
    #[error("symlinks are not allowed in an extension")]
    Symlink,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_error_displays_each_message() {
        let text = || "adapter.lua".to_owned();
        let cases = [
            (
                ExtensionError::NotAnExtension,
                "not an extension (no extension.toml)",
            ),
            (
                ExtensionError::from(InvalidName::new("Claude")),
                "\"Claude\" is not a valid name",
            ),
            (
                ExtensionError::from(InvalidVersion::new("1.2")),
                "\"1.2\" is not semver",
            ),
            (
                ExtensionError::from(UnsupportedApi::new(1)),
                "1 not supported (this agentstacks speaks 0)",
            ),
            (
                ExtensionError::EntryOutside { text: text() },
                "\"adapter.lua\" must stay inside the extension",
            ),
            (
                ExtensionError::from(NotLuaFile::new(text())),
                "\"adapter.lua\" is not a .lua file",
            ),
            (
                ExtensionError::from(DoesNotExist::new(text())),
                "\"adapter.lua\" does not exist",
            ),
            (
                ExtensionError::Symlink,
                "symlinks are not allowed in an extension",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
