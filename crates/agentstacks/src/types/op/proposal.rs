//! The operations a tool extension proposes, and a proposal as the core records it.
//!
//! There is no merge and no run-command operation: a file shared with the user is read and
//! rewritten whole by the extension, and nothing a third party writes runs.
//!
//! # Responsibilities
//!
//! - [`Operation`] is a write, a copy of a payload file, a skip with a reason, a required
//!   binary, or a delete. An extension proposes the first four; the core adds a delete for a
//!   file of a removed pack.
//! - It gives the path an operation names and the ids it accounts for.
//! - [`Proposal`] pairs an operation with its [`Origin`].
//!
//! # Non-responsibilities
//!
//! Whether a proposal is accepted belongs to the proposal record, and who owns the file it
//! writes belongs to the owner rule.

use crate::types::name::BinaryName;
use crate::types::op::{ItemId, OpError, OpPath, Origin, SkipReason};

/// One operation.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::{Operation, OpPath};
///
/// let write = Operation::Write {
///     path: OpPath::new(".claude/x")?,
///     content: b"x".to_vec(),
///     covers: Vec::new(),
/// };
/// assert_eq!(write.path().map(OpPath::as_str), Some(".claude/x"));
/// assert!(write.covered().is_empty());
/// # Ok::<(), agentstacks::types::op::PathShape>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Writes `content` to `path`. An empty `covers` accounts for no item.
    Write {
        /// The file written.
        path: OpPath,
        /// Its bytes.
        content: Vec<u8>,
        /// The items the write accounts for.
        covers: Vec<ItemId>,
    },
    /// Copies the payload file `file` to `path`.
    Copy {
        /// The file written.
        path: OpPath,
        /// The payload file copied.
        file: ItemId,
    },
    /// Skips `item`, which the tool cannot express.
    Skip {
        /// The item skipped.
        item: ItemId,
        /// Why.
        reason: SkipReason,
    },
    /// Requires `name` on `PATH`.
    RequireBinary {
        /// The binary.
        name: BinaryName,
    },
    /// Deletes `path`, which a removed pack once wrote. Never proposed by an extension.
    Delete {
        /// The file deleted.
        path: OpPath,
    },
}

impl Operation {
    /// A `require_binary` of `text`.
    ///
    /// # Errors
    ///
    /// [`OpError::InvalidBinaryName`] when `text` is not a binary name.
    pub fn require_binary(text: &str) -> Result<Self, OpError> {
        BinaryName::new(text)
            .map(|name| Self::RequireBinary { name })
            .map_err(OpError::InvalidBinaryName)
    }

    /// The path a write, a copy or a delete names.
    #[must_use]
    pub const fn path(&self) -> Option<&OpPath> {
        match self {
            Self::Write { path, .. } | Self::Copy { path, .. } | Self::Delete { path } => {
                Some(path)
            }
            Self::Skip { .. } | Self::RequireBinary { .. } => None,
        }
    }

    /// The ids the operation accounts for: a copy's file, a write's `covers`, a skip's item.
    #[must_use]
    pub fn covered(&self) -> &[ItemId] {
        match self {
            Self::Write { covers, .. } => covers,
            Self::Copy { file, .. } => std::slice::from_ref(file),
            Self::Skip { item, .. } => std::slice::from_ref(item),
            Self::RequireBinary { .. } | Self::Delete { .. } => &[],
        }
    }
}

/// An operation and who proposed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// The operation.
    pub op: Operation,
    /// Who proposed it.
    pub origin: Origin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::{ItemName, PackName};
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn agent(pack: &str, name: &str) -> Result<ItemId, Box<dyn std::error::Error>> {
        Ok(ItemId::agent(&PackName::new(pack)?, &ItemName::new(name)?))
    }

    fn file() -> Result<ItemId, Box<dyn std::error::Error>> {
        Ok(ItemId::skill_file(
            &PackName::new("core")?,
            &ItemName::new("status")?,
            &TreePath::from_parts(&["SKILL.md"]),
        ))
    }

    fn write(covers: Vec<ItemId>) -> Result<Operation, Box<dyn std::error::Error>> {
        Ok(Operation::Write {
            path: OpPath::new(".claude/x")?,
            content: b"x".to_vec(),
            covers,
        })
    }

    fn shown(ids: &[ItemId]) -> Vec<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn binary_name_refusal_maps_to_the_op_refusal() -> TestResult {
        let refused = Operation::require_binary("Git")
            .err()
            .ok_or("Git is not a binary name")?;
        assert_eq!(
            refused.to_string(),
            "require_binary: name: \"Git\" is not a valid name"
        );
        let accepted = Operation::require_binary("git")?;
        assert!(matches!(accepted, Operation::RequireBinary { .. }));
        Ok(())
    }

    #[test]
    fn operation_lists_what_it_covers() -> TestResult {
        let copy = Operation::Copy {
            path: OpPath::new(".claude/x")?,
            file: file()?,
        };
        assert_eq!(shown(copy.covered()), ["core/skill/status/SKILL.md"]);
        let both = write(vec![agent("core", "coder")?, agent("core", "other")?])?;
        assert_eq!(
            shown(both.covered()),
            ["core/agent/coder", "core/agent/other"]
        );
        let skip = Operation::Skip {
            item: agent("core", "coder")?,
            reason: SkipReason::new("no")?,
        };
        assert_eq!(shown(skip.covered()), ["core/agent/coder"]);
        assert!(Operation::require_binary("git")?.covered().is_empty());
        assert!(write(Vec::new())?.covered().is_empty());
        Ok(())
    }

    #[test]
    fn operation_path_of_each_kind() -> TestResult {
        let path = OpPath::new(".claude/x")?;
        let write = write(Vec::new())?;
        let copy = Operation::Copy {
            path: path.clone(),
            file: file()?,
        };
        let delete = Operation::Delete { path: path.clone() };
        for op in [&write, &copy, &delete] {
            assert_eq!(op.path(), Some(&path));
        }
        let skip = Operation::Skip {
            item: agent("core", "coder")?,
            reason: SkipReason::new("no")?,
        };
        assert_eq!(skip.path(), None);
        assert_eq!(Operation::require_binary("git")?.path(), None);
        Ok(())
    }
}
