//! Steps 2 and 3 of an apply: write the new files beside their targets, then check the disk.
//!
//! # Responsibilities
//!
//! - [`stage`] writes a temporary file for every step that writes, checks every step that touches
//!   disk against where its path leads now, and removes every temporary file when it refuses.
//! - [`Staged`] is the flow-internal carrier of the paths that were staged, in plan order.
//!
//! # Non-responsibilities
//!
//! Whether a path changed is the rule [`race`](crate::types::plan::race); where the temporary file
//! lives is the workspace's. Renaming the files into place belongs to the swap.

use std::io;

use crate::port::Workspace;
use crate::types::op::OpPath;
use crate::types::plan::{Plan, race};
use crate::types::report::{Diagnostic, Diagnostics, Location};

/// The paths whose new content sits in a temporary file, in plan order.
#[derive(Debug)]
pub(super) struct Staged {
    paths: Vec<OpPath>,
}

impl Staged {
    /// The staged paths, in plan order.
    pub(super) fn into_paths(self) -> Vec<OpPath> {
        self.paths
    }
}

/// Stages every write, then checks every step that touches disk.
///
/// # Errors
///
/// Every refusal found: a target that changed after the plan or now leads outside the project, or
/// the I/O failure of a write or a look at the disk. Before it refuses, the flow discards the
/// temporary file of every write it started, including one whose write failed, so none is left
/// behind. A temporary file that cannot be discarded stays and adds a refusal.
pub(super) fn stage<W: Workspace>(plan: &Plan, workspace: &W) -> Result<Staged, Diagnostics> {
    let mut staged = Staged { paths: Vec::new() };
    let mut refusals = Diagnostics::new();
    let failed = write_temps(plan, workspace, &mut staged, &mut refusals);
    if refusals.is_empty() {
        check(plan, workspace, &mut refusals);
    }
    if refusals.is_empty() {
        return Ok(staged);
    }
    for path in staged.paths.iter().chain(&failed) {
        if let Err(error) = workspace.discard(path) {
            refusals.push(Diagnostic::new(Location::file(path.as_str()), error));
        }
    }
    Err(refusals)
}

/// Stages each step that writes, in plan order, and stops at the first failure.
///
/// Returns the path whose write was started and failed, which may have left a partial temporary
/// file; `None` when every write succeeded or the failure came before a write was started.
fn write_temps<W: Workspace>(
    plan: &Plan,
    workspace: &W,
    staged: &mut Staged,
    refusals: &mut Diagnostics,
) -> Option<OpPath> {
    for step in plan.steps().iter().filter(|step| step.action.writes()) {
        let Some(bytes) = step.new.as_deref() else {
            let error = io::Error::new(io::ErrorKind::InvalidData, "has no content to write");
            refusals.push(Diagnostic::new(Location::file(step.path.as_str()), error));
            return None;
        };
        match workspace.stage(&step.path, bytes) {
            Ok(()) => staged.paths.push(step.path.clone()),
            Err(error) => {
                refusals.push(Diagnostic::new(Location::file(step.path.as_str()), error));
                return Some(step.path.clone());
            }
        }
    }
    None
}

