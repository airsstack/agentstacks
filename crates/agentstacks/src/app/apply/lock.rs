//! Step 5 of an apply: write `agentstacks.lock` last.
//!
//! # Responsibilities
//!
//! - [`write`] chooses what the lock holds, the plan's lock in full when every step landed and
//!   the lock the plan rules build from the steps that landed when step 4 failed, then turns it
//!   into text and hands it to the workspace.
//!
//! # Non-responsibilities
//!
//! Which entries a partial lock keeps is the rule [`lock_contents`](crate::types::plan::lock_contents);
//! the lock's text is [`write_lock`](crate::types::lock::write_lock); the temporary file, the rename
//! and the lock's mode belong to the workspace.

use std::io;
use std::path::Path;

use crate::app::apply::swap::Swapped;
use crate::port::Workspace;
use crate::types::lock::{Lockfile, write_lock};
use crate::types::plan::{Landed, Plan, lock_contents};
use crate::types::report::{Diagnostic, Diagnostics, Location};

/// The lock file, relative to the project.
const LOCK_FILE: &str = "agentstacks.lock";

/// Writes the lock for what `swapped` did.
///
/// # Errors
///
/// The I/O failure of the write, located at `agentstacks.lock`. Files that landed stay where they
/// landed.
pub(super) fn write<W: Workspace>(
    plan: &Plan,
    swapped: &Swapped,
    workspace: &W,
) -> Result<(), Diagnostics> {
    let lock = if swapped.failed() {
        partial(plan, swapped)?
    } else {
        plan.lock().clone()
    };
    workspace
        .write_lock(&write_lock(&lock))
        .map_err(|error| Diagnostics::from(Diagnostic::new(Location::file(LOCK_FILE), error)))
}

