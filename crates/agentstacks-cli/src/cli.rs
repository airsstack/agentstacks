//! The command-line grammar (clap lives only in this crate).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Sets up agentic coding tools from one tool-neutral copy of skills, agents, commands, hooks and
/// rules.
#[derive(Debug, Parser)]
#[command(name = "agentstacks", version, about, arg_required_else_help = true)]
pub(crate) struct Cli {
    /// The project directory; the current directory when absent. Parent directories are not
    /// searched.
    #[arg(short = 'C', global = true, value_name = "DIR")]
    pub(crate) dir: Option<PathBuf>,

    /// What to do.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The five commands.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Write agentstacks.toml for the given tools and packs, then plan and apply it.
    Init {
        /// A tool: an official name or a directory in the project. Repeat for more.
        #[arg(long, required = true, value_name = "TOOL")]
        tool: Vec<String>,
        /// A pack: an official name or a directory in the project. Repeat for more.
        #[arg(long, required = true, value_name = "PACK")]
        pack: Vec<String>,
        /// Apply without asking.
        #[arg(long)]
        yes: bool,
    },
    /// Show what apply would do, and write nothing.
    Plan,
    /// Show the plan, ask, then write the files and the lock.
    Apply {
        /// Apply without asking.
        #[arg(long)]
        yes: bool,
    },
    /// Report drift from agentstacks.lock.
    Doctor,
    /// Move each git source to its newest tag, then plan and apply.
    Update {
        /// Apply without asking.
        #[arg(long)]
        yes: bool,
    },
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::{CommandFactory as _, Parser as _};

    use super::{Cli, Command};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn init_without_pack_is_a_usage_error() -> TestResult {
        let Err(error) = Cli::try_parse_from(["agentstacks", "init", "--tool", "claude"]) else {
            return Err("init without --pack must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains("--pack <PACK>"), "{error}");
        Ok(())
    }

    #[test]
    fn init_collects_repeated_flags() -> TestResult {
        let cli = Cli::try_parse_from([
            "agentstacks",
            "init",
            "--tool",
            "claude",
            "--pack",
            "core",
            "--pack",
            "./packs/team",
            "--yes",
        ])?;
        let Command::Init { tool, pack, yes } = cli.command else {
            return Err("expected init".into());
        };
        assert_eq!(tool, ["claude"]);
        assert_eq!(pack, ["core", "./packs/team"]);
        assert!(yes);
        Ok(())
    }

    #[test]
    fn dash_c_works_before_and_after_the_subcommand() -> TestResult {
        let before = Cli::try_parse_from(["agentstacks", "-C", "proj", "plan"])?;
        let after = Cli::try_parse_from(["agentstacks", "plan", "-C", "proj"])?;
        assert_eq!(before.dir.as_deref(), Some(Path::new("proj")));
        assert_eq!(after.dir.as_deref(), Some(Path::new("proj")));
        let none = Cli::try_parse_from(["agentstacks", "doctor"])?;
        assert_eq!(none.dir, None);
        Ok(())
    }

    #[test]
    fn no_subcommand_is_an_error() -> TestResult {
        let Err(error) = Cli::try_parse_from(["agentstacks"]) else {
            return Err("no subcommand must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        Ok(())
    }

    #[test]
    fn unknown_subcommand_is_an_error() -> TestResult {
        let Err(error) = Cli::try_parse_from(["agentstacks", "frobnicate"]) else {
            return Err("an unknown subcommand must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        assert!(
            error
                .to_string()
                .contains("unrecognized subcommand 'frobnicate'"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn yes_is_accepted_by_apply_and_update_only() {
        assert!(Cli::try_parse_from(["agentstacks", "apply", "--yes"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "update", "--yes"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "plan", "--yes"]).is_err());
        assert!(Cli::try_parse_from(["agentstacks", "doctor", "--yes"]).is_err());
    }
}
