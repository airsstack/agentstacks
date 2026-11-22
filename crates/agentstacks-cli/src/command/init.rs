//! The `init` command.
//!
//! # Responsibilities
//!
//! - [`run`] writes `agentstacks.toml` from the arguments, then plans, shows and applies it, with
//!   the lines that tell the author where the file stands when the plan is refused, declined or
//!   fails.
//!
//! # Non-responsibilities
//!
//! Classifying the arguments, resolving them and rendering the file belong to the library, and the
//! plan and apply sequence to [`followup`](super::followup).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::init::init;
use agentstacks::port::{Adapter, Binary, Git, Workspace};

use super::followup::{Rewrite, plan_then_apply};
use super::{Console, Ports};
use crate::exit::Outcome;

/// Writes `agentstacks.toml` for `tools` and `packs`, then plans and applies it.
///
/// A refusal of the arguments prints the diagnostics, writes nothing and is
/// [`Outcome::Refused`].
pub(crate) fn run<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
    tools: &[String],
    packs: &[String],
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
    if let Err(refusals) = init(&ports.workspace, &ports.git, tools, packs) {
        console.refuse(refusals);
        return Outcome::Refused;
    }
    plan_then_apply(ports, yes, Rewrite::Written, console)
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

    const FIX: &str = "agentstacks.toml written; fix the above and run apply\n";
    const KEPT: &str = "agentstacks.toml written; nothing applied (run apply)\n";

    fn args() -> (Vec<String>, Vec<String>) {
        (
            vec!["./extensions/claude".to_owned()],
            vec!["./packs/core".to_owned()],
        )
    }

    #[test]
    fn init_with_yes_applies() -> TestResult {
        let world = World::empty()?;
        let (tools, packs) = args();
        let mut console = console(false, "");
        let got = run(&world.ports, &tools, &packs, true, &mut console);
        assert_eq!(got, Outcome::Success, "{}", err_of(&console));
        assert_eq!(err_of(&console), "");
        assert!(out_of(&console).ends_with("applied: 1 written, 0 deleted\n"));
        let workspace = &world.ports.workspace;
        let written = workspace.project_written().ok_or("a project file")?;
        assert!(written.contains("path = \"./packs/core\""), "{written}");
        assert_eq!(
            workspace.file_bytes(&OpPath::new(AGENT_FILE)?),
            Some(b"agent".to_vec())
        );
        Ok(())
    }

    #[test]
    fn init_existing_file_is_refused_with_nothing_written() -> TestResult {
        let world = World::paths()?;
        let (tools, packs) = args();
        let mut console = console(false, "");
        let got = run(&world.ports, &tools, &packs, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            "agentstacks.toml: already exists (edit it and run apply)\n"
        );
        assert_eq!(world.ports.workspace.project_written(), None);
        assert_eq!(out_of(&console), "");
        Ok(())
    }

    #[test]
    fn init_refused_plan_keeps_the_toml() -> TestResult {
        let mut world = World::empty()?;
        let workspace = std::mem::take(&mut world.ports.workspace).fail(WorkspaceCall::LockText);
        let ports = Ports {
            workspace,
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let (tools, packs) = args();
        let mut console = console(false, "");
        let got = run(&ports, &tools, &packs, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("agentstacks.lock: {}\n{FIX}", FakeWorkspace::denied())
        );
        assert!(ports.workspace.project_written().is_some());
        Ok(())
    }

    #[test]
    fn init_lock_read_error_adds_the_fix_line() -> TestResult {
        let world = World::empty()?;
        let ports = Ports {
            workspace: Shifting::new(world.ports.workspace, None, true),
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let (tools, packs) = args();
        let mut console = console(true, "y\n");
        let got = run(&ports, &tools, &packs, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("agentstacks.lock: {}\n{FIX}", FakeWorkspace::denied())
        );
        Ok(())
    }

    #[test]
    fn init_declined_is_success_with_the_follow_up() -> TestResult {
        let world = World::empty()?;
        let (tools, packs) = args();
        let mut console = console(true, "n\n");
        let got = run(&world.ports, &tools, &packs, false, &mut console);
        assert_eq!(got, Outcome::Success);
        assert_eq!(err_of(&console), format!("Apply? [y/N] {KEPT}"));
        assert!(world.ports.workspace.project_written().is_some());
        assert_eq!(
            world.ports.workspace.file_bytes(&OpPath::new(AGENT_FILE)?),
            None
        );
        Ok(())
    }

    #[test]
    fn init_no_terminal_prints_both_lines_and_is_refused() -> TestResult {
        let world = World::empty()?;
        let (tools, packs) = args();
        let mut console = console(false, "");
        let got = run(&world.ports, &tools, &packs, false, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("stdin is not a terminal; pass --yes to apply\n{KEPT}")
        );
        assert!(world.ports.workspace.project_written().is_some());
        Ok(())
    }
}
