//! The show, ask and apply sequence `apply`, `init` and `update` share.
//!
//! # Responsibilities
//!
//! - [`confirm`] prints the plan, then either stops (nothing to change, a lock that cannot be
//!   read), applies (`--yes` or an answer of yes) or refuses to (no terminal, an answer of no).
//! - [`Confirmed`] says which of those happened, so a command can pick its exit code and its
//!   follow-up line.
//!
//! # Non-responsibilities
//!
//! Making the plan and the follow-up lines `init` and `update` add belong to the commands. What
//! counts as nothing to change belongs to [`show`](crate::show), and applying to the library.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::apply::apply;
use agentstacks::port::Workspace;
use agentstacks::types::plan::Plan;
use agentstacks::types::{Diagnostic, Location};

use super::Console;
use crate::prompt::{Answer, ask};
use crate::show::{PlanView, nothing_to_change, plan_text, written_and_deleted};

/// The lock's name in the project, the file a read failure is located at.
const LOCK_FILE: &str = "agentstacks.lock";

/// The line `apply` prints when the author declines.
pub(super) const NOTHING_APPLIED: &str = "nothing applied";

/// How the sequence ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirmed {
    /// The plan lists nothing to do and the lock already matches: nothing was asked or written.
    NothingToChange,
    /// The plan was applied.
    Applied {
        /// The files written, counted as the plan summary counts them.
        written: usize,
        /// The files deleted.
        deleted: usize,
    },
    /// The author answered no or nothing.
    Declined,
    /// There was no terminal to ask on and no `--yes`.
    NoTerminal,
    /// The lock could not be read or the apply failed; the diagnostics were printed.
    Failed,
}

/// Shows `plan` on stdout and applies it when the author agrees.
///
/// The order is: print the plan; read the lock and stop on `agentstacks.lock: <error>` if it cannot
/// be read; stop with `nothing to change` when the plan lists no step and the lock already holds
/// what the plan would write; apply when `yes`; otherwise ask, and apply on yes. When the author
/// declines, `declined` is the line printed to stderr. A refusal to apply is printed to stderr as
/// one diagnostic per line. The caller's `plan` is the one applied, so a
/// plan is never made twice.
pub(crate) fn confirm<W, I, O, E>(
    plan: &Plan,
    workspace: &W,
    yes: bool,
    declined: &str,
    console: &mut Console<I, O, E>,
) -> Confirmed
where
    W: Workspace,
    I: BufRead,
    O: Write,
    E: Write,
{
    console.out_line(plan_text(plan, PlanView::Summary));

    let lock = match workspace.lock_text() {
        Ok(lock) => lock,
        Err(error) => {
            console.err_line(Diagnostic::new(Location::file(LOCK_FILE), error));
            return Confirmed::Failed;
        }
    };
    if nothing_to_change(plan, lock.as_deref()) {
        console.out_line("nothing to change");
        return Confirmed::NothingToChange;
    }
    if yes {
        return apply_plan(plan, workspace, console);
    }
    match ask(console.is_terminal, &mut console.input, &mut console.err) {
        Answer::Yes => apply_plan(plan, workspace, console),
        Answer::No => {
            console.err_line(declined);
            Confirmed::Declined
        }
        Answer::NoTerminal => {
            console.err_line("stdin is not a terminal; pass --yes to apply");
            Confirmed::NoTerminal
        }
    }
}

