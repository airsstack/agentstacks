//! Steps 8 and 9 of the plan flow: deciding what happens to every path.
//!
//! # Responsibilities
//!
//! - [`decide_steps`] resolves every path an extension or the core proposed, through the
//!   [`Workspace`] port, and hands it with the new bytes, the proposer, the owner, what the
//!   proposing tool read, the lock's hash and the disk state to `decide_proposed`.
//! - It does the same for every `[[file]]` of the lock that nothing proposes now, whoever owns it,
//!   through `decide_unproposed`: the files of a removed pack or tool, and the files a pack or
//!   tool still listed stopped proposing.
//! - [`Decided`] is the flow's carrier for the steps and the refusals.
//!
//! # Non-responsibilities
//!
//! Every refusal and every action is decided by the rules in `types`; this file only gathers the
//! facts they need. It runs only when every earlier step succeeded, so no path is resolved
//! against a half-built plan.

use std::collections::BTreeSet;

use crate::app::plan::read::Inputs;
use crate::app::plan::tools::Ran;
use crate::port::Workspace;
use crate::types::op::{OpPath, Operation, owner};
use crate::types::plan::{ProposedPath, Step, decide_proposed, decide_unproposed};
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::tree::ContentHash;

/// What deciding every path found.
pub(super) struct Decided {
    /// The step of every path that gets one.
    pub(super) steps: Vec<Step>,
    /// Every path that is refused.
    pub(super) refusals: Diagnostics,
}

/// Decides the step of every proposed path and of every locked path nothing proposes.
pub(super) fn decide_steps<W: Workspace>(inputs: &Inputs, ran: &Ran, workspace: &W) -> Decided {
    let mut decided = Decided {
        steps: Vec::new(),
        refusals: Diagnostics::new(),
    };
    let locked = inputs.lock.as_ref().map_or(&[][..], |lock| lock.files());
    let mut proposed = BTreeSet::new();

    for proposal in ran.proposals.iter() {
        let (path, new) = match &proposal.op {
            Operation::Write { path, content, .. } => (path, content.clone()),
            // Every file id of the payload has its final bytes for the tool that was given it, so a
            // copy always finds them under its proposer.
            Operation::Copy { path, file } => {
                let bytes = proposal
                    .origin
                    .tool()
                    .and_then(|tool| ran.files.get(&(tool.clone(), file.clone())));
                match bytes {
                    Some(bytes) => (path, bytes.clone()),
                    None => continue,
                }
            }
            Operation::Skip { .. } | Operation::RequireBinary { .. } | Operation::Delete { .. } => {
                continue;
            }
        };
        proposed.insert(path.clone());
        let resolution = match workspace.resolve(path) {
            Ok(resolution) => resolution,
            Err(error) => {
                decided
                    .refusals
                    .push(Diagnostic::new(Location::file(path.as_str()), error));
                continue;
            }
        };
        let shared = proposal
            .origin
            .tool()
            .and_then(|tool| ran.runs.get(tool))
            .and_then(|run| run.reads.shared(path, &run.declared));
        let input = ProposedPath {
            path: path.clone(),
            new,
            origin: proposal.origin.clone(),
            owner: owner(proposal),
            shared,
            locked: lock_hash(locked, path),
            resolution,
        };
        match decide_proposed(input) {
            Ok(step) => decided.steps.push(step),
            Err(refusal) => decided.refusals.push(refusal),
        }
    }

    for entry in locked {
        // A lock path is a valid project path, so it always becomes an operation path.
        let Ok(path) = OpPath::new(&entry.path().normalized()) else {
            continue;
        };
        if proposed.contains(&path) {
            continue;
        }
        let resolution = match workspace.resolve(&path) {
            Ok(resolution) => resolution,
            Err(error) => {
                decided
                    .refusals
                    .push(Diagnostic::new(Location::file(path.as_str()), error));
                continue;
            }
        };
        match decide_unproposed(path, entry, &resolution) {
            Ok(step) => decided.steps.push(step),
            Err(refusal) => decided.refusals.push(refusal),
        }
    }
    decided
}