/// The lock after a failed swap: the plan's tools, packs and airsl pin, and the file entries of
/// the steps that landed, with the old entries standing for those that did not.
fn partial(plan: &Plan, swapped: &Swapped) -> Result<Lockfile, Diagnostics> {
    let new = plan.lock();
    let contents = lock_contents(
        new.tools().to_vec(),
        new.packs().to_vec(),
        plan.steps(),
        new.airsl().cloned(),
        Landed::Only {
            paths: swapped.landed(),
            old: plan.old_files(),
        },
    )
    .map_err(|error| {
        let error = io::Error::new(io::ErrorKind::InvalidData, error);
        Diagnostics::from(Diagnostic::new(Location::file(LOCK_FILE), error))
    })?;
    Lockfile::new(Path::new(LOCK_FILE), contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::apply::stage::stage;
    use crate::app::apply::swap::swap;
    use crate::app::apply::testing::{
        CORE_VERSION, Fallible, TestResult, file, hash_in, old, path, plan, shown, six, step,
        written,
    };
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::plan::Action;
    use crate::types::tree::ContentHash;

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    /// Stages and swaps `plan` on `workspace`, then writes the lock.
    fn run(plan: &Plan, workspace: &FakeWorkspace) -> Fallible<Result<(), Diagnostics>> {
        let staged = stage(plan, workspace).map_err(|r| format!("{:?}", shown(r)))?;
        let swapped = swap(plan, staged, workspace);
        Ok(write(plan, &swapped, workspace))
    }

    #[test]
    fn write_lock_writes_the_plans_lock_after_a_full_swap() -> TestResult {
        let (plan, workspace) = six()?;
        run(&plan, &workspace)?.map_err(|r| format!("{:?}", shown(r)))?;
        assert_eq!(workspace.lock_written(), Some(write_lock(plan.lock())));
        Ok(())
    }

    #[test]
    fn write_lock_after_a_failed_second_swap_holds_only_the_first() -> TestResult {
        let first = step(
            ".claude/a",
            Action::Update,
            Some("core"),
            Some(b"new a"),
            Some(file(b"old a")),
        )?;
        let second = step(
            ".claude/b",
            Action::Update,
            Some("core"),
            Some(b"new b"),
            Some(file(b"old b")),
        )?;
        let olds = vec![
            old(".claude/a", b"old a", "core")?,
            old(".claude/b", b"old b", "core")?,
        ];
        let plan = plan(vec![first, second], olds)?;
        let workspace = FakeWorkspace::new()
            .file(&path(".claude/a")?, b"old a")
            .file(&path(".claude/b")?, b"old b")
            .fail(WorkspaceCall::Swap(path(".claude/b")?));
        run(&plan, &workspace)?.map_err(|r| format!("{:?}", shown(r)))?;
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/a"),
            Some(ContentHash::of_bytes(b"new a"))
        );
        assert_eq!(
            hash_in(&lock, ".claude/b"),
            Some(ContentHash::of_bytes(b"old b"))
        );
        let revs: Vec<String> = lock
            .packs()
            .iter()
            .map(|pack| pack.version().to_string())
            .collect();
        assert_eq!(revs, [CORE_VERSION]);
        Ok(())
    }

    #[test]
    fn write_lock_keeps_the_entry_of_a_delete_not_performed() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Swap(path(".claude/b")?));
        run(&plan, &workspace)?.map_err(|r| format!("{:?}", shown(r)))?;
        assert!(
            !workspace
                .calls()
                .iter()
                .any(|call| matches!(call, WorkspaceCall::Delete(_)))
        );
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/e"),
            Some(ContentHash::of_bytes(b"old e"))
        );
        Ok(())
    }

    #[test]
    fn write_lock_applies_forget_and_unchanged_after_a_failure() -> TestResult {
        let update = step(
            ".claude/a",
            Action::Update,
            Some("core"),
            Some(b"new a"),
            Some(file(b"old a")),
        )?;
        let forget = step(".claude/f", Action::Forget, Some("core"), None, None)?;
        let adopted = step(
            ".claude/u1",
            Action::Unchanged,
            Some("core"),
            Some(b"u1"),
            Some(file(b"u1")),
        )?;
        let locked = step(
            ".claude/u2",
            Action::Unchanged,
            Some("core"),
            Some(b"u2 edited"),
            Some(file(b"u2 edited")),
        )?;
        let olds = vec![
            old(".claude/a", b"old a", "core")?,
            old(".claude/f", b"old f", "core")?,
            old(".claude/u2", b"u2 written", "core")?,
        ];
        let plan = plan(vec![update, forget, adopted, locked], olds)?;
        let workspace = FakeWorkspace::new()
            .file(&path(".claude/a")?, b"old a")
            .file(&path(".claude/u1")?, b"u1")
            .file(&path(".claude/u2")?, b"u2 edited")
            .fail(WorkspaceCall::Swap(path(".claude/a")?));
        run(&plan, &workspace)?.map_err(|r| format!("{:?}", shown(r)))?;
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/a"),
            Some(ContentHash::of_bytes(b"old a"))
        );
        assert_eq!(hash_in(&lock, ".claude/f"), None);
        assert_eq!(
            hash_in(&lock, ".claude/u1"),
            Some(ContentHash::of_bytes(b"u1"))
        );
        assert_eq!(
            hash_in(&lock, ".claude/u2"),
            Some(ContentHash::of_bytes(b"u2 edited"))
        );
        Ok(())
    }

    #[test]
    fn write_lock_keeps_the_old_entry_of_a_restore_not_landed() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::Swap(path(".claude/a")?));
        run(&plan, &workspace)?.map_err(|r| format!("{:?}", shown(r)))?;
        let restore = WorkspaceCall::Swap(path(".claude/c")?);
        assert!(!workspace.calls().contains(&restore));
        let lock = written(&workspace)?;
        assert_eq!(
            hash_in(&lock, ".claude/c"),
            Some(ContentHash::of_bytes(b"old c"))
        );
        Ok(())
    }

    #[test]
    fn write_lock_reports_the_lock_io_error() -> TestResult {
        let (plan, workspace) = six()?;
        let workspace = workspace.fail(WorkspaceCall::WriteLock);
        let refusals = run(&plan, &workspace)?.err().ok_or("a refusal")?;
        assert_eq!(shown(refusals), [format!("agentstacks.lock: {}", denied())]);
        assert_eq!(
            workspace.file_bytes(&path(".claude/b")?),
            Some(b"new b".to_vec())
        );
        Ok(())
    }
}