/// Applies `plan` and reports the counts, or the refusals.
fn apply_plan<W, I, O, E>(plan: &Plan, workspace: &W, console: &mut Console<I, O, E>) -> Confirmed
where
    W: Workspace,
    I: BufRead,
    O: Write,
    E: Write,
{
    match apply(plan, workspace) {
        Ok(()) => {
            let (written, deleted) = written_and_deleted(plan);
            console.out_line(format!("applied: {written} written, {deleted} deleted"));
            if written + deleted > 0 {
                console.out_line(
                    "note: start a new session — hooks, skills and agents load at session start",
                );
            }
            Confirmed::Applied { written, deleted }
        }
        Err(refusals) => {
            console.refuse(refusals);
            Confirmed::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{FakeWorkspace, WorkspaceCall};
    use agentstacks::types::lock::write_lock;
    use agentstacks::types::op::OpPath;
    use agentstacks::types::plan::Action;

    use super::*;
    use crate::command::testing::{
        TestResult, console, create_step, err_of, out_of, plan_of, step_of,
    };

    const PATH: &str = ".claude/a";

    fn untouched(console: &crate::command::testing::TestConsole) -> bool {
        console.input.position() == 0
    }

    #[test]
    fn nothing_to_change_asks_nothing() -> TestResult {
        let plan = plan_of(vec![step_of("x", Action::Unchanged)?])?;
        let workspace = FakeWorkspace::new().lock(&write_lock(plan.lock()));
        let mut console = console(true, "y\n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::NothingToChange);
        assert!(untouched(&console));
        assert!(out_of(&console).ends_with("nothing to change\n"));
        assert_eq!(err_of(&console), "");
        assert_eq!(workspace.lock_written(), None);
        Ok(())
    }

    #[test]
    fn yes_flag_applies_without_asking() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new();
        let mut console = console(false, "");
        let got = confirm(&plan, &workspace, true, NOTHING_APPLIED, &mut console);
        assert_eq!(
            got,
            Confirmed::Applied {
                written: 1,
                deleted: 0
            }
        );
        assert_eq!(err_of(&console), "");
        assert_eq!(
            workspace.file_bytes(&OpPath::new(PATH)?),
            Some(b"new".to_vec())
        );
        Ok(())
    }

    #[test]
    fn no_terminal_without_yes_refuses() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new();
        let mut console = console(false, "y\n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::NoTerminal);
        assert_eq!(
            err_of(&console),
            "stdin is not a terminal; pass --yes to apply\n"
        );
        assert!(untouched(&console));
        assert_eq!(workspace.file_bytes(&OpPath::new(PATH)?), None);
        Ok(())
    }

    #[test]
    fn declined_prints_nothing_applied() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new();
        let mut console = console(true, "n\n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::Declined);
        assert_eq!(err_of(&console), "Apply? [y/N] nothing applied\n");
        assert_eq!(workspace.file_bytes(&OpPath::new(PATH)?), None);
        assert_eq!(workspace.lock_written(), None);
        Ok(())
    }

    #[test]
    fn accepted_applies_and_prints_hint() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new();
        let mut console = console(true, " Yes \n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(
            got,
            Confirmed::Applied {
                written: 1,
                deleted: 0
            }
        );
        assert!(out_of(&console).ends_with("applied: 1 written, 0 deleted\nnote: start a new session — hooks, skills and agents load at session start\n"));
        assert_eq!(err_of(&console), "Apply? [y/N] ");
        assert_eq!(workspace.lock_written(), Some(write_lock(plan.lock())));
        Ok(())
    }

    #[test]
    fn apply_failure_prints_diagnostics() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new();
        workspace.set_file(&OpPath::new(PATH)?, b"someone else");
        let mut console = console(false, "");
        let got = confirm(&plan, &workspace, true, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::Failed);
        assert_eq!(
            err_of(&console),
            format!("{PATH}: changed after plan; run plan again\n")
        );
        assert!(!out_of(&console).contains("applied:"));
        Ok(())
    }

    #[test]
    fn zero_write_apply_prints_no_hint() -> TestResult {
        let plan = plan_of(vec![step_of("x", Action::Unchanged)?])?;
        let workspace = FakeWorkspace::new().lock("# a different lock\n");
        let mut console = console(true, "y\n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(
            got,
            Confirmed::Applied {
                written: 0,
                deleted: 0
            }
        );
        assert!(out_of(&console).ends_with("applied: 0 written, 0 deleted\n"));
        assert!(!out_of(&console).contains("start a new session"));
        Ok(())
    }

    #[test]
    fn lock_only_change_still_asks() -> TestResult {
        let plan = plan_of(vec![step_of("x", Action::Unchanged)?])?;
        let workspace = FakeWorkspace::new().lock("# a different lock\n");
        let mut console = console(true, "n\n");
        let got = confirm(&plan, &workspace, false, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::Declined);
        assert!(!untouched(&console));
        Ok(())
    }

    #[test]
    fn lock_read_error_refuses_without_asking() -> TestResult {
        let plan = plan_of(vec![create_step(PATH)?])?;
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::LockText);
        let mut console = console(true, "y\n");
        let got = confirm(&plan, &workspace, true, NOTHING_APPLIED, &mut console);
        assert_eq!(got, Confirmed::Failed);
        assert_eq!(
            err_of(&console),
            format!("agentstacks.lock: {}\n", FakeWorkspace::denied())
        );
        assert!(untouched(&console));
        assert_eq!(workspace.file_bytes(&OpPath::new(PATH)?), None);
        Ok(())
    }
}
