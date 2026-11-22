//! The `doctor` command.
//!
//! # Responsibilities
//!
//! - [`run`] checks the project against `agentstacks.lock` and prints the report.
//!
//! # Non-responsibilities
//!
//! What counts as drift belongs to the library, and the layout of the report to
//! [`show`](crate::show).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::doctor::doctor;
use agentstacks::port::{Binary, Workspace};

use super::Console;
use crate::exit::Outcome;
use crate::show::doctor_text;

/// Prints every finding to stdout, or `healthy`. [`Outcome::Success`] only with no finding.
pub(crate) fn run<W, B, I, O, E>(
    workspace: &W,
    binary: &B,
    console: &mut Console<I, O, E>,
) -> Outcome
where
    W: Workspace,
    B: Binary,
    I: BufRead,
    O: Write,
    E: Write,
{
    let findings = doctor(workspace, binary);
    console.out_line(doctor_text(&findings));
    if findings.is_empty() {
        Outcome::Success
    } else {
        Outcome::Refused
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::types::op::OpPath;

    use super::*;
    use crate::command::apply;
    use crate::command::testing::{AGENT_FILE, TestResult, World, console, err_of, out_of};

    fn applied() -> Result<World, Box<dyn std::error::Error>> {
        let world = World::paths()?;
        let mut console = console(false, "");
        assert_eq!(apply(&world.ports, true, &mut console), Outcome::Success);
        Ok(world)
    }

    #[test]
    fn doctor_healthy_is_success() -> TestResult {
        let world = applied()?;
        let mut console = console(false, "");
        let got = run(&world.ports.workspace, &world.ports.binary, &mut console);
        assert_eq!(got, Outcome::Success, "{}", out_of(&console));
        assert_eq!(out_of(&console), "healthy\n");
        assert_eq!(err_of(&console), "");
        Ok(())
    }

    #[test]
    fn doctor_with_a_finding_is_refused() -> TestResult {
        let world = applied()?;
        world
            .ports
            .workspace
            .set_file(&OpPath::new(AGENT_FILE)?, b"edited");
        let mut console = console(false, "");
        let got = run(&world.ports.workspace, &world.ports.binary, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(out_of(&console), format!("changed: {AGENT_FILE}\n"));
        Ok(())
    }
}
