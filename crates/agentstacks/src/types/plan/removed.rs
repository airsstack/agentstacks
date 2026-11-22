//! What a removed tool may still write.
//!
//! A tool removed from `agentstacks.toml` runs once more so it can take its entries out of the
//! files it shares with the user. Everything else it proposes is dropped.
//!
//! # Responsibilities
//!
//! - [`keep_shared_writes`] keeps only the writes to a shared path.
//!
//! # Non-responsibilities
//!
//! Dropping the files such a tool wrote for itself is the flow's, which makes the deletes from
//! the lock. This filter runs before the duplicate-path check sees the proposals.

use crate::types::op::{OpPath, Operation, Proposal};
use crate::types::plan::Reads;

/// The `proposals` of a removed tool that write a path shared with the user.
///
/// A copy, a skip, a required binary and a write to a path that was not read are dropped.
#[must_use]
pub fn keep_shared_writes(
    proposals: Vec<Proposal>,
    reads: &Reads,
    declared: &[OpPath],
) -> Vec<Proposal> {
    proposals
        .into_iter()
        .filter(|proposal| match &proposal.op {
            Operation::Write { path, .. } => reads.shared(path, declared).is_some(),
            Operation::Copy { .. }
            | Operation::Skip { .. }
            | Operation::RequireBinary { .. }
            | Operation::Delete { .. } => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::{ItemName, PackName, ToolName};
    use crate::types::op::{ItemId, Origin, SkipReason};
    use crate::types::plan::ReadOutcome;
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn proposal(op: Operation) -> Result<Proposal, Box<dyn std::error::Error>> {
        Ok(Proposal {
            op,
            origin: Origin::Tool {
                tool: ToolName::new("claude")?,
            },
        })
    }

    fn write(path: &str) -> Result<Proposal, Box<dyn std::error::Error>> {
        proposal(Operation::Write {
            path: OpPath::new(path)?,
            content: b"x".to_vec(),
            covers: Vec::new(),
        })
    }

    fn all(shared: &str, unread: &str) -> Result<Vec<Proposal>, Box<dyn std::error::Error>> {
        let item = ItemId::agent(&PackName::new("core")?, &ItemName::new("coder")?);
        let file = ItemId::skill_file(
            &PackName::new("core")?,
            &ItemName::new("status")?,
            &TreePath::from_parts(&["SKILL.md"]),
        );
        Ok(vec![
            write(shared)?,
            write(unread)?,
            proposal(Operation::Copy {
                path: OpPath::new(".claude/copy")?,
                file,
            })?,
            proposal(Operation::Skip {
                item,
                reason: SkipReason::new("no")?,
            })?,
            proposal(Operation::require_binary("git")?)?,
        ])
    }

    #[test]
    fn removed_tool_keeps_only_shared_writes() -> TestResult {
        let shared = OpPath::new(".claude/settings.json")?;
        let mut reads = Reads::default();
        reads.record(shared.clone(), ReadOutcome::Absent);
        let kept = keep_shared_writes(
            all(".claude/settings.json", ".claude/other")?,
            &reads,
            std::slice::from_ref(&shared),
        );
        assert_eq!(kept, [write(".claude/settings.json")?]);
        Ok(())
    }

    #[test]
    fn removed_tool_without_reads_keeps_nothing() -> TestResult {
        let shared = OpPath::new(".claude/settings.json")?;
        let kept = keep_shared_writes(
            all(".claude/settings.json", ".claude/other")?,
            &Reads::default(),
            std::slice::from_ref(&shared),
        );
        assert!(kept.is_empty());
        Ok(())
    }
}
