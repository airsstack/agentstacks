//! The apply flow: the five steps in order.
//!
//! # Responsibilities
//!
//! - [`apply`] runs the steps that act on a plan that was shown, and returns every refusal found.
//!
//! # Non-responsibilities
//!
//! It holds no rule and takes no port but the workspace, so it cannot plan again.

use crate::app::apply::lock::write;
use crate::app::apply::stage::stage;
use crate::app::apply::swap::swap;
use crate::port::Workspace;
use crate::types::plan::Plan;
use crate::types::report::Diagnostics;

/// Applies `plan` through `workspace`, the only writer.
///
/// A [`Plan`] holds no refusal, because planning returns none when it finds one, so the first step
/// holds by type. The rest run in this order:
///
/// 1. every new file is written to a temporary file beside its target;
/// 2. every target that will change is looked at again, and any that changed after the plan or now
///    leads outside the project refuses the whole apply with every temporary file removed;
/// 3. the temporary files are renamed onto their targets, then the files a removed pack wrote are
///    deleted, stopping at the first failure;
/// 4. `agentstacks.lock` is written last. After a failure in step 3 it holds only the steps that
///    landed, so the next plan sees the disk as it is.
///
/// # Errors
///
/// Every refusal found: from the second step before anything is renamed or deleted, or from the
/// third step and the lock write, both of which are returned when both fail.
pub fn apply<W: Workspace>(plan: &Plan, workspace: &W) -> Result<(), Diagnostics> {
    let staged = stage(plan, workspace)?;
    let swapped = swap(plan, staged, workspace);
    let lock = write(plan, &swapped, workspace);
    let mut refusals = Diagnostics::new();
    refusals.extend(swapped.into_failures());
    if let Err(lock) = lock {
        refusals.extend(lock);
    }
    refusals.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::apply::testing::{
        Fallible, TestResult, hash_in, path, plan, shown, six, step, written,
    };
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::op::OpPath;
    use crate::types::plan::{Action, Link, Resolution};
    use crate::types::tree::ContentHash;

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    fn call(kind: fn(OpPath) -> WorkspaceCall, at: &str) -> Fallible<WorkspaceCall> {
        Ok(kind(path(at)?))
    }

    #[test]
    fn apply_runs_stage_check_swap_then_lock_in_order() -> TestResult {
        let (plan, workspace) = six()?;
        apply(&plan, &workspace).map_err(|r| format!("{:?}", shown(r)))?;
        let expected = vec![
            call(WorkspaceCall::Stage, ".claude/a")?,
            call(WorkspaceCall::Stage, ".claude/b")?,
            call(WorkspaceCall::Stage, ".claude/c")?,
            call(WorkspaceCall::Resolve, ".claude/a")?,
            call(WorkspaceCall::Resolve, ".claude/b")?,
            call(WorkspaceCall::Resolve, ".claude/c")?,
            call(WorkspaceCall::Resolve, ".claude/e")?,
            call(WorkspaceCall::Swap, ".claude/a")?,
            call(WorkspaceCall::Swap, ".claude/b")?,
            call(WorkspaceCall::Swap, ".claude/c")?,
            call(WorkspaceCall::Delete, ".claude/e")?,
            WorkspaceCall::WriteLock,
        ];
        assert_eq!(workspace.calls(), expected);
        Ok(())
    }

    #[test]
    fn apply_applies_a_forget_of_a_dangling_symlink() -> TestResult {
        let forget = step(".claude/f", Action::Forget, Some("core"), None, None)?;
        let plan = plan(vec![forget], Vec::new())?;
        let workspace = FakeWorkspace::new().symlink(&path(".claude/f")?, &path(".claude/gone")?);
        apply(&plan, &workspace).map_err(|r| format!("{:?}", shown(r)))?;
        let Resolution::Inside { link, .. } = workspace.resolve(&path(".claude/f")?)? else {
            return Err("the link stays inside".into());
        };
        assert_eq!(link, Link::Symlink);
        assert_eq!(hash_in(&written(&workspace)?, ".claude/f"), None);
        Ok(())
    }

    #[test]
    fn apply_touches_nothing_on_a_race() -> TestResult {
        let (plan, workspace) = six()?;
        workspace.set_file(&path(".claude/b")?, b"edited b");
        let refusals = apply(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(
            shown(refusals),
            [".claude/b: changed after plan; run plan again"]
        );
        assert!(workspace.calls().iter().all(|call| !matches!(
            call,
            WorkspaceCall::Swap(_) | WorkspaceCall::Delete(_) | WorkspaceCall::WriteLock
        )));
        assert!(workspace.temps().is_empty());
        Ok(())
    }

    #[test]
    fn apply_writes_the_r17_lock_after_a_failed_swap() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Swap(path(".claude/b")?));
        let refusals = apply(&plan, &workspace).err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [format!(".claude/b: {}", denied())]);
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/a"),
            Some(ContentHash::of_bytes(b"new a"))
        );
        assert_eq!(
            hash_in(&lock, ".claude/b"),
            Some(ContentHash::of_bytes(b"old b"))
        );
        Ok(())
    }

    #[test]
    fn apply_reports_a_swap_failure_and_a_lock_failure_together() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace
            .fail(WorkspaceCall::Swap(path(".claude/b")?))
            .fail(WorkspaceCall::WriteLock);
        let refusals = apply(&plan, &workspace).err().ok_or("a refusal")?;
        let mut lines = shown(refusals);
        lines.sort();
        assert_eq!(
            lines,
            [
                format!(".claude/b: {}", denied()),
                format!("agentstacks.lock: {}", denied()),
            ]
        );
        Ok(())
    }

    #[test]
    fn apply_leaves_unchanged_and_forget_steps_alone_on_disk() -> TestResult {
        let (plan, workspace) = six()?;
        apply(&plan, &workspace).map_err(|r| format!("{:?}", shown(r)))?;
        let touched: Vec<String> = workspace
            .calls()
            .iter()
            .filter_map(|call| match call {
                WorkspaceCall::Stage(p) | WorkspaceCall::Swap(p) | WorkspaceCall::Delete(p) => {
                    Some(p.as_str().to_owned())
                }
                _ => None,
            })
            .collect();
        assert!(!touched.contains(&".claude/d".to_owned()));
        assert!(!touched.contains(&".claude/f".to_owned()));
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/d"),
            Some(ContentHash::of_bytes(b"new d"))
        );
        assert_eq!(hash_in(&lock, ".claude/f"), None);
        Ok(())
    }
}
