//! Step 4 of an apply: put the staged files in place, then delete.
//!
//! # Responsibilities
//!
//! - [`swap`] renames each staged temporary file onto its target in plan order, then deletes each
//!   file a removed pack wrote in plan order. At the first failure it stops and removes the
//!   temporary files not yet renamed.
//! - [`Swapped`] is the flow-internal carrier of which paths landed and which refusals were found,
//!   which the lock write reads to choose what the lock holds.
//!
//! # Non-responsibilities
//!
//! Writing through a symlink, keeping a file's mode and leaving empty directories after a delete
//! belong to the workspace; which entries the lock keeps belongs to the lock rule.

use std::collections::BTreeSet;
use std::io;

use crate::app::apply::stage::Staged;
use crate::port::Workspace;
use crate::types::op::OpPath;
use crate::types::plan::{Action, Plan};
use crate::types::report::{Diagnostic, Location};

/// What step 4 did: the paths whose rename or delete was performed, and the refusals found.
#[derive(Debug, Default)]
pub(super) struct Swapped {
    landed: BTreeSet<OpPath>,
    failures: Vec<Diagnostic>,
}

impl Swapped {
    /// The paths whose rename or delete was performed.
    pub(super) const fn landed(&self) -> &BTreeSet<OpPath> {
        &self.landed
    }

    /// Whether step 4 stopped at a failure.
    pub(super) const fn failed(&self) -> bool {
        !self.failures.is_empty()
    }

    fn fail(&mut self, path: &OpPath, error: io::Error) {
        self.failures
            .push(Diagnostic::new(Location::file(path.as_str()), error));
    }

    /// The refusals found, which are empty when every step landed.
    pub(super) fn into_failures(self) -> Vec<Diagnostic> {
        self.failures
    }
}

/// Swaps every staged file, then performs every delete.
///
/// A failure does not return early with an error: it is held in the result, because the lock is
/// still written afterwards.
pub(super) fn swap<W: Workspace>(plan: &Plan, staged: Staged, workspace: &W) -> Swapped {
    let mut swapped = Swapped::default();
    let paths = staged.into_paths();
    for (index, path) in paths.iter().enumerate() {
        match workspace.swap(path) {
            Ok(()) => {
                swapped.landed.insert(path.clone());
            }
            Err(error) => {
                swapped.fail(path, error);
                for left in &paths[index..] {
                    if let Err(error) = workspace.discard(left) {
                        swapped.fail(left, error);
                    }
                }
                return swapped;
            }
        }
    }
    for step in plan
        .steps()
        .iter()
        .filter(|step| step.action == Action::Delete)
    {
        match workspace.delete(&step.path) {
            Ok(()) => {
                swapped.landed.insert(step.path.clone());
            }
            Err(error) => {
                swapped.fail(&step.path, error);
                return swapped;
            }
        }
    }
    swapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::apply::stage::stage;
    use crate::app::apply::testing::{Fallible, TestResult, file, path, plan, shown, six, step};
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::plan::Action;

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    /// Stages `plan` against `workspace` and swaps it.
    fn staged_and_swapped(plan: &Plan, workspace: &FakeWorkspace) -> Fallible<Swapped> {
        let staged = stage(plan, workspace).map_err(|r| format!("{:?}", shown(r)))?;
        Ok(swap(plan, staged, workspace))
    }

    fn failures(swapped: Swapped) -> Vec<String> {
        swapped
            .into_failures()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn mutations(workspace: &FakeWorkspace) -> Vec<String> {
        workspace
            .calls()
            .iter()
            .filter_map(|call| match call {
                WorkspaceCall::Swap(path) => Some(format!("swap {}", path.as_str())),
                WorkspaceCall::Delete(path) => Some(format!("delete {}", path.as_str())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn swap_swaps_every_temp_then_deletes() -> TestResult {
        let (plan, workspace) = six()?;
        let swapped = staged_and_swapped(&plan, &workspace)?;
        assert_eq!(
            mutations(&workspace),
            [
                "swap .claude/a",
                "swap .claude/b",
                "swap .claude/c",
                "delete .claude/e"
            ]
        );
        assert!(!swapped.failed());
        assert_eq!(
            workspace.file_bytes(&path(".claude/b")?),
            Some(b"new b".to_vec())
        );
        assert_eq!(workspace.file_bytes(&path(".claude/e")?), None);
        Ok(())
    }

    #[test]
    fn swap_stops_at_the_first_failed_swap() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Swap(path(".claude/b")?));
        let swapped = staged_and_swapped(&plan, &workspace)?;
        assert_eq!(failures(swapped), [format!(".claude/b: {}", denied())]);
        assert_eq!(mutations(&workspace), ["swap .claude/a", "swap .claude/b"]);
        assert!(workspace.temps().is_empty());
        Ok(())
    }

    #[test]
    fn swap_stops_at_a_failed_delete() -> TestResult {
        let writes = step(
            ".claude/a",
            Action::Update,
            Some("core"),
            Some(b"new a"),
            Some(file(b"old a")),
        )?;
        let first = step(
            ".claude/d1",
            Action::Delete,
            Some("core"),
            None,
            Some(file(b"d1")),
        )?;
        let second = step(
            ".claude/d2",
            Action::Delete,
            Some("core"),
            None,
            Some(file(b"d2")),
        )?;
        let plan = plan(vec![writes, first, second], Vec::new())?;
        let workspace = FakeWorkspace::new()
            .file(&path(".claude/a")?, b"old a")
            .file(&path(".claude/d1")?, b"d1")
            .file(&path(".claude/d2")?, b"d2")
            .fail(WorkspaceCall::Delete(path(".claude/d1")?));
        let swapped = staged_and_swapped(&plan, &workspace)?;
        assert_eq!(failures(swapped), [format!(".claude/d1: {}", denied())]);
        assert_eq!(
            mutations(&workspace),
            ["swap .claude/a", "delete .claude/d1"]
        );
        assert_eq!(
            workspace.file_bytes(&path(".claude/a")?),
            Some(b"new a".to_vec())
        );
        Ok(())
    }

    #[test]
    fn swap_records_exactly_the_steps_that_landed() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Swap(path(".claude/b")?));
        let swapped = staged_and_swapped(&plan, &workspace)?;
        assert!(swapped.failed());
        let landed: Vec<&str> = swapped.landed().iter().map(OpPath::as_str).collect();
        assert_eq!(landed, [".claude/a"]);
        Ok(())
    }

    #[test]
    fn swap_reports_a_failed_cleanup_of_a_temp_it_never_swapped() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace
            .fail(WorkspaceCall::Swap(path(".claude/b")?))
            .fail(WorkspaceCall::Discard(path(".claude/c")?));
        let swapped = staged_and_swapped(&plan, &workspace)?;
        assert_eq!(
            failures(swapped),
            [
                format!(".claude/b: {}", denied()),
                format!(".claude/c: {}", denied()),
            ]
        );
        Ok(())
    }
}
