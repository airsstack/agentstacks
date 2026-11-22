//! What a plan tells the author without refusing anything.
//!
//! # Responsibilities
//!
//! - [`Note`] is one thing the plan reports: an ignored dot entry, a `native/` directory no tool
//!   claims, or a source that changed and was resolved again. Its `Display` is the text shown.
//! - [`AirslPin`] is the airsl version the lock pins now and the one installed, for the plan to
//!   show the change.
//!
//! # Non-responsibilities
//!
//! Deciding that a source is stale, and rendering an [`AirslPin`], belong to other code.

use std::fmt;
use std::path::PathBuf;

use crate::types::manifest::PackVersion;
use crate::types::name::ToolName;
use crate::types::report::Location;

/// One note of a plan.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// use agentstacks::types::plan::Note;
///
/// let note = Note::Ignored {
///     root: PathBuf::from("packs/core"),
///     path: PathBuf::from(".DS_Store"),
/// };
/// assert_eq!(note.to_string(), "packs/core/.DS_Store: ignored (dot entry)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A dot entry that was left out of a pack or an extension.
    Ignored {
        /// The directory that was loaded.
        root: PathBuf,
        /// The entry, relative to `root`.
        path: PathBuf,
    },
    /// A `native/<dir>` directory that no tool of the project claims.
    UnusedNative {
        /// The directory name under `native/`.
        dir: ToolName,
        /// The tool being planned.
        tool: ToolName,
    },
    /// A source that changed and was resolved again.
    SourceChanged {
        /// The entry of `agentstacks.toml` whose source changed.
        location: Location,
    },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ignored { root, path } => {
                write!(
                    f,
                    "{}: ignored (dot entry)",
                    Location::file(root.join(path))
                )
            }
            Self::UnusedNative { dir, tool } => write!(f, "native/{dir}: not for {tool}"),
            Self::SourceChanged { location } => {
                write!(f, "{location}: source changed, resolved again")
            }
        }
    }
}

/// The airsl version the lock pins and the one installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirslPin {
    /// The version the lock pinned, when it pinned one.
    pub locked: Option<PackVersion>,
    /// The installed version, which the new lock pins.
    pub installed: PackVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn note_texts_match_the_spec() -> TestResult {
        let ignored = Note::Ignored {
            root: PathBuf::from("packs/core"),
            path: PathBuf::from(".DS_Store"),
        };
        assert_eq!(
            ignored.to_string(),
            "packs/core/.DS_Store: ignored (dot entry)"
        );
        let native = Note::UnusedNative {
            dir: ToolName::new("cluade")?,
            tool: ToolName::new("claude")?,
        };
        assert_eq!(native.to_string(), "native/cluade: not for claude");
        let changed = Note::SourceChanged {
            location: Location::at_key("agentstacks.toml", "packs.core"),
        };
        assert_eq!(
            changed.to_string(),
            "agentstacks.toml: packs.core: source changed, resolved again"
        );
        Ok(())
    }
}
