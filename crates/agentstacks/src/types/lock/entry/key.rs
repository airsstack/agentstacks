//! What identifies one lock entry, so two entries with the same identity are duplicates.
//!
//! It is a type of its own because the duplicate refusal names the entry by this identity alone:
//! [`ListedTwice`](crate::types::ListedTwice) shows it, and the phrase stays defined once.
//!
//! # Responsibilities
//!
//! - [`EntryKey`] has a variant per lock array (`tool`, `pack`, `file`) holding the typed values
//!   that identify an entry of it. It displays the array's word, then each value quoted:
//!   `tool "claude"`, `file ".claude/x"`.
//!
//! # Non-responsibilities
//!
//! Finding the repeated entries belongs to the lock reader.

use std::fmt;

use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::project::SubPath;

/// The identity of one lock entry, shown with the word of the array it is in.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{EntryKey, PackName};
///
/// let key = EntryKey::Pack(PackName::new("core")?);
/// assert_eq!(key.to_string(), "pack \"core\"");
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKey {
    /// A `[[tool]]` entry, identified by its name.
    Tool(ToolName),
    /// A `[[pack]]` entry, identified by its name.
    Pack(PackName),
    /// A `[[file]]` entry, identified by its path as written by the later of the entries that
    /// name it.
    File(SubPath),
}

impl fmt::Display for EntryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tool(name) => write!(f, "tool \"{name}\""),
            Self::Pack(name) => write!(f, "pack \"{name}\""),
            Self::File(path) => write!(f, "file \"{path}\""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn entry_key_shows_the_array_word_then_each_value_quoted() -> TestResult {
        let cases = [
            (EntryKey::Tool(ToolName::new("claude")?), "tool \"claude\""),
            (EntryKey::Pack(PackName::new("core")?), "pack \"core\""),
            (
                EntryKey::File(SubPath::new(".claude/x")?),
                "file \".claude/x\"",
            ),
        ];
        for (key, shown) in cases {
            assert_eq!(key.to_string(), shown);
        }
        Ok(())
    }
}
