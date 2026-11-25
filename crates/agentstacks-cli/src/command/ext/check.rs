//! The `ext test` command.
//!
//! # Responsibilities
//!
//! - [`run`] resolves the extension directory and each `--pack` directory against the project
//!   directory, runs the library flow with the paths as resolved and as written, prints the report
//!   through [`show`](crate::show) and returns the outcome.
//!
//! # Non-responsibilities
//!
//! The steps of the check and the verdict belong to the library, and the text of the report to
//! `show`. The exit code follows the verdict: a run with no verdict, or a failed one, is
//! [`Outcome::Refused`].

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use agentstacks::app::ext::{Dir, check};
use agentstacks::port::{Adapter, Binary, Git, Scratch};
use agentstacks::types::check::Verdict;

use crate::command::{Console, Ports};
use crate::exit::Outcome;
use crate::show::check_text;

/// What `ext test` was given: the directories as the author wrote them and the directory they are
/// resolved against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckRequest<'a> {
    /// The project directory (`-C`, or the current directory).
    pub(crate) base: &'a Path,
    /// The extension directory.
    pub(crate) extension: &'a Path,
    /// The directories of the packs to install beside the conformance pack.
    pub(crate) packs: &'a [PathBuf],
}

/// Checks the extension `request` names and prints the report.
///
/// Refusals and what `airsl` printed on its standard error go to stderr, everything else to
/// stdout. The scratch project is gone when this returns.
pub(crate) fn run<W, G, B, A, S, I, O, E>(
    ports: &Ports<W, G, B, A>,
    scratch: &S,
    request: CheckRequest<'_>,
    console: &mut Console<I, O, E>,
) -> Outcome
where
    G: Git,
    B: Binary,
    A: Adapter,
    S: Scratch,
    I: BufRead,
    O: Write,
    E: Write,
{
    let CheckRequest {
        base,
        extension,
        packs,
    } = request;
    let resolved = |path: &Path| base.join(path);
    let extension_dir = resolved(extension);
    let extension_label = extension.to_string_lossy();
    let pack_dirs: Vec<(PathBuf, String)> = packs
        .iter()
        .map(|pack| (resolved(pack), pack.to_string_lossy().into_owned()))
        .collect();
    let pack_args: Vec<Dir<'_>> = pack_dirs
        .iter()
        .map(|(dir, label)| Dir {
            resolved: dir,
            label,
        })
        .collect();
    let report = check(
        &ports.binary,
        scratch,
        &ports.git,
        &ports.adapter,
        Dir {
            resolved: &extension_dir,
            label: &extension_label,
        },
        &pack_args,
    );
    let passed = report.verdict == Some(Verdict::Passed);
    let text = check_text(report);
    if !text.out.is_empty() {
        console.out_line(text.out);
    }
    if !text.err.is_empty() {
        console.err_line(text.err);
    }
    if passed {
        Outcome::Success
    } else {
        Outcome::Refused
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::port::{
        FakeAdapter, FakeBinary, FakeFileTree, FakeGit, FakeScratch, FakeWorkspace, ScratchCall,
    };

    use super::*;
    use crate::command::testing::{console, err_of, out_of};

    type FakePorts = Ports<FakeWorkspace, FakeGit, FakeBinary, FakeAdapter>;

    fn ports(binary: FakeBinary) -> FakePorts {
        Ports {
            workspace: FakeWorkspace::new(),
            git: FakeGit::new(),
            binary,
            adapter: FakeAdapter::new(),
        }
    }

    fn request<'a>(extension: &'a str, packs: &'a [PathBuf]) -> CheckRequest<'a> {
        CheckRequest {
            base: Path::new("/proj"),
            extension: Path::new(extension),
            packs,
        }
    }

    #[test]
    fn a_missing_extension_prints_the_path_as_written() {
        let ports = ports(FakeBinary::new().airsl("airsl 0.1.5"));
        let scratch = FakeScratch::new();
        let mut console = console(false, "");
        let got = run(&ports, &scratch, request("ext/missing", &[]), &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(out_of(&console), "");
        assert_eq!(
            err_of(&console),
            format!("ext/missing: {}\n", FakeFileTree::not_found())
        );
        assert_eq!(
            scratch.calls(),
            [ScratchCall::Open {
                dir: PathBuf::from("/proj/ext/missing"),
                label: PathBuf::from("ext/missing"),
            }]
        );
    }

    #[test]
    fn a_missing_airsl_is_refused_without_a_verdict() {
        let ports = ports(FakeBinary::new());
        let mut console = console(false, "");
        let got = run(
            &ports,
            &FakeScratch::new(),
            request("ext/x", &[]),
            &mut console,
        );
        assert_eq!(got, Outcome::Refused);
        assert_eq!(out_of(&console), "");
        assert_eq!(
            err_of(&console),
            "ext test: airsl not found on PATH (needed for the Lua tests and the hooks)\n"
        );
    }

    #[test]
    fn a_failed_verdict_is_refused_and_ends_stdout_with_failed() {
        let ports = ports(FakeBinary::new().airsl("airsl 0.1.5"));
        let manifest = "[extension]\nname = \"mytool\"\nversion = \"0.1.0\"\napi = 1\n\
                        description = \"d\"\nentry = \"adapter.lua\"\n";
        let scratch = FakeScratch::new().dir(
            Path::new("/proj/ext/x"),
            FakeFileTree::new("elsewhere")
                .file("extension.toml", manifest.as_bytes())
                .file("adapter.lua", b"return {}\n"),
        );
        let mut console = console(false, "");
        let packs = [PathBuf::from("../nope")];
        let got = run(&ports, &scratch, request("ext/x", &packs), &mut console);
        assert_eq!(got, Outcome::Refused);
        assert_eq!(out_of(&console).lines().last(), Some("failed"));
        assert_eq!(
            err_of(&console),
            format!("../nope: {}\n", FakeFileTree::not_found())
        );
    }

    #[test]
    fn an_absolute_path_is_kept_and_the_label_is_as_typed() {
        let ports = ports(FakeBinary::new().airsl("airsl 0.1.5"));
        let scratch = FakeScratch::new();
        let mut console = console(false, "");
        let _ = run(&ports, &scratch, request("/abs/ext", &[]), &mut console);
        assert_eq!(
            scratch.calls(),
            [ScratchCall::Open {
                dir: PathBuf::from("/abs/ext"),
                label: PathBuf::from("/abs/ext"),
            }]
        );
    }
}
