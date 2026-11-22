//! The `plan` command.
//!
//! # Responsibilities
//!
//! - [`run`] makes the plan, prints it and writes nothing.
//! - [`make`] is the plan step the other commands that show a plan start from: it makes the plan
//!   or prints its refusals.
//!
//! # Non-responsibilities
//!
//! The planning rules belong to the library, and asking and applying to [`confirm`](super::confirm).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::plan::plan;
use agentstacks::port::{Adapter, Binary, Git, Workspace};
use agentstacks::types::plan::Plan;

use super::{Console, Ports};
use crate::exit::Outcome;
use crate::show::plan_text;

/// Plans the project, or prints the sorted refusals to stderr and returns `None`.
pub(super) fn make<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
    console: &mut Console<I, O, E>,
) -> Option<Plan>
where
    W: Workspace,
    G: Git,
    B: Binary,
    A: Adapter,
    I: BufRead,
    O: Write,
    E: Write,
{
    match plan(&ports.workspace, &ports.git, &ports.binary, &ports.adapter) {
        Ok(plan) => Some(plan),
        Err(refusals) => {
            console.refuse(refusals);
            None
        }
    }
}

/// Prints the plan to stdout: [`Outcome::Success`], or [`Outcome::Refused`] with the refusals on
/// stderr. Nothing is written to the project.
pub(crate) fn run<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
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
    make(ports, console).map_or(Outcome::Refused, |plan| {
        console.out_line(plan_text(&plan));
        Outcome::Success
    })
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{FakeAdapter, FakeBinary, FakeGit, FakeWorkspace, WorkspaceCall};
    use agentstacks::types::op::OpPath;

    use super::*;
    use crate::command::Ports;
    use crate::command::testing::{AGENT_FILE, TestResult, World, console, err_of, out_of};

    #[test]
    fn plan_prints_and_writes_nothing() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = run(&world.ports, &mut console);
        assert_eq!(got, Outcome::Success);
        let out = out_of(&console);
        assert!(out.starts_with("agentstacks plan\n"), "{out}");
        assert!(out.contains(&format!("create   {AGENT_FILE}")), "{out}");
        assert_eq!(err_of(&console), "");
        let workspace = &world.ports.workspace;
        assert_eq!(workspace.file_bytes(&OpPath::new(AGENT_FILE)?), None);
        assert_eq!(workspace.lock_written(), None);
        assert!(!workspace.calls().iter().any(|call| matches!(
            call,
            WorkspaceCall::Stage(_) | WorkspaceCall::Swap(_) | WorkspaceCall::WriteLock
        )));
        Ok(())
    }

    #[test]
    fn plan_refusal_exits_refused() -> TestResult {
        let ports = Ports {
            workspace: FakeWorkspace::new(),
            git: FakeGit::new(),
            binary: FakeBinary::new(),
            adapter: FakeAdapter::new(),
        };
        let missing = ports.workspace.project_text().err().ok_or("a project")?;
        let mut console = console(false, "");
        let got = run(&ports, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(err_of(&console), format!("agentstacks.toml: {missing}\n"));
        assert_eq!(out_of(&console), "");
        Ok(())
    }
}
