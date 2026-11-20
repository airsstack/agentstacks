//! The command-line grammar (clap lives only in this crate).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use clap::Parser;

/// Sets up agentic coding tools from one tool-neutral copy of skills, agents, commands, hooks and
/// rules.
#[derive(Debug, Parser)]
#[command(name = "agentstacks", version, about)]
pub(crate) struct Cli;

#[cfg(test)]
mod tests {
    use clap::{CommandFactory as _, Parser as _};

    use super::Cli;

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["agentstacks", "init"]).is_err());
    }
}
