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
use std::process::ExitCode;

use clap::Parser as _;

use cli::{Cli, Command};
use command::Console;

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
        Command::Init { tool, pack, yes } => command::init(&ports, &tool, &pack, yes, &mut console),
        Command::Plan => command::plan(&ports, &mut console),
        Command::Apply { yes } => command::apply(&ports, yes, &mut console),
        Command::Doctor => command::doctor(&ports.workspace, &ports.binary, &mut console),
        Command::Update { yes } => command::update(&ports, yes, &mut console),
    };
    drop(console);
    outcome.code()
}
