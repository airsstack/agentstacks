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

/// The commands.
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
        #[arg(long, conflicts_with = "dry_run")]
        yes: bool,
        /// Show the rendered agentstacks.toml and the plan, and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show what apply would do, and write nothing.
    Plan {
        /// Show counts per owner (the default).
        #[arg(long, conflicts_with = "verbose")]
        summary: bool,
        /// Show every file.
        #[arg(long)]
        verbose: bool,
    },
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
    /// Work with tool extensions.
    #[command(subcommand)]
    Ext(Ext),
}

/// The subcommands of `ext`.
#[derive(Debug, Subcommand)]
pub(crate) enum Ext {
    /// Add a tool extension to agentstacks.toml, then plan and apply.
    Add {
        /// The extension: a git URL, a directory in the project, or an official tool name.
        #[arg(value_name = "SOURCE")]
        source: String,
        /// The git tag to pin; a git URL needs this or --rev.
        #[arg(long, conflicts_with = "rev", value_name = "TAG")]
        tag: Option<String>,
        /// The commit to pin; a git URL needs this or --tag.
        #[arg(long, value_name = "REV")]
        rev: Option<String>,
        /// The directory of the extension inside the repository.
        #[arg(long, value_name = "DIR")]
        subdir: Option<String>,
        /// Apply without asking.
        #[arg(long)]
        yes: bool,
    },
    /// Check an extension against the built-in conformance pack in a temporary project.
    Test {
        /// The extension directory, read from where it is and never written to.
        #[arg(value_name = "EXTENSION")]
        extension: PathBuf,
        /// A pack to install beside the conformance pack. Repeat for more.
        #[arg(long = "pack", value_name = "DIR")]
        pack: Vec<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::{CommandFactory as _, Parser as _};

    use super::{Cli, Command, Ext};

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
        let Command::Init {
            tool, pack, yes, ..
        } = cli.command
        else {
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
    fn init_dry_run_and_yes_conflict() -> TestResult {
        assert!(
            Cli::try_parse_from([
                "agentstacks",
                "init",
                "--tool",
                "claude",
                "--pack",
                "core",
                "--dry-run"
            ])
            .is_ok()
        );
        let Err(error) = Cli::try_parse_from([
            "agentstacks",
            "init",
            "--tool",
            "claude",
            "--pack",
            "core",
            "--dry-run",
            "--yes",
        ]) else {
            return Err("--dry-run with --yes must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        assert!(
            error
                .to_string()
                .contains("the argument '--dry-run' cannot be used with '--yes'"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn plan_summary_and_verbose_conflict() -> TestResult {
        assert!(Cli::try_parse_from(["agentstacks", "plan", "--summary"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "plan", "--verbose"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "plan"]).is_ok());
        let Err(error) = Cli::try_parse_from(["agentstacks", "plan", "--summary", "--verbose"])
        else {
            return Err("--summary with --verbose must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        Ok(())
    }

    #[test]
    fn yes_is_accepted_by_apply_and_update_only() {
        assert!(Cli::try_parse_from(["agentstacks", "apply", "--yes"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "update", "--yes"]).is_ok());
        assert!(Cli::try_parse_from(["agentstacks", "plan", "--yes"]).is_err());
        assert!(Cli::try_parse_from(["agentstacks", "doctor", "--yes"]).is_err());
    }

    /// What `ext add` parsed to: `(source, tag, rev, subdir, yes)` and the `-C` directory.
    type Added = (String, Option<String>, Option<String>, Option<String>, bool);

    fn parsed_add(argv: &[&str]) -> Result<(Option<String>, Added), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from(argv)?;
        let dir = cli.dir.map(|dir| dir.display().to_string());
        let Command::Ext(Ext::Add {
            source,
            tag,
            rev,
            subdir,
            yes,
        }) = cli.command
        else {
            return Err("expected ext add".into());
        };
        Ok((dir, (source, tag, rev, subdir, yes)))
    }

    #[test]
    fn ext_add_parses() -> TestResult {
        let (dir, added) = parsed_add(&[
            "agentstacks",
            "ext",
            "add",
            "https://example.com/x.git",
            "--tag",
            "v1.2.0",
            "--subdir",
            "ext/mytool",
        ])?;
        assert_eq!(dir, None);
        assert_eq!(
            added,
            (
                "https://example.com/x.git".to_owned(),
                Some("v1.2.0".to_owned()),
                None,
                Some("ext/mytool".to_owned()),
                false
            )
        );

        let (dir, added) = parsed_add(&[
            "agentstacks",
            "-C",
            "proj",
            "ext",
            "add",
            "./ext/mytool",
            "--yes",
        ])?;
        assert_eq!(dir.as_deref(), Some("proj"));
        assert_eq!(added, ("./ext/mytool".to_owned(), None, None, None, true));

        let (dir, added) = parsed_add(&["agentstacks", "ext", "add", "x", "-C", "proj"])?;
        assert_eq!(dir.as_deref(), Some("proj"));
        assert_eq!(added, ("x".to_owned(), None, None, None, false));

        let (_, added) = parsed_add(&["agentstacks", "ext", "add", "u", "--rev", "abc"])?;
        assert_eq!(added.2.as_deref(), Some("abc"));

        let Err(error) =
            Cli::try_parse_from(["agentstacks", "ext", "add", "x", "--tag", "a", "--rev", "b"])
        else {
            return Err("--tag with --rev must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        assert!(
            error
                .to_string()
                .contains("the argument '--tag <TAG>' cannot be used with '--rev <REV>'"),
            "{error}"
        );

        let Err(error) = Cli::try_parse_from(["agentstacks", "ext"]) else {
            return Err("ext alone must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        let Err(error) = Cli::try_parse_from(["agentstacks", "ext", "add"]) else {
            return Err("ext add without a source must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        Ok(())
    }

    /// What `ext test` parsed to: the `-C` directory, the extension and the packs.
    type Tested = (Option<String>, String, Vec<String>);

    fn parsed_test(argv: &[&str]) -> Result<Tested, Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from(argv)?;
        let dir = cli.dir.map(|dir| dir.display().to_string());
        let Command::Ext(Ext::Test { extension, pack }) = cli.command else {
            return Err("expected ext test".into());
        };
        let packs = pack.iter().map(|pack| pack.display().to_string()).collect();
        Ok((dir, extension.display().to_string(), packs))
    }

    #[test]
    fn ext_test_parses() -> TestResult {
        let (dir, extension, packs) = parsed_test(&["agentstacks", "ext", "test", "ext/mytool"])?;
        assert_eq!(
            (dir, extension.as_str(), packs),
            (None, "ext/mytool", vec![])
        );

        let (dir, extension, packs) = parsed_test(&[
            "agentstacks",
            "-C",
            "proj",
            "ext",
            "test",
            "ext/mytool",
            "--pack",
            "packs/a",
            "--pack",
            "packs/b",
        ])?;
        assert_eq!(dir.as_deref(), Some("proj"));
        assert_eq!(extension, "ext/mytool");
        assert_eq!(packs, ["packs/a", "packs/b"]);

        let (dir, extension, packs) =
            parsed_test(&["agentstacks", "ext", "-C", "proj", "test", "x"])?;
        assert_eq!((dir.as_deref(), extension.as_str()), (Some("proj"), "x"));
        assert!(packs.is_empty());

        let (dir, extension, packs) = parsed_test(&[
            "agentstacks",
            "ext",
            "test",
            "x",
            "--pack",
            "p",
            "-C",
            "proj",
        ])?;
        assert_eq!((dir.as_deref(), extension.as_str()), (Some("proj"), "x"));
        assert_eq!(packs, ["p"]);

        let (dir, _, _) = parsed_test(&["agentstacks", "ext", "test", "-C", "proj", "x"])?;
        assert_eq!(dir.as_deref(), Some("proj"));

        let Err(error) = Cli::try_parse_from(["agentstacks", "ext", "test"]) else {
            return Err("ext test without an extension must fail".into());
        };
        assert_eq!(error.exit_code(), 2);
        Ok(())
    }
}
