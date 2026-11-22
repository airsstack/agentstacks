//! One `git` command, run the way every source needs it.
//!
//! # Responsibilities
//!
//! - [`GitRun`] describes one command: the program, an optional repository, the subcommand, its
//!   arguments, the operands that follow `--` and an optional standard input.
//! - [`GitRun::argv`] builds the argument vector: `-c core.autocrlf=false` first, then
//!   `-C <repo>`, the subcommand and its arguments, then `-- <operands>`. A URL is always an
//!   operand, so it is never read as an option.
//! - [`GitRun::run`] starts it with `GIT_TERMINAL_PROMPT=0`, so git never waits for a password,
//!   and without the variables that name a repository (`GIT_DIR` and its kin), so the command only
//!   touches the repository `-C` names. It answers its standard output or the [`FetchError`] for
//!   the failure.
//!
//! # Non-responsibilities
//!
//! What the output means belongs to the callers and to `types::resolve`.

use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::types::project::GitUrl;
use crate::types::resolve::FetchError;

/// The variables that make git work in a repository other than the one `-C` names. They are
/// removed from every command, so an inherited `GIT_DIR` (set by a git hook, for one) cannot send a
/// fetch into the user's own repository.
const REPOSITORY_VARIABLES: [&str; 7] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

/// One `git` command to run.
pub(super) struct GitRun<'a> {
    program: &'a OsStr,
    repo: Option<&'a Path>,
    subcommand: &'static str,
    args: Vec<OsString>,
    operands: Vec<OsString>,
    stdin: Option<&'a [u8]>,
}

impl<'a> GitRun<'a> {
    /// A run of `subcommand` by `program`, which is `git` unless a caller says otherwise.
    pub(super) const fn new(program: &'a OsStr, subcommand: &'static str) -> Self {
        Self {
            program,
            repo: None,
            subcommand,
            args: Vec::new(),
            operands: Vec::new(),
            stdin: None,
        }
    }

    /// Runs inside `repo` (`-C <repo>`).
    pub(super) const fn repo(mut self, repo: &'a Path) -> Self {
        self.repo = Some(repo);
        self
    }