/// The hash the lock records for `path`.
fn lock_hash(locked: &[crate::types::lock::FileEntry], path: &OpPath) -> Option<ContentHash> {
    locked
        .iter()
        .find(|entry| entry.path().normalized() == path.as_str())
        .map(|entry| entry.hash().clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::app::plan::read::read_inputs;
    use crate::app::plan::testing::{
        PROJECT, Scenario, TestResult, git_locked, hash_of, lock_text, locked_file, locked_pack,
        op, origin, shown, tool,
    };
    use crate::app::plan::tools::ToolRun;
    use crate::port::FakeWorkspace;
    use crate::port::{FakeFileTree, WorkspaceCall};
    use crate::types::lock::{FileEntry, LockContents};
    use crate::types::name::ToolName;
    use crate::types::op::{Proposal, Proposals};
    use crate::types::plan::{Action, DiskState, ReadOutcome, Reads};

    fn lock_of(files: Vec<FileEntry>) -> Result<String, Box<dyn std::error::Error>> {
        lock_text(LockContents {
            packs: vec![locked_pack(
                "core",
                git_locked(crate::app::plan::testing::REV_A, Some("v1"), None)?,
                hash_of(&FakeFileTree::new("p"))?,
            )?],
            files,
            ..LockContents::default()
        })
    }

    fn write(
        line: usize,
        path: &str,
        bytes: &[u8],
    ) -> Result<Proposal, Box<dyn std::error::Error>> {
        Ok(Proposal {
            op: Operation::Write {
                path: op(path)?,
                content: bytes.to_vec(),
                covers: Vec::new(),
            },
            origin: origin("claude", line)?,
        })
    }

    fn ran(
        proposals: Vec<Proposal>,
        runs: Vec<(ToolName, ToolRun)>,
    ) -> Result<Ran, Box<dyn std::error::Error>> {
        let mut record = Proposals::new();
        for proposal in proposals {
            record.propose(proposal)?;
        }
        Ok(Ran {
            proposals: record,
            runs: runs.into_iter().collect(),
            skips: Vec::new(),
            files: BTreeMap::new(),
            refusals: Diagnostics::new(),
        })
    }

    fn decide(
        workspace: FakeWorkspace,
        lock: Option<&str>,
        ran: &Ran,
    ) -> Result<(Decided, FakeWorkspace), Box<dyn std::error::Error>> {
        let scenario = Scenario::new(PROJECT).workspace(|_| workspace.project(PROJECT));
        let scenario = match lock {
            Some(text) => scenario.lock(text),
            None => scenario,
        };
        let inputs = read_inputs(&scenario.workspace).map_err(|found| shown(found).join("; "))?;
        let decided = decide_steps(&inputs, ran, &scenario.workspace);
        Ok((decided, scenario.workspace))
    }

    fn steps_of(decided: &Decided) -> Vec<(String, Action)> {
        decided
            .steps
            .iter()
            .map(|step| (step.path.to_string(), step.action))
            .collect()
    }

    #[test]
    fn decide_steps_forgets_an_absent_locked_path() -> TestResult {
        let lock = lock_of(vec![locked_file(".claude/old.md", b"old", "core")?])?;
        let (decided, _) = decide(FakeWorkspace::new(), Some(&lock), &ran(vec![], vec![])?)?;
        assert_eq!(
            steps_of(&decided),
            [(".claude/old.md".to_owned(), Action::Forget)]
        );
        Ok(())
    }

    #[test]
    fn decide_steps_forgets_a_locked_path_now_a_dangling_symlink() -> TestResult {
        let lock = lock_of(vec![locked_file(".claude/old.md", b"old", "core")?])?;
        let workspace = FakeWorkspace::new().symlink(&op(".claude/old.md")?, &op("missing")?);
        let (decided, _) = decide(workspace, Some(&lock), &ran(vec![], vec![])?)?;
        assert!(decided.refusals.is_empty());
        assert_eq!(
            steps_of(&decided),
            [(".claude/old.md".to_owned(), Action::Forget)]
        );
        Ok(())
    }

    #[test]
    fn decide_steps_refuses_to_delete_a_changed_locked_path() -> TestResult {
        let lock = lock_of(vec![locked_file(".claude/old.md", b"old", "core")?])?;
        let workspace = FakeWorkspace::new().file(&op(".claude/old.md")?, b"edited");
        let (decided, _) = decide(workspace, Some(&lock), &ran(vec![], vec![])?)?;
        assert_eq!(
            shown(decided.refusals),
            [".claude/old.md: changed since the last apply; not deleting it"]
        );
        Ok(())
    }

    #[test]
    fn decide_steps_refuses_a_proposed_path_that_is_not_a_regular_file() -> TestResult {
        let workspace = FakeWorkspace::new().not_regular(&op(".claude/x")?);
        let ran = ran(vec![write(3, ".claude/x", b"x")?], vec![])?;
        let (decided, _) = decide(workspace, None, &ran)?;
        assert_eq!(
            shown(decided.refusals),
            [".claude/x: is not a regular file"]
        );
        Ok(())
    }

    fn settings_run(read: &[u8]) -> Result<(ToolName, ToolRun), Box<dyn std::error::Error>> {
        let path = op(".claude/settings.json")?;
        let mut reads = Reads::default();
        reads.record(
            path.clone(),
            ReadOutcome::Bytes(ContentHash::of_bytes(read)),
        );
        Ok((
            tool("claude")?,
            ToolRun {
                reads,
                declared: vec![path],
            },
        ))
    }

    #[test]
    fn decide_steps_refuses_a_shared_file_changed_after_it_was_read() -> TestResult {
        let workspace = FakeWorkspace::new().file(&op(".claude/settings.json")?, b"other");
        let ran = ran(
            vec![write(4, ".claude/settings.json", b"new")?],
            vec![settings_run(b"old")?],
        )?;
        let (decided, _) = decide(workspace, None, &ran)?;
        assert_eq!(
            shown(decided.refusals),
            [".claude/settings.json: changed after it was read; run plan again"]
        );
        Ok(())
    }

    #[test]
    fn decide_steps_keeps_old_bytes_for_a_shared_file() -> TestResult {
        let workspace = FakeWorkspace::new().file(&op(".claude/settings.json")?, b"old");
        let ran = ran(
            vec![write(4, ".claude/settings.json", b"new")?],
            vec![settings_run(b"old")?],
        )?;
        let (decided, _) = decide(workspace, None, &ran)?;
        assert!(decided.refusals.is_empty());
        let step = decided.steps.first().ok_or("a step")?;
        assert_eq!(step.action, Action::Update);
        assert_eq!(step.old.as_deref(), Some(&b"old"[..]));
        assert_eq!(step.owner, None);
        Ok(())
    }

    #[test]
    fn decide_steps_reports_a_resolve_failure_with_the_io_error() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::Resolve(op(".claude/x")?));
        let ran = ran(vec![write(3, ".claude/x", b"x")?], vec![])?;
        let (decided, _) = decide(workspace, None, &ran)?;
        assert_eq!(
            shown(decided.refusals),
            [format!(".claude/x: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn decide_steps_records_the_disk_state_each_step_was_decided_on() -> TestResult {
        let lock = lock_of(vec![locked_file(".claude/old.md", b"old", "core")?])?;
        let ran = ran(vec![write(3, ".claude/new.md", b"new")?], vec![])?;
        let (decided, _) = decide(FakeWorkspace::new(), Some(&lock), &ran)?;
        let by_path: BTreeMap<String, Option<DiskState>> = decided
            .steps
            .iter()
            .map(|step| (step.path.to_string(), step.decided_on.clone()))
            .collect();
        assert_eq!(
            by_path.get(".claude/new.md"),
            Some(&Some(DiskState::Absent))
        );
        assert_eq!(by_path.get(".claude/old.md"), Some(&None));
        Ok(())
    }

    #[test]
    fn decide_steps_decides_a_locked_path_a_listed_pack_stopped_proposing() -> TestResult {
        let lock = lock_of(vec![locked_file(
            ".claude/skills/old/SKILL.md",
            b"old",
            "core",
        )?])?;
        let workspace = FakeWorkspace::new().file(&op(".claude/skills/old/SKILL.md")?, b"old");
        let (decided, _) = decide(workspace, Some(&lock), &ran(vec![], vec![])?)?;
        assert!(decided.refusals.is_empty());
        let step = decided.steps.first().ok_or("a step")?;
        assert_eq!(step.action, Action::Delete);
        assert_eq!(
            step.owner.as_ref().map(ToString::to_string).as_deref(),
            Some("core")
        );
        Ok(())
    }
}
