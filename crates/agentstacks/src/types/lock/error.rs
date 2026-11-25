//! The messages of `agentstacks.lock` with a wording of their own.
//!
//! The generic shapes belong to [`FieldError`](crate::types::FieldError) and the source messages
//! to [`SourceError`](crate::types::SourceError); every other refusal the lock can meet is one
//! variant here.
//!
//! It is a file of its own because the reader, the writer and the airsl check all raise these
//! refusals, so none of them can own the wording.
//!
//! # Responsibilities
//!
//! - [`LockError`] has a variant per lock refusal, each displaying exactly the message the author
//!   reads after the location. A name refusal wraps [`InvalidName`], a version refusal wraps
//!   [`InvalidVersion`] and a hash refusal wraps [`ContentHashError`], all transparently. A file
//!   path written with a backslash, or naming `agentstacks.toml` or `agentstacks.lock`, has a
//!   variant of its own.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it.

use crate::types::lock::entry::EntryKey;
use crate::types::manifest::InvalidVersion;
use crate::types::name::InvalidName;
use crate::types::report::ListedTwice;
use crate::types::report::MustBeRelative;
use crate::types::report::OutsideProject;
use crate::types::report::UnsupportedApi;
use crate::types::tree::ContentHashError;

/// A refusal of `agentstacks.lock` as a whole or of one of its entries.
///
/// # Examples
///
/// ```
/// use agentstacks::types::LockError;
///
/// assert_eq!(
///     LockError::UnknownOwner { text: "kore".to_owned() }.to_string(),
///     "\"kore\" is not a locked pack or tool"
/// );
/// assert_eq!(
///     LockError::AirslMissing.to_string(),
///     "[airsl] missing, but locked packs require airsl"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LockError {
    /// An `api` this agentstacks does not speak.
    #[error("api {0}")]
    UnsupportedApi(#[from] UnsupportedApi),
    /// An `[airsl]` version that is not an exact version.
    #[error("\"{text}\" is not a version")]
    NotAVersion {
        /// The refused text.
        text: String,
    },
    /// An entry name that is not a valid name.
    #[error(transparent)]
    Name(#[from] InvalidName),
    /// An entry version that is not semver.
    #[error(transparent)]
    Version(#[from] InvalidVersion),
    /// A hash that is not `sha256:` and 64 lowercase hex characters.
    #[error(transparent)]
    Hash(#[from] ContentHashError),
    /// A file path that is absolute.
    #[error(transparent)]
    MustBeRelative(#[from] MustBeRelative),
    /// A file path that climbs out of the project.
    #[error(transparent)]
    OutsideProject(#[from] OutsideProject),
    /// A file path written with `\` instead of `/`.
    #[error("\"{text}\" must use / as the separator")]
    Separator {
        /// The refused path, as written.
        text: String,
    },
    /// A file path that names one of agentstacks' own files.
    #[error("\"{text}\" is managed by agentstacks")]
    Managed {
        /// The refused path, as written.
        text: String,
    },
    /// A `by` that names no locked pack or tool.
    #[error("\"{text}\" is not a locked pack or tool")]
    UnknownOwner {
        /// The refused text.
        text: String,
    },
    /// A tool, pack or file locked twice, named by its [`EntryKey`].
    #[error(transparent)]
    ListedTwice(#[from] ListedTwice<EntryKey>),
    /// Locked packs need airsl and the lock records none.
    #[error("[airsl] missing, but locked packs require airsl")]
    AirslMissing,
    /// The lock records airsl and no locked pack needs it.
    #[error("[airsl] present, but no locked pack requires airsl")]
    AirslUnneeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;
    use crate::types::name::ToolName;
    use crate::types::project::SubPath;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn lock_error_displays_each_refusal_text() -> TestResult {
        let hash_error = ContentHash::new("sha256:XYZ")
            .err()
            .ok_or("expected a refused hash")?;
        let cases = [
            (
                LockError::from(UnsupportedApi::new(2)),
                "api 2 not supported (this agentstacks speaks 1)",
            ),
            (
                LockError::NotAVersion {
                    text: "0.1".to_owned(),
                },
                "\"0.1\" is not a version",
            ),
            (
                LockError::Name(
                    PackName::new("Core")
                        .err()
                        .ok_or("expected a refused name")?,
                ),
                "\"Core\" is not a valid name",
            ),
            (
                LockError::from(InvalidVersion::new("1.2")),
                "\"1.2\" is not semver",
            ),
            (
                LockError::Hash(hash_error),
                "bad hash (expected sha256:<64 lowercase hex>)",
            ),
            (
                LockError::from(MustBeRelative::new("/x")),
                "\"/x\" must be relative",
            ),
            (
                LockError::from(OutsideProject::new("../x")),
                "\"../x\" must stay inside the project",
            ),
            (
                LockError::UnknownOwner {
                    text: "kore".to_owned(),
                },
                "\"kore\" is not a locked pack or tool",
            ),
            (
                LockError::from(ListedTwice::new(EntryKey::Tool(ToolName::new("claude")?))),
                "tool \"claude\" listed twice",
            ),
            (
                LockError::from(ListedTwice::new(EntryKey::Pack(PackName::new("core")?))),
                "pack \"core\" listed twice",
            ),
            (
                LockError::from(ListedTwice::new(EntryKey::File(SubPath::new(".claude/x")?))),
                "file \".claude/x\" listed twice",
            ),
            (
                LockError::Separator {
                    text: "a\\b".to_owned(),
                },
                "\"a\\b\" must use / as the separator",
            ),
            (
                LockError::Managed {
                    text: "agentstacks.lock".to_owned(),
                },
                "\"agentstacks.lock\" is managed by agentstacks",
            ),
            (
                LockError::AirslMissing,
                "[airsl] missing, but locked packs require airsl",
            ),
            (
                LockError::AirslUnneeded,
                "[airsl] present, but no locked pack requires airsl",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
        Ok(())
    }
}
