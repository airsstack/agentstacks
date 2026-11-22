//! The `update` command.
//!
//! # Responsibilities
//!
//! - [`run`] lists the newer tags, writes them into `agentstacks.toml` and then plans, shows and
//!   applies like `apply`, with the lines that tell the author where the file stands.
//! - It writes nothing, and plans nothing, when no tag is newer.
//!
//! # Non-responsibilities
//!
//! Finding the newer tags and editing the file belong to the library, and the plan and apply
//! sequence to [`followup`](super::followup).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::update::{retag, update};
use agentstacks::port::{Adapter, Binary, Git, Workspace};

use super::followup::{Rewrite, plan_then_apply};
use super::{Console, Ports};
use crate::exit::Outcome;
use crate::show::{has_newer, update_text};

/// Moves each git source to its newest tag, then plans and applies.
///
/// A refusal to list the tags, or to write them (the file changed meanwhile, or cannot be
/// written), prints the diagnostics and is [`Outcome::Refused`] with the file as it was and no
/// plan.
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
    let entries = match update(&ports.workspace, &ports.git) {
        Ok(entries) => entries,
        Err(refusals) => {
            console.refuse(refusals);
            return Outcome::Refused;
        }
    };
    if !entries.is_empty() {
        console.out_line(update_text(&entries));
    }
    if !has_newer(&entries) {
        console.out_line("nothing to update");
        return Outcome::Success;
    }
    if let Err(refusals) = retag(&ports.workspace, &entries) {
        console.refuse(refusals);
        return Outcome::Refused;
    }
    console.out_line("agentstacks.toml updated");
    plan_then_apply(ports, yes, Rewrite::Updated, console)
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{FakeWorkspace, WorkspaceCall};
    use agentstacks::types::op::OpPath;

    use super::*;
    use crate::command::Ports;
    use crate::command::testing::{
        AGENT_FILE, GIT_PROJECT, Shifting, TestResult, World, console, err_of, out_of,
    };

    const FIX: &str = "agentstacks.toml updated; fix the above and run apply\n";
    const KEPT: &str = "agentstacks.toml updated; nothing applied (run apply)\n";
    const LISTED: &str = "  packs.core: packs-v0.1.0 \u{2192} packs-v0.2.0\n";

    #[test]
    fn update_with_only_path_sources_prints_nothing_to_update() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Success);
        assert_eq!(out_of(&console), "nothing to update\n");
        assert_eq!(err_of(&console), "");
        assert_eq!(world.ports.workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn update_writes_tags_then_applies_with_yes() -> TestResult {
        let world = World::git()?;
        let mut console = console(false, "");
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Success, "{}", err_of(&console));
        let out = out_of(&console);
        assert!(
            out.starts_with(&format!(
                "{LISTED}agentstacks.toml updated\nagentstacks plan\n"
            )),
            "{out}"
        );
        assert!(out.ends_with("applied: 1 written, 0 deleted\n"), "{out}");
        assert_eq!(err_of(&console), "");
        let written = world.ports.workspace.project_written().ok_or("a project")?;
        assert_eq!(written, GIT_PROJECT.replace("packs-v0.1.0", "packs-v0.2.0"));
        assert_eq!(
            world.ports.workspace.file_bytes(&OpPath::new(AGENT_FILE)?),
            Some(b"agent".to_vec())
        );
        Ok(())
    }

    #[test]
    fn update_declined_keeps_the_toml_and_succeeds() -> TestResult {
        let world = World::git()?;
        let mut console = console(true, "n\n");
        let got = run(&world.ports, false, &mut console);
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
    fn update_refused_plan_adds_the_fix_line() -> TestResult {
        let world = World::unserved()?;
        let mut console = console(false, "");
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        let err = err_of(&console);
        assert!(err.ends_with(FIX), "{err}");
        assert!(err.lines().count() >= 2, "{err}");
        assert!(world.ports.workspace.project_written().is_some());
        Ok(())
    }

    #[test]
    fn update_retag_refusal_is_refused_with_no_plan() -> TestResult {
        let world = World::git()?;
        let edited = GIT_PROJECT.replace("packs-v0.1.0", "packs-v0.1.5");
        let ports = Ports {
            workspace: Shifting::new(world.ports.workspace, Some(&edited), false),
            git: world.ports.git,
            binary: world.ports.binary,
            adapter: world.ports.adapter,
        };
        let mut console = console(false, "");
        let got = run(&ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            "agentstacks.toml: packs.core: tag changed during update (run update again)\n"
        );
        assert_eq!(out_of(&console), LISTED);
        assert!(ports.adapter.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_failed_apply_adds_the_fix_line() -> TestResult {
        let mut world = World::git()?;
        let target = OpPath::new(AGENT_FILE)?;
        world.ports.workspace =
            std::mem::take(&mut world.ports.workspace).fail(WorkspaceCall::Stage(target));
        let mut console = console(false, "");
        let got = run(&world.ports, true, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("{AGENT_FILE}: {}\n{FIX}", FakeWorkspace::denied())
        );
        Ok(())
    }

    #[test]
    fn update_no_terminal_prints_both_lines_and_is_refused() -> TestResult {
        let world = World::git()?;
        let mut console = console(false, "");
        let got = run(&world.ports, false, &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            format!("stdin is not a terminal; pass --yes to apply\n{KEPT}")
        );
        Ok(())
    }
}
