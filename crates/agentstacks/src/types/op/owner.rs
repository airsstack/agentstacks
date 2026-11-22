//! The pack or tool a proposal's file is written for.
//!
//! The lock records who wrote each file. An extension never passes it: the core derives it from
//! what the operation covers and where it came from.
//!
//! # Responsibilities
//!
//! - `owner` gives the derived owner of a proposal: the pack of a copied file, the one pack
//!   a write covers, the pack of a root file the core writes, and otherwise the tool.
//!
//! # Non-responsibilities
//!
//! A path that is shared has no single owner; deciding that belongs to the step rules, which
//! never ask this function about one. The owner of a delete is the lock entry's, not derived.

use crate::types::lock::Owner;
use crate::types::name::PackName;
use crate::types::op::{ItemId, Operation, Origin, Proposal};

/// The owner of `proposal`, when its operation has one.
///
/// A copy is owned by the pack of its file, and a write by the one pack every covered item
/// belongs to. A root file the core writes is owned by its pack. Any other write is owned by the
/// tool that proposed it. A skip, a required binary and a delete have no derived owner.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{PackName, ToolName};
/// use agentstacks::types::op::{OpPath, Operation, Origin, Proposal, owner};
///
/// let proposal = Proposal {
///     op: Operation::Write {
///         path: OpPath::new("rules.md")?,
///         content: Vec::new(),
///         covers: Vec::new(),
///     },
///     origin: Origin::Tool { tool: ToolName::new("claude")? },
/// };
/// assert_eq!(owner(&proposal).map(|owner| owner.to_string()).as_deref(), Some("claude"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn owner(proposal: &Proposal) -> Option<Owner> {
    match &proposal.op {
        Operation::Copy { file, .. } => Some(Owner::from(file.pack())),
        Operation::Write { covers, .. } => Some(write_owner(covers, &proposal.origin)),
        Operation::Skip { .. } | Operation::RequireBinary { .. } | Operation::Delete { .. } => None,
    }
}

/// The owner of a write: the pack of a core root file, the one pack its `covers` all belong to,
/// or the tool.
fn write_owner(covers: &[ItemId], origin: &Origin) -> Owner {
    match origin {
        Origin::Core { pack } => Owner::from(pack),
        Origin::Extension { tool, .. } | Origin::Tool { tool } => {
            single_pack(covers).map_or_else(|| Owner::from(tool), Owner::from)
        }
    }
}

/// The one pack every id in `covers` belongs to; `None` when `covers` is empty or spans several.
fn single_pack(covers: &[ItemId]) -> Option<&PackName> {
    let first = covers.first()?.pack();
    covers.iter().all(|id| id.pack() == first).then_some(first)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::name::{ItemName, ToolName};
    use crate::types::op::{OpPath, SkipReason};
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn extension() -> Result<Origin, Box<dyn std::error::Error>> {
        Ok(Origin::Extension {
            tool: ToolName::new("claude")?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: NonZeroUsize::MIN,
        })
    }

    fn agent(pack: &str, name: &str) -> Result<ItemId, Box<dyn std::error::Error>> {
        Ok(ItemId::agent(&PackName::new(pack)?, &ItemName::new(name)?))
    }

    fn write(covers: Vec<ItemId>, origin: Origin) -> Result<Proposal, Box<dyn std::error::Error>> {
        Ok(Proposal {
            op: Operation::Write {
                path: OpPath::new(".claude/x")?,
                content: Vec::new(),
                covers,
            },
            origin,
        })
    }

    fn shown(proposal: &Proposal) -> Option<String> {
        owner(proposal).map(|owner| owner.to_string())
    }

    #[test]
    fn copy_is_owned_by_the_pack_of_its_file() -> TestResult {
        let file = ItemId::skill_file(
            &PackName::new("core")?,
            &ItemName::new("status")?,
            &TreePath::from_parts(&["SKILL.md"]),
        );
        let proposal = Proposal {
            op: Operation::Copy {
                path: OpPath::new(".claude/x")?,
                file,
            },
            origin: extension()?,
        };
        assert_eq!(shown(&proposal).as_deref(), Some("core"));
        Ok(())
    }

    #[test]
    fn write_covering_one_pack_is_owned_by_that_pack() -> TestResult {
        let covers = vec![
            agent("core", "coder")?,
            ItemId::hook(&PackName::new("core")?, 0),
        ];
        let proposal = write(covers, extension()?)?;
        assert_eq!(shown(&proposal).as_deref(), Some("core"));
        Ok(())
    }

    #[test]
    fn write_without_covers_is_owned_by_the_tool() -> TestResult {
        let proposal = write(Vec::new(), extension()?)?;
        assert_eq!(shown(&proposal).as_deref(), Some("claude"));
        Ok(())
    }

    #[test]
    fn write_covering_several_packs_is_owned_by_the_tool() -> TestResult {
        let covers = vec![agent("core", "coder")?, agent("sdlc", "planner")?];
        let proposal = write(covers, extension()?)?;
        assert_eq!(shown(&proposal).as_deref(), Some("claude"));
        Ok(())
    }

    #[test]
    fn core_root_file_is_owned_by_its_pack() -> TestResult {
        let origin = Origin::Core {
            pack: PackName::new("team-lint")?,
        };
        let proposal = write(Vec::new(), origin)?;
        assert_eq!(shown(&proposal).as_deref(), Some("team-lint"));
        Ok(())
    }

    #[test]
    fn runtime_write_is_owned_by_the_tool() -> TestResult {
        let origin = Origin::Tool {
            tool: ToolName::new("claude")?,
        };
        let proposal = write(Vec::new(), origin)?;
        assert_eq!(shown(&proposal).as_deref(), Some("claude"));
        Ok(())
    }

    #[test]
    fn skip_require_binary_and_delete_have_no_derived_owner() -> TestResult {
        let ops = [
            Operation::Skip {
                item: agent("core", "coder")?,
                reason: SkipReason::new("no")?,
            },
            Operation::require_binary("git")?,
            Operation::Delete {
                path: OpPath::new(".claude/x")?,
            },
        ];
        for op in ops {
            let proposal = Proposal {
                op,
                origin: extension()?,
            };
            assert_eq!(owner(&proposal), None);
        }
        Ok(())
    }
}
