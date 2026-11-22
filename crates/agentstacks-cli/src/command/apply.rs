//! The `apply` command.
//!
//! # Responsibilities
//!
//! - [`run`] makes the plan and runs the [`confirm`](super::confirm) sequence on it, then maps how
//!   that ended to an exit outcome.
//!
//! # Non-responsibilities
//!
//! The sequence itself belongs to `confirm`, and applying to the library.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::port::{Adapter, Binary, Git, Workspace};

use super::confirm::{Confirmed, NOTHING_APPLIED, confirm};
use super::plan::make;
use super::{Console, Ports};
use crate::exit::Outcome;

/// Plans, shows, asks (unless `yes`) and applies.
///
/// A refused plan, a missing terminal and a failed apply or lock read are [`Outcome::Refused`].
/// Nothing to change, an applied plan and a declined question are [`Outcome::Success`].
pub(crate) fn run<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
    yes: bool,
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
        return Outcome::Refused;
    };
    match confirm(&plan, &ports.workspace, yes, NOTHING_APPLIED, console) {
        Confirmed::NothingToChange | Confirmed::Applied { .. } | Confirmed::Declined => {
            Outcome::Success
        }
        Confirmed::NoTerminal | Confirmed::Failed => Outcome::Refused,
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{FakeWorkspace, WorkspaceCall};
    use agentstacks::types::op::OpPath;

    use super::*;
    use crate::command::testing::{
        AGENT_FILE, Shifting, TestResult, World, console, err_of, out_of,
    };

    #[test]
    fn apply_declined_is_success() -> TestResult {
        let world = World::paths()?;
        let mut console = console(true, "n\n");
        let got = run(&world.ports, false, &mut console);
        assert_eq!(got, Outcome::Success);
        assert_eq!(err_of(&console), "Apply? [y/N] nothing applied\n");
        let workspace = &world.ports.workspace;
        assert_eq!(workspace.file_bytes(&OpPath::new(AGENT_FILE)?), None);
        Ok(())
    }

    #[test]
    fn apply_no_terminal_is_refused() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = run(&world.ports, false, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            "stdin is not a terminal; pass --yes to apply\n"
        );
        Ok(())
    }

    #[test]
    fn apply_with_yes_writes_the_file_and_the_lock() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Success);
        assert!(out_of(&console).ends_with("applied: 1 written, 0 deleted\n"));
        let workspace = &world.ports.workspace;
        assert_eq!(
            workspace.file_bytes(&OpPath::new(AGENT_FILE)?),
            Some(b"agent".to_vec())
        );
        assert!(workspace.lock_written().is_some());
        Ok(())
    }

    #[test]
    fn apply_with_a_refused_plan_is_refused() -> TestResult {
        let world = World::paths()?;
        world
            .ports
            .workspace
            .set_file(&OpPath::new(AGENT_FILE)?, b"mine");
        let mut console = console(false, "");
        // The file exists and no lock owns it, so the plan itself is refused.
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert!(
            err_of(&console).contains(AGENT_FILE),
            "{}",
            err_of(&console)
        );
        Ok(())
    }

    #[test]
    fn apply_failed_apply_is_refused() -> TestResult {
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
        let got = run(&ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert!(
            !out_of(&console).contains("applied:"),
            "{}",
            out_of(&console)
        );
        assert!(
            err_of(&console).contains(AGENT_FILE),
            "{}",
            err_of(&console)
        );
        Ok(())
    }

    #[test]
    fn apply_lock_read_error_is_refused() -> TestResult {
        let world = World::paths()?;
        let ports = Ports {
            workspace: Shifting::new(world.ports.workspace, None, true),
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let mut console = console(false, "");
        let got = run(&ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("agentstacks.lock: {}\n", FakeWorkspace::denied())
        );
        Ok(())
    }
}