    /// Adds arguments before `--`.
    pub(super) fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
    }

    /// Adds the URL as an operand after `--`.
    pub(super) fn url(self, url: &GitUrl) -> Self {
        self.operand(url.as_str())
    }

    /// Adds an operand after `--`.
    pub(super) fn operand(mut self, operand: impl AsRef<OsStr>) -> Self {
        self.operands.push(operand.as_ref().to_owned());
        self
    }

    /// Feeds `bytes` to the command's standard input.
    pub(super) const fn stdin(mut self, bytes: &'a [u8]) -> Self {
        self.stdin = Some(bytes);
        self
    }

    /// The arguments after the program name.
    pub(super) fn argv(&self) -> Vec<OsString> {
        let mut argv: Vec<OsString> = vec!["-c".into(), "core.autocrlf=false".into()];
        if let Some(repo) = self.repo {
            argv.push("-C".into());
            argv.push(repo.into());
        }
        argv.push(self.subcommand.into());
        argv.extend(self.args.iter().cloned());
        if !self.operands.is_empty() {
            argv.push("--".into());
            argv.extend(self.operands.iter().cloned());
        }
        argv
    }

    /// The command, ready to start: output captured, no prompt for credentials, and none of the
    /// variables that would point git at another repository.
    fn command(&self) -> Command {
        let mut command = Command::new(self.program);
        for variable in REPOSITORY_VARIABLES {
            command.env_remove(variable);
        }
        command
            .args(self.argv())
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    /// Runs the command and returns its standard output.
    ///
    /// # Errors
    ///
    /// - [`FetchError::GitNotFound`] with an empty `needed_by` when the program does not exist.
    /// - [`FetchError::GitSpawn`] when it cannot be started for another reason.
    /// - [`FetchError::GitFailed`] with the subcommand and git's trimmed standard error when it
    ///   exits with a failure.
    pub(super) fn run(&self) -> Result<Vec<u8>, FetchError> {
        let mut child = self.command().spawn().map_err(spawn_failure)?;
        let pipe = child.stdin.take();
        // The input is written from its own thread while the output is read, so an answer larger
        // than a pipe buffer cannot leave git and this process each waiting on the other. A write
        // that fails because git already exited shows in git's exit status.
        let output = std::thread::scope(|scope| {
            if let (Some(mut pipe), Some(bytes)) = (pipe, self.stdin) {
                scope.spawn(move || {
                    let _ = pipe.write_all(bytes);
                });
            }
            child.wait_with_output()
        })
        .map_err(FetchError::GitSpawn)?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(FetchError::GitFailed {
                subcommand: self.subcommand.to_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }
}

/// The refusal for a program that did not start: git missing, or another failure.
fn spawn_failure(error: io::Error) -> FetchError {
    if error.kind() == io::ErrorKind::NotFound {
        FetchError::GitNotFound {
            needed_by: Vec::new(),
        }
    } else {
        FetchError::GitSpawn(error)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::process::git::testing::{CRLF, SourceRepo, TestResult};
    use crate::types::report::{Diagnostic, Location};

    const GIT: &str = "git";

    /// Enough ids that git's answers fill the pipe buffers many times over, so the test hangs
    /// when the input is written before any output is read.
    const STDIN_IDS: usize = 20_000;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn argv_puts_autocrlf_first_and_the_url_after_double_dash() -> TestResult {
        let repo = Path::new("/r");
        let run = GitRun::new(OsStr::new(GIT), "fetch")
            .repo(repo)
            .args(["--no-tags", "--depth", "1"])
            .url(&GitUrl::new("u")?)
            .operand("x");
        assert_eq!(
            run.argv(),
            os(&[
                "-c",
                "core.autocrlf=false",
                "-C",
                "/r",
                "fetch",
                "--no-tags",
                "--depth",
                "1",
                "--",
                "u",
                "x"
            ])
        );
        Ok(())
    }

    #[test]
    fn argv_has_no_double_dash_without_operands() {
        let run = GitRun::new(OsStr::new(GIT), "init").args(["--bare", "-q"]);
        assert_eq!(
            run.argv(),
            os(&["-c", "core.autocrlf=false", "init", "--bare", "-q"])
        );
    }

    #[test]
    fn run_sets_git_terminal_prompt_off() {
        let command = GitRun::new(OsStr::new(GIT), "init").command();
        let prompt = command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new("GIT_TERMINAL_PROMPT"))
            .and_then(|(_, value)| value);
        assert_eq!(prompt, Some(OsStr::new("0")));
    }

    #[test]
    fn run_clears_the_variables_that_point_git_at_a_repository() {
        let command = GitRun::new(OsStr::new(GIT), "fetch").command();
        let envs: Vec<_> = command.get_envs().collect();
        for key in REPOSITORY_VARIABLES {
            assert!(
                envs.contains(&(OsStr::new(key), None)),
                "{key} is not removed"
            );
        }
    }

    #[test]
    fn a_missing_program_is_git_not_found() -> TestResult {
        let error = GitRun::new(OsStr::new("/nonexistent/dir/git"), "init")
            .run()
            .err();
        let Some(FetchError::GitNotFound { needed_by }) = &error else {
            return Err(format!("expected GitNotFound, got {error:?}").into());
        };
        assert!(needed_by.is_empty());
        Ok(())
    }

    #[test]
    fn a_failing_command_carries_subcommand_and_trimmed_stderr() -> TestResult {
        let url = GitUrl::new("file:///nope")?;
        let error = GitRun::new(OsStr::new(GIT), "ls-remote")
            .url(&url)
            .run()
            .err();
        let Some(FetchError::GitFailed { subcommand, stderr }) = &error else {
            return Err(format!("expected GitFailed, got {error:?}").into());
        };
        assert_eq!(subcommand, "ls-remote");
        assert!(
            stderr.starts_with("fatal: '/nope' does not appear to be a git repository"),
            "{stderr}"
        );
        assert!(!stderr.ends_with('\n'));
        Ok(())
    }

    #[test]
    fn a_spawn_error_other_than_not_found_is_a_git_spawn_refusal() -> TestResult {
        let dir = tempfile::tempdir()?;
        let error = GitRun::new(dir.path().as_os_str(), "init")
            .run()
            .err()
            .ok_or("expected a failure")?;
        let FetchError::GitSpawn(io) = &error else {
            return Err(format!("expected GitSpawn, got {error:?}").into());
        };
        assert_ne!(io.kind(), io::ErrorKind::NotFound);
        let expected = format!("agentstacks.toml: git: {io}");
        let shown = Diagnostic::new(Location::file("agentstacks.toml"), error).to_string();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn stdin_is_fed_while_output_is_read() -> TestResult {
        let source = SourceRepo::new()?;
        let path = source.path();
        let line = format!("{}\n", source.crlf_blob);
        let input = line.repeat(STDIN_IDS);
        let output = GitRun::new(OsStr::new(GIT), "cat-file")
            .repo(&path)
            .args(["--batch"])
            .stdin(input.as_bytes())
            .run()?;
        let mut frame = format!("{} blob {}\n", source.crlf_blob, CRLF.len()).into_bytes();
        frame.extend_from_slice(CRLF);
        frame.push(b'\n');
        assert_eq!(output, frame.repeat(STDIN_IDS));
        Ok(())
    }
}
