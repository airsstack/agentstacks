//! The `agentstacks` binary: a thin entry point with no branching, so it carries no inline tests.

mod cli;

use clap::Parser as _;

fn main() {
    cli::Cli::parse();
}
