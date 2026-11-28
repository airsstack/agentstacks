//! The `agentstacks` binary: parses the command line, opens the project and hands the command its
//! ports and streams.
//!
//! It only dispatches. What each command does is in [`command`] and is tested there against the
//! library fakes, so this file carries no inline tests; `tests/` runs the real binary.

mod cli;
mod command;
mod exit;
mod prompt;
mod show;
mod wire;

use std::io::{self, IsTerminal as _};
use std::path::Path;
use std::process::ExitCode;

use agentstacks::types::project::GitFlags;
use clap::Parser as _;

use cli::{Cli, Command, Ext};
use command::Console;
use command::ext::{AddRequest, CheckRequest};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stdin = io::stdin();
    let mut console = Console {
        is_terminal: stdin.is_terminal(),
        input: stdin.lock(),
        out: io::stdout().lock(),
        err: io::stderr().lock(),
    };
    let ports = match wire::open(cli.dir.as_deref(), &mut console.err) {
        Ok(ports) => ports,
        Err(outcome) => return outcome.code(),
    };
    let outcome = match cli.command {
        Command::Init {
            tool,
            pack,
            yes,
            dry_run,
        } => command::init(
            &ports,
            &tool,
            &pack,
            if dry_run {
                command::InitMode::DryRun
            } else if yes {
                command::InitMode::Yes
            } else {
                command::InitMode::Ask
            },
            &mut console,
        ),
        Command::Plan { verbose, .. } => command::plan(
            &ports,
            if verbose {
                crate::show::PlanView::Full
            } else {
                crate::show::PlanView::Summary
            },
            &mut console,
        ),
        Command::Apply { yes } => command::apply(&ports, yes, &mut console),
        Command::Doctor => {
            command::doctor(&ports.workspace, &ports.git, &ports.binary, &mut console)
        }
        Command::Update { yes } => command::update(&ports, yes, &mut console),
        Command::Ext(Ext::Add {
            source,
            tag,
            rev,
            subdir,
            yes,
        }) => {
            let request = AddRequest {
                source: &source,
                flags: GitFlags {
                    tag: tag.as_deref(),
                    rev: rev.as_deref(),
                    subdir: subdir.as_deref(),
                },
                yes,
            };
            command::ext::add(&ports, request, &mut console)
        }
        Command::Ext(Ext::Test { extension, pack }) => {
            let request = CheckRequest {
                base: cli.dir.as_deref().unwrap_or_else(|| Path::new(".")),
                extension: &extension,
                packs: &pack,
            };
            command::ext::check(&ports, &wire::scratch(), request, &mut console)
        }
    };
    drop(console);
    outcome.code()
}
