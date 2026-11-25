//! The `ext add` command.
//!
//! # Responsibilities
//!
//! - [`run`] adds a tool extension to `agentstacks.toml`, prints `agentstacks.toml: added
//!   tools.<name>` and then plans, shows and applies like `update`, with the lines that tell the
//!   author where the file stands.
//!
//! # Non-responsibilities
//!
//! Classifying the source, loading the extension and editing the file belong to the library, and
//! the plan and apply sequence to `followup`.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

use agentstacks::app::ext::add;
use agentstacks::port::{Adapter, Binary, Git, Workspace};
use agentstacks::types::project::GitFlags;

use crate::command::followup::{Rewrite, plan_then_apply};
use crate::command::{Console, Ports};
use crate::exit::Outcome;

/// What `ext add` was given: the source and its git options.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AddRequest<'a> {
    /// A git URL, a directory of the project or an official tool name.
    pub(crate) source: &'a str,
    /// The tag or commit a git URL is pinned to, and the directory of the extension inside it.
    pub(crate) flags: GitFlags<'a>,
    /// Whether to apply without asking.
    pub(crate) yes: bool,
}

/// Adds the extension `request` names to `agentstacks.toml`, then plans and applies.
///
/// A refusal of the source or of the file prints the diagnostics, writes nothing and is
/// [`Outcome::Refused`], with no plan. A refused plan keeps the new table and prints
/// `agentstacks.toml updated; fix the above and run apply`.
pub(crate) fn run<W, G, B, A, I, O, E>(
    ports: &Ports<W, G, B, A>,
    request: AddRequest<'_>,
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
    let AddRequest { source, flags, yes } = request;
    match add(&ports.workspace, &ports.git, source, flags) {
        Ok(name) => console.out_line(format!("agentstacks.toml: added tools.{name}")),
        Err(refusals) => {
            console.refuse(refusals);
            return Outcome::Refused;
        }
    }
    plan_then_apply(ports, yes, Rewrite::Updated, console)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::testing::{TestResult, World, console, err_of, out_of};

    const FIX: &str = "agentstacks.toml updated; fix the above and run apply\n";
    const KEPT: &str = "agentstacks.toml updated; nothing applied (run apply)\n";
    const NO_TERMINAL: &str = "stdin is not a terminal; pass --yes to apply\n";

    fn request(source: &str, yes: bool) -> AddRequest<'_> {
        AddRequest {
            source,
            flags: GitFlags::default(),
            yes,
        }
    }

    #[test]
    fn ext_add_with_yes_prints_the_table_then_applies() -> TestResult {
        let world = World::paths()?.with_tool("opencode")?;
        let mut console = console(false, "");
        let got = run(&world.ports, request("./ext/opencode", true), &mut console);
        assert_eq!(got, Outcome::Success, "{}", err_of(&console));
        let out = out_of(&console);
        assert!(
            out.starts_with("agentstacks.toml: added tools.opencode\nagentstacks plan\n"),
            "{out}"
        );
        assert!(out.ends_with("applied: 2 written, 0 deleted\n"), "{out}");
        assert_eq!(err_of(&console), "");
        let written = world.ports.workspace.project_written().ok_or("a project")?;
        assert!(written.contains("[tools.opencode]"), "{written}");
        Ok(())
    }

    #[test]
    fn ext_add_refused_plan_adds_the_fix_line() -> TestResult {
        let world = World::unserved()?.with_tool("opencode")?;
        let mut console = console(false, "");
        let got = run(&world.ports, request("./ext/opencode", true), &mut console);
        assert_eq!(got, Outcome::Refused);
        assert!(out_of(&console).starts_with("agentstacks.toml: added tools.opencode\n"));
        let err = err_of(&console);
        assert!(err.ends_with(FIX), "{err}");
        let written = world.ports.workspace.project_written().ok_or("a project")?;
        assert!(written.contains("[tools.opencode]"), "{written}");
        Ok(())
    }

    #[test]
    fn ext_add_no_terminal() -> TestResult {
        let world = World::paths()?.with_tool("opencode")?;
        let mut console = console(false, "");
        let got = run(&world.ports, request("./ext/opencode", false), &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(err_of(&console), format!("{NO_TERMINAL}{KEPT}"));
        assert!(world.ports.workspace.project_written().is_some());
        Ok(())
    }

    #[test]
    fn ext_add_refusal_prints_it_and_plans_nothing() -> TestResult {
        let world = World::paths()?;
        let mut console = console(false, "");
        let got = run(
            &world.ports,
            request("./extensions/claude", true),
            &mut console,
        );
        assert_eq!(got, Outcome::Refused);
        assert_eq!(
            err_of(&console),
            "agentstacks.toml: tools.claude already exists (edit it there)\n"
        );
        assert_eq!(out_of(&console), "");
        assert_eq!(world.ports.workspace.project_written(), None);
        Ok(())
    }
}
