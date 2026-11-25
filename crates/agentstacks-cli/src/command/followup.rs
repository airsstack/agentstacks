//! The plan, show and apply that follow a rewrite of `agentstacks.toml`.
//!
//! # Responsibilities
//!
//! - [`plan_then_apply`] is what `init`, `update` and `ext add` do once the file is rewritten: the
//!   same plan, show and apply as `apply`, then the line on stderr that tells the author where the
//!   file stands, and the exit outcome.
//! - [`Rewrite`] says which of the two the file went through, for the wording of those lines.
//!
//! # Non-responsibilities
//!
//! The rewrite itself and the sequence it runs belong to the commands and
//! [`confirm`](super::confirm).

use std::fmt::{self, Display};
use std::io::{BufRead, Write};

use agentstacks::port::{Adapter, Binary, Git, Workspace};

use super::confirm::{Confirmed, confirm};
use super::plan::make;
use super::{Console, Ports};
use crate::exit::Outcome;

/// What was done to `agentstacks.toml` before the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rewrite {
    /// `init` wrote it.
    Written,
    /// `update` moved its tags, or `ext add` added a tool.
    Updated,
}

impl Display for Rewrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Written => "written",
            Self::Updated => "updated",
        })
    }
}

impl Rewrite {
    /// The line for a file that stays as it is while the apply did not happen.
    fn kept(self) -> String {
        format!("agentstacks.toml {self}; nothing applied (run apply)")
    }

    /// The line for a plan or an apply that failed.
    fn fix(self) -> String {
        format!("agentstacks.toml {self}; fix the above and run apply")
    }
}