/// Looks at every step that touches disk and reports each one that changed, leads outside the
/// project, or could not be looked at.
fn check<W: Workspace>(plan: &Plan, workspace: &W, refusals: &mut Diagnostics) {
    for step in plan
        .steps()
        .iter()
        .filter(|step| step.action.touches_disk())
    {
        match workspace.resolve(&step.path) {
            Ok(fresh) => {
                let refusal = race(step, &fresh).and_then(|found| found.refusal(&step.path));
                refusals.extend(refusal);
            }
            Err(error) => {
                refusals.push(Diagnostic::new(Location::file(step.path.as_str()), error));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::apply::testing::{TestResult, file, path, plan, shown, six, step};
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::plan::{Action, DiskState};

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    fn calls_of(
        workspace: &FakeWorkspace,
        keep: impl Fn(&WorkspaceCall) -> Option<String>,
    ) -> Vec<String> {
        workspace.calls().iter().filter_map(keep).collect()
    }

    fn staged_paths(workspace: &FakeWorkspace) -> Vec<String> {
        calls_of(workspace, |call| match call {
            WorkspaceCall::Stage(path) => Some(path.as_str().to_owned()),
            _ => None,
        })
    }

    fn resolved_paths(workspace: &FakeWorkspace) -> Vec<String> {
        calls_of(workspace, |call| match call {
            WorkspaceCall::Resolve(path) => Some(path.as_str().to_owned()),
            _ => None,
        })
    }

    fn wrote_nothing_else(workspace: &FakeWorkspace) -> bool {
        workspace.temps().is_empty()
            && workspace.lock_written().is_none()
            && !workspace
                .calls()
                .iter()
                .any(|call| matches!(call, WorkspaceCall::Swap(_) | WorkspaceCall::Delete(_)))
    }

    #[test]
    fn stage_writes_a_temp_only_for_create_update_and_restore() -> TestResult {
        let (plan, workspace) = six()?;
        let staged = stage(&plan, &workspace).map_err(|r| format!("{:?}", shown(r)))?;
        assert_eq!(
            staged_paths(&workspace),
            [".claude/a", ".claude/b", ".claude/c"]
        );
        let paths = staged.into_paths();
        let paths: Vec<&str> = paths.iter().map(OpPath::as_str).collect();
        assert_eq!(paths, [".claude/a", ".claude/b", ".claude/c"]);
        assert_eq!(workspace.temps().len(), 3);
        Ok(())
    }

    #[test]
    fn stage_refuses_every_target_changed_after_plan() -> TestResult {
        let (plan, workspace) = six()?;
        workspace.set_file(&path(".claude/b")?, b"edited b");
        workspace.set_file(&path(".claude/a")?, b"appeared a");
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(
            shown(refusals),
            [
                ".claude/a: changed after plan; run plan again",
                ".claude/b: changed after plan; run plan again",
            ]
        );
        assert!(wrote_nothing_else(&workspace));
        Ok(())
    }

    #[test]
    fn stage_checks_a_shared_file_against_what_read_returned() -> TestResult {
        let shared = step(
            ".claude/settings.json",
            Action::Update,
            None,
            Some(b"{\"new\":1}"),
            Some(file(b"{\"user\":1}")),
        )?;
        let plan = plan(vec![shared], Vec::new())?;
        let same = FakeWorkspace::new().file(&path(".claude/settings.json")?, b"{\"user\":1}");
        assert!(stage(&plan, &same).is_ok());

        let differs = FakeWorkspace::new().file(&path(".claude/settings.json")?, b"{\"user\":2}");
        let refusals = stage(&plan, &differs).err().ok_or("a refusal")?;
        assert_eq!(
            shown(refusals),
            [".claude/settings.json: changed after plan; run plan again"]
        );
        assert!(differs.temps().is_empty());
        Ok(())
    }

    #[test]
    fn stage_checks_delete_steps_and_skips_unchanged() -> TestResult {
        let (plan, workspace) = six()?;
        workspace.set_file(&path(".claude/e")?, b"edited e");
        workspace.set_file(&path(".claude/d")?, b"edited d");
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(
            shown(refusals),
            [".claude/e: changed after plan; run plan again"]
        );
        assert!(!resolved_paths(&workspace).contains(&".claude/d".to_owned()));
        Ok(())
    }

    #[test]
    fn stage_skips_the_race_check_for_forget() -> TestResult {
        let (plan, workspace) = six()?;
        workspace.set_file(&path(".claude/f")?, b"now a file");
        assert!(stage(&plan, &workspace).is_ok());
        assert!(!resolved_paths(&workspace).contains(&".claude/f".to_owned()));
        Ok(())
    }

    #[test]
    fn stage_ignores_a_mode_only_change() -> TestResult {
        let update = step(
            ".claude/b",
            Action::Update,
            Some("core"),
            Some(b"new b"),
            Some(file(b"old b")),
        )?;
        let plan = plan(vec![update], Vec::new())?;
        let workspace = FakeWorkspace::new().file_with_mode(&path(".claude/b")?, b"old b", 0o755);
        assert!(stage(&plan, &workspace).is_ok());
        Ok(())
    }

    #[test]
    fn stage_refuses_a_delete_that_now_resolves_outside() -> TestResult {
        let delete = step(
            ".claude/e",
            Action::Delete,
            Some("core"),
            None,
            Some(file(b"old e")),
        )?;
        let plan = plan(vec![delete], Vec::new())?;
        let workspace = FakeWorkspace::new().outside(&path(".claude/e")?);
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [".claude/e: resolves outside the project"]);
        assert_eq!(resolved_paths(&workspace), [".claude/e"]);
        assert!(workspace.temps().is_empty());
        Ok(())
    }

    #[test]
    fn stage_io_failure_at_the_second_write_discards_the_first_temp() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Stage(path(".claude/b")?));
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [format!(".claude/b: {}", denied())]);
        assert!(wrote_nothing_else(&workspace));
        Ok(())
    }

    #[test]
    fn stage_discards_the_temp_of_the_write_that_failed() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Stage(path(".claude/b")?));
        stage(&plan, &workspace).err().ok_or("a refusal")?;
        let discarded = calls_of(&workspace, |call| match call {
            WorkspaceCall::Discard(path) => Some(path.as_str().to_owned()),
            _ => None,
        });
        assert_eq!(discarded, [".claude/a", ".claude/b"]);
        Ok(())
    }

    #[test]
    fn stage_io_failure_while_checking_discards_every_temp() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Resolve(path(".claude/c")?));
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [format!(".claude/c: {}", denied())]);
        assert!(wrote_nothing_else(&workspace));
        Ok(())
    }

    #[test]
    fn stage_refuses_a_write_step_that_carries_no_bytes() -> TestResult {
        let empty = step(
            ".claude/a",
            Action::Create,
            Some("core"),
            None,
            Some(DiskState::Absent),
        )?;
        let plan = plan(vec![empty], Vec::new())?;
        let workspace = FakeWorkspace::new();
        let refusals = stage(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [".claude/a: has no content to write"]);
        assert!(workspace.calls().is_empty());
        Ok(())
    }
}
