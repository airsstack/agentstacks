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
//!   [`InvalidVersion`] and a hash refusal wraps [`ContentHashError`], all transparently.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it.

use crate::types::api::API;
use crate::types::lock::merge::{MergeFile, MergeKey};
use crate::types::manifest::InvalidVersion;
use crate::types::name::InvalidName;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::project::SubPath;
use crate::types::tree::ContentHashError;

/// A refusal of `agentstacks.lock` as a whole or of one of its entries.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{LockError, PackName};
///
/// assert_eq!(
///     LockError::UnknownOwner { name: "kore".to_owned() }.to_string(),
///     "\"kore\" is not a locked pack or tool"
/// );
/// assert_eq!(
///     LockError::DuplicatePack { name: PackName::new("core")? }.to_string(),
///     "pack \"core\" listed twice"
/// );
/// assert_eq!(
///     LockError::AirslMissing.to_string(),
///     "[airsl] missing, but locked packs require airsl"
/// );
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LockError {
    /// An `api` this agentstacks does not speak.
    #[error("api {api} not supported (this agentstacks speaks {})", API)]
    ApiUnsupported {
        /// The api the file asks for.
        api: i64,
    },
    /// An `[airsl]` version that is not an exact version.
    #[error("\"{value}\" is not a version")]
    NotAVersion {
        /// The refused text.
        value: String,
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
    #[error("\"{value}\" must be relative")]
    NotRelative {
        /// The refused text.
        value: String,
    },
    /// A file path that climbs out of the project.
    #[error("\"{value}\" must stay inside the project")]
    OutsideProject {
        /// The refused text.
        value: String,
    },
    /// A `by` that names no locked pack or tool.
    #[error("\"{name}\" is not a locked pack or tool")]
    UnknownOwner {
        /// The refused text.
        name: String,
    },
    /// A tool locked twice.
    #[error("tool \"{name}\" listed twice")]
    DuplicateTool {
        /// The repeated name.
        name: ToolName,
    },
    /// A pack locked twice.
    #[error("pack \"{name}\" listed twice")]
    DuplicatePack {
        /// The repeated name.
        name: PackName,
    },
    /// A file recorded twice.
    #[error("file \"{path}\" listed twice")]
    DuplicateFile {
        /// The repeated path, as written by the later of the entries that name it.
        path: SubPath,
    },
    /// A merge recorded twice for one file and key.
    #[error("merge \"{file}\" \"{key}\" listed twice")]
    DuplicateMerge {
        /// The file the merge touched.
        file: MergeFile,
        /// The key the merge touched.
        key: MergeKey,
    },
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
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn lock_error_displays_each_refusal_text() -> TestResult {
        let hash_error = ContentHash::parse("sha256:XYZ")
            .err()
            .ok_or("expected a refused hash")?;
        let cases = [
            (
                LockError::ApiUnsupported { api: 1 },
                "api 1 not supported (this agentstacks speaks 0)",
            ),
            (
                LockError::NotAVersion {
                    value: "0.1".to_owned(),
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
                LockError::NotRelative {
                    value: "/x".to_owned(),
                },
                "\"/x\" must be relative",
            ),
            (
                LockError::OutsideProject {
                    value: "../x".to_owned(),
                },
                "\"../x\" must stay inside the project",
            ),
            (
                LockError::UnknownOwner {
                    name: "kore".to_owned(),
                },
                "\"kore\" is not a locked pack or tool",
            ),
            (
                LockError::DuplicateTool {
                    name: ToolName::new("claude")?,
                },
                "tool \"claude\" listed twice",
            ),
            (
                LockError::DuplicatePack {
                    name: PackName::new("core")?,
                },
                "pack \"core\" listed twice",
            ),
            (
                LockError::DuplicateFile {
                    path: SubPath::new(".claude/x")?,
                },
                "file \".claude/x\" listed twice",
            ),
            (
                LockError::DuplicateMerge {
                    file: MergeFile::new(".claude/settings.json")?,
                    key: MergeKey::new("hooks")?,
                },
                "merge \".claude/settings.json\" \"hooks\" listed twice",
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