/// Plans, shows, asks (unless `yes`) and applies, then adds the line for the file's state.
///
/// A refused plan and a failed apply or lock read print the diagnostics and then `agentstacks.toml
/// <written|updated>; fix the above and run apply`, and exit [`Outcome::Refused`]. A declined
/// question prints `agentstacks.toml <written|updated>; nothing applied (run apply)` and exits
/// [`Outcome::Success`]. A missing terminal prints its line, then that one, and exits
/// [`Outcome::Refused`]. An applied plan and nothing to change print nothing more.
pub(super) fn plan_then_apply<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
    yes: bool,
    rewrite: Rewrite,
    console: &mut Console<I, O, E>,
) -> Outcome
where
    W: Workspace,
    G: Git,
    B: Binary,
    A: Adapter,
    I: BufRead,
    O: Write,
    E: Write,
{
    let Some(plan) = make(ports, console) else {
        console.err_line(rewrite.fix());
        return Outcome::Refused;
    };
    match confirm(&plan, &ports.workspace, yes, &rewrite.kept(), console) {
        Confirmed::NothingToChange | Confirmed::Applied { .. } | Confirmed::Declined => {
            Outcome::Success
        }
        Confirmed::NoTerminal => {
            console.err_line(rewrite.kept());
            Outcome::Refused
        }
        Confirmed::Failed => {
            console.err_line(rewrite.fix());
            Outcome::Refused
        }
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{FakeWorkspace, WorkspaceCall};
    use agentstacks::types::op::OpPath;

    use super::*;
    use crate::command::Ports;
    use crate::command::testing::{
        AGENT_FILE, Shifting, TestResult, World, console, err_of, out_of,
    };

    const KEPT_WRITTEN: &str = "agentstacks.toml written; nothing applied (run apply)";
    const KEPT_UPDATED: &str = "agentstacks.toml updated; nothing applied (run apply)";
    const FIX_WRITTEN: &str = "agentstacks.toml written; fix the above and run apply";
    const FIX_UPDATED: &str = "agentstacks.toml updated; fix the above and run apply";

    #[test]
    fn rewrite_display_is_the_past_tense_word() {
        assert_eq!(Rewrite::Written.to_string(), "written");
        assert_eq!(Rewrite::Updated.to_string(), "updated");
    }

    #[test]
    fn rewrite_kept_line_names_the_rewrite() {
        assert_eq!(Rewrite::Written.kept(), KEPT_WRITTEN);
        assert_eq!(Rewrite::Updated.kept(), KEPT_UPDATED);
    }

    #[test]
    fn rewrite_fix_line_names_the_rewrite() {
        assert_eq!(Rewrite::Written.fix(), FIX_WRITTEN);
        assert_eq!(Rewrite::Updated.fix(), FIX_UPDATED);
    }

    #[test]
    fn applied_is_success_with_no_follow_up() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = plan_then_apply(&world.ports, true, Rewrite::Updated, &mut console);
        assert_eq!(got, Outcome::Success);
        assert_eq!(err_of(&console), "");
        assert!(out_of(&console).ends_with("applied: 1 written, 0 deleted\n"));
        Ok(())
    }

    #[test]
    fn nothing_to_change_is_success_with_no_follow_up() -> TestResult {
        let first_world = World::paths()?;
        let mut first = console(false, "");
        assert_eq!(
            plan_then_apply(&first_world.ports, true, Rewrite::Written, &mut first),
            Outcome::Success
        );
        // A second project that already holds the file and the lock the first apply wrote.
        let lock = first_world.ports.workspace.lock_written().ok_or("a lock")?;
        let mut world = World::paths()?;
        world.ports.workspace = std::mem::take(&mut world.ports.workspace)
            .file(&OpPath::new(AGENT_FILE)?, b"agent")
            .lock(&lock);
        let mut console = console(false, "");
        let got = plan_then_apply(&world.ports, false, Rewrite::Written, &mut console);
        assert_eq!(got, Outcome::Success, "{}", err_of(&console));
        assert_eq!(err_of(&console), "");
        assert!(out_of(&console).ends_with("nothing to change\n"));
        Ok(())
    }

    #[test]
    fn declined_is_success_with_the_kept_line() -> TestResult {
        let world = World::paths()?;
        let mut console = console(true, "n\n");
        let got = plan_then_apply(&world.ports, false, Rewrite::Written, &mut console);
        assert_eq!(got, Outcome::Success);
        assert_eq!(err_of(&console), format!("Apply? [y/N] {KEPT_WRITTEN}\n"));
        Ok(())
    }

    #[test]
    fn no_terminal_is_refused_with_both_lines() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = plan_then_apply(&world.ports, false, Rewrite::Updated, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("stdin is not a terminal; pass --yes to apply\n{KEPT_UPDATED}\n")
        );
        Ok(())
    }

    #[test]
    fn refused_plan_is_refused_with_the_fix_line() -> TestResult {
        let world = World::unserved()?;
        let mut console = console(false, "");
        let got = plan_then_apply(&world.ports, true, Rewrite::Updated, &mut console);
        assert_eq!(got, Outcome::Refused);
        let err = err_of(&console);
        assert!(err.ends_with(&format!("{FIX_UPDATED}\n")), "{err}");
        Ok(())
    }

    #[test]
    fn failed_lock_read_is_refused_with_the_fix_line() -> TestResult {
        let world = World::paths()?;
        let ports = Ports {
            workspace: Shifting::new(world.ports.workspace, None, true),
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let mut console = console(false, "");
        let got = plan_then_apply(&ports, true, Rewrite::Written, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!(
                "agentstacks.lock: {}\n{FIX_WRITTEN}\n",
                FakeWorkspace::denied()
            )
        );
        Ok(())
    }

    #[test]
    fn failed_apply_is_refused_with_the_fix_line() -> TestResult {
        let mut world = World::paths()?;
        let workspace = std::mem::take(&mut world.ports.workspace)
            .fail(WorkspaceCall::Stage(OpPath::new(AGENT_FILE)?));
        let ports = Ports {
            workspace,
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let mut console = console(false, "");
        let got = plan_then_apply(&ports, true, Rewrite::Written, &mut console);
        assert_eq!(got, Outcome::Refused);
        let err = err_of(&console);
        assert!(err.ends_with(&format!("{FIX_WRITTEN}\n")), "{err}");
        assert!(!out_of(&console).contains("applied:"));
        Ok(())
    }
}
