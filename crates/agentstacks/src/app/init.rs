//! The flow that writes a new `agentstacks.toml` from the arguments of `init`.
//!
//! It exists so a project starts from one command: each `--tool` and `--pack` names either an
//! official entry or a directory in the project, and the flow turns them into the file the other
//! flows read.
//!
//! # Responsibilities
//!
//! - [`init`] refuses when the file is already there, collapses repeated arguments, classifies
//!   each one with [`classify`], resolves it (the newest `packs-v*` tag of the official repository
//!   with the major version of the `api`, or the name in the manifest of a project directory),
//!   refuses two arguments of one option that share a key, and writes the text [`render_project`]
//!   returns.
//! - It collects every refusal of the arguments before it stops, and writes nothing then. Git
//!   missing from `PATH` is one refusal at `init` that names every official argument.
//!
//! # Non-responsibilities
//!
//! No rule: which arguments are paths, which tag is newest and the layout of the file belong to
//! `types`. The flow does no pack check of its own; a directory is loaded with [`Pack::load`] or
//! [`Extension::load`], whose refusals pass through unchanged.

use std::io::ErrorKind;

use crate::app::local::local_tree;
use crate::app::newest::{NewestError, newest_release};
use crate::port::{Git, Workspace};
use crate::types::extension::Extension;
use crate::types::name::{PackName, ToolName};
use crate::types::pack::Pack;
use crate::types::project::{
    GitUrl, PROJECT_FILE, Pin, ProjectError, Source, SubPath, render_project,
};
use crate::types::report::{AlreadyGiven, Diagnostic, Diagnostics, FormatError, Location};
use crate::types::resolve::FetchError;
use crate::types::shorthand::{Argument, Flag, ShorthandError, classify};

/// The location of refusals that belong to no argument and no file: `init` itself.
const INIT: &str = "init";

/// One resolved argument: its key in the file and where it comes from.
struct Resolved {
    key: String,
    source: Source,
}

/// What resolving one argument came to.
enum Outcome {
    Resolved(Resolved),
    Refused(Diagnostics),
    /// Git is not on `PATH`; the flow merges every argument that met this into one refusal.
    NeedsGit,
}

/// Writes `agentstacks.toml` listing `tools` and `packs`.
///
/// Each argument is an official name or a directory in the project. An official name is written as
/// the official repository at its newest `packs-v*` tag of the `api` major version, with the
/// directory inside it. A directory is written as given, under the name in its manifest.
///
/// # Errors
///
/// Returns every refusal, sorted, and writes nothing, when `agentstacks.toml` exists or cannot be
/// looked at, when an argument is refused (an unknown name, a path that is absolute, climbs out of
/// the project or resolves outside it, a directory the loaders refuse, no release tag, git's own
/// failure), when two arguments of one option give the same key, or when git is missing from `PATH`
/// (one refusal at `init` that names every official argument). Returns the refusal of the write
/// when the write fails.
pub fn init<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
    tools: &[String],
    packs: &[String],
) -> Result<(), Diagnostics> {
    match workspace.project_text() {
        Ok(_) => return Err(at_project(ProjectError::AlreadyExists)),
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(at_project(error)),
    }

    let mut refusals = Diagnostics::new();
    let mut without_git = Vec::new();
    let tools = collect(
        workspace,
        git,
        Flag::Tool,
        tools,
        &mut refusals,
        &mut without_git,
    );
    let packs = collect(
        workspace,
        git,
        Flag::Pack,
        packs,
        &mut refusals,
        &mut without_git,
    );
    if !without_git.is_empty() {
        let error = FetchError::GitNotFound {
            needed_by: without_git,
        };
        refusals.push(Diagnostic::new(Location::verbatim(INIT), error));
    }
    let tools = named(tools, ToolName::new, &mut refusals);
    let packs = named(packs, PackName::new, &mut refusals);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    workspace
        .write_project(&render_project(&tools, &packs))
        .map_err(at_project)
}

/// Resolves the arguments of one option, in the order given, each repeated argument once.
///
/// A refusal goes to `refusals`, an argument that needs git to `without_git` as its flag text, and
/// a second argument that gives a key already given is refused at itself. The arguments that
/// resolve come back with their flag text, for the typed names to be made from them.
fn collect<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
    flag: Flag,
    arguments: &[String],
    refusals: &mut Diagnostics,
    without_git: &mut Vec<String>,
) -> Vec<(String, Resolved)> {
    let mut given: Vec<(String, Resolved)> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for arg in arguments {
        if seen.contains(&arg.as_str()) {
            continue;
        }
        seen.push(arg);
        let text = flag_text(flag, arg);
        let resolved = match resolve(workspace, git, flag, arg, &text) {
            Outcome::Resolved(resolved) => resolved,
            Outcome::Refused(found) => {
                refusals.extend(found);
                continue;
            }
            Outcome::NeedsGit => {
                without_git.push(text);
                continue;
            }
        };
        if let Some((first, _)) = given
            .iter()
            .find(|(_, earlier)| earlier.key == resolved.key)
        {
            let error =
                ProjectError::from(AlreadyGiven::new(flag.noun(), resolved.key, first.as_str()));
            refusals.push(Diagnostic::new(Location::verbatim(text), error));
            continue;
        }
        given.push((text, resolved));
    }
    given
}

/// Resolves one argument, given with `flag` and written as `text`.
fn resolve<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
    flag: Flag,
    arg: &str,
    text: &str,
) -> Outcome {
    let at = || Location::verbatim(text);
    match classify(flag, arg) {
        Err(refused) => Outcome::Refused(refused.into()),
        Ok(Argument::Official { url, subdir }) => official(git, arg, &url, subdir, text),
        // `init` has no git option, so only `ext add` meets a git URL.
        Ok(Argument::Git(_)) => {
            Outcome::Refused(Diagnostic::new(at(), ShorthandError::NotOfficial(flag)).into())
        }
        Ok(Argument::Path(path)) => {
            let tree = match local_tree(workspace, &path, at()) {
                Ok(tree) => tree,
                Err(refusal) => return Outcome::Refused(refusal.into()),
            };
            let loaded = match flag {
                Flag::Pack => Pack::load(&tree).map(|pack| pack.manifest().name().to_string()),
                // `Flag::Ext` is the source of `ext add`, which never reaches `init`.
                Flag::Tool | Flag::Ext => {
                    Extension::load(&tree).map(|extension| extension.manifest().name().to_string())
                }
            };
            match loaded {
                Ok(key) => Outcome::Resolved(Resolved {
                    key,
                    source: Source::Path(path),
                }),
                Err(refusals) => Outcome::Refused(refusals),
            }
        }
    }
}

/// Resolves the official name `arg` to the newest release tag of the official repository that has
/// the major version of the `api`.
fn official<G: Git>(git: &G, arg: &str, url: &GitUrl, subdir: SubPath, text: &str) -> Outcome {
    let tag = match newest_release(git, url) {
        Ok(tag) => tag,
        Err(NewestError::Fetch(FetchError::GitNotFound { .. })) => return Outcome::NeedsGit,
        Err(error) => {
            return Outcome::Refused(Diagnostic::new(Location::verbatim(text), error).into());
        }
    };
    Outcome::Resolved(Resolved {
        key: arg.to_owned(),
        source: Source::Git {
            url: url.clone(),
            pin: Pin::Tag(tag),
            subdir: Some(subdir),
        },
    })
}

/// The entries of `given` under their typed names, each name made by `make`.
///
/// A key that is not a valid name is refused at its argument. The loaders and the official lists
/// only give valid names, so this is the one place a name's type is checked, not a case to expect.
fn named<N, E>(
    given: Vec<(String, Resolved)>,
    make: impl Fn(&str) -> Result<N, E>,
    refusals: &mut Diagnostics,
) -> Vec<(N, Source)> {
    let mut entries = Vec::new();
    for (text, resolved) in given {
        match make(&resolved.key) {
            Ok(name) => entries.push((name, resolved.source)),
            Err(_) => refusals.push(Diagnostic::new(
                Location::verbatim(text),
                ProjectError::InvalidName,
            )),
        }
    }
    entries
}

/// The option and its argument as written: `--pack "core"`.
fn flag_text(flag: Flag, arg: &str) -> String {
    format!("{flag} \"{arg}\"")
}

/// A collection holding one refusal located at `agentstacks.toml`.
fn at_project(error: impl Into<FormatError>) -> Diagnostics {
    Diagnostic::new(Location::file(PROJECT_FILE), error).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{FakeFileTree, FakeGit, FakeWorkspace, GitCall, WorkspaceCall};
    use crate::types::project::{CommitId, GitTag, LocalPath};
    use crate::types::resolve::RemoteTag;
    use crate::types::shorthand::OFFICIAL_REPOSITORY;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    fn url() -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(OFFICIAL_REPOSITORY)?)
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn listing(names: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        names
            .iter()
            .map(|name| {
                Ok(RemoteTag {
                    rev: CommitId::new(REV)?,
                    tag: GitTag::new(name)?,
                })
            })
            .collect()
    }

    /// A git whose official repository lists `packs-v1.0.0` and `packs-v1.1.0`.
    fn git() -> Result<FakeGit, Box<dyn std::error::Error>> {
        Ok(FakeGit::new().tags(&url()?, listing(&["packs-v1.0.0", "packs-v1.1.0"])?))
    }

    fn pack_tree(root: &str, name: &str) -> FakeFileTree {
        let identity = format!(
            "[pack]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n"
        );
        FakeFileTree::new(root)
            .file("pack.toml", identity.as_bytes())
            .file(
                "skills/a/SKILL.md",
                b"---\nname: a\ndescription: d\n---\n\nbody\n",
            )
    }

    fn extension_tree(root: &str, name: &str) -> FakeFileTree {
        let manifest = format!(
            "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\n"
        );
        FakeFileTree::new(root)
            .file("extension.toml", manifest.as_bytes())
            .file("adapter.lua", b"return {}\n")
    }

    fn with_pack(
        workspace: FakeWorkspace,
        path: &str,
        name: &str,
    ) -> Result<FakeWorkspace, Box<dyn std::error::Error>> {
        let dir = LocalPath::new(path)?;
        let root = path.trim_start_matches("./");
        Ok(workspace.source(&dir, pack_tree(root, name)))
    }

    fn official_source(subdir: &str, tag: &str) -> Result<Source, Box<dyn std::error::Error>> {
        Ok(Source::Git {
            url: url()?,
            pin: Pin::Tag(GitTag::new(tag)?),
            subdir: Some(SubPath::new(subdir)?),
        })
    }

    fn refused(result: Result<(), Diagnostics>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(result
            .err()
            .ok_or("expected refusals")?
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    #[test]
    fn init_refuses_an_existing_project_file() -> TestResult {
        let workspace = FakeWorkspace::new().project("api = 1\n");
        let result = init(
            &workspace,
            &git()?,
            &strings(&["claude"]),
            &strings(&["core"]),
        );
        assert_eq!(
            refused(result)?,
            ["agentstacks.toml: already exists (edit it and run apply)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_reports_a_read_error_other_than_not_found() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::ProjectText);
        let result = init(&workspace, &git()?, &strings(&["claude"]), &[]);
        assert_eq!(
            refused(result)?,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_writes_the_official_set() -> TestResult {
        let workspace = FakeWorkspace::new();
        init(
            &workspace,
            &git()?,
            &strings(&["claude"]),
            &strings(&["core", "sdlc"]),
        )
        .map_err(|refusals| format!("{refusals:?}"))?;
        let expected = render_project(
            &[(
                ToolName::new("claude")?,
                official_source("extensions/claude", "packs-v1.1.0")?,
            )],
            &[
                (
                    PackName::new("core")?,
                    official_source("packs/core", "packs-v1.1.0")?,
                ),
                (
                    PackName::new("sdlc")?,
                    official_source("packs/sdlc", "packs-v1.1.0")?,
                ),
            ],
        );
        assert_eq!(workspace.project_written(), Some(expected));
        Ok(())
    }

    #[test]
    fn init_collapses_a_repeated_argument() -> TestResult {
        let workspace = FakeWorkspace::new();
        let git = git()?;
        init(&workspace, &git, &[], &strings(&["core", "core"]))
            .map_err(|refusals| format!("{refusals:?}"))?;
        let expected = render_project(
            &[],
            &[(
                PackName::new("core")?,
                official_source("packs/core", "packs-v1.1.0")?,
            )],
        );
        assert_eq!(workspace.project_written(), Some(expected));
        let listed = git
            .calls()
            .iter()
            .filter(|call| matches!(call, GitCall::ListTags { .. }))
            .count();
        assert_eq!(listed, 1);
        Ok(())
    }

    #[test]
    fn init_refuses_two_arguments_with_the_same_key() -> TestResult {
        let workspace = with_pack(FakeWorkspace::new(), "./packs/core", "core")?;
        let result = init(
            &workspace,
            &git()?,
            &[],
            &strings(&["core", "./packs/core"]),
        );
        assert_eq!(
            refused(result)?,
            ["--pack \"./packs/core\": pack \"core\" is already given by --pack \"core\""]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_refuses_the_tool_duplicate_with_the_tool_wording() -> TestResult {
        let workspace =
            FakeWorkspace::new().source(&LocalPath::new("./ext")?, extension_tree("ext", "claude"));
        let result = init(&workspace, &git()?, &strings(&["claude", "./ext"]), &[]);
        assert_eq!(
            refused(result)?,
            ["--tool \"./ext\": tool \"claude\" is already given by --tool \"claude\""]
        );
        Ok(())
    }

    #[test]
    fn init_refuses_when_no_release_tag_exists() -> TestResult {
        let git = FakeGit::new().tags(&url()?, listing(&["v1.0.0", "packs-v1.0.0-rc.1"])?);
        let workspace = FakeWorkspace::new();
        let result = init(&workspace, &git, &[], &strings(&["core"]));
        assert_eq!(
            refused(result)?,
            [
                "--pack \"core\": no packs-v1 release tag in https://github.com/airsstack/agentstacks"
            ]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_official_picks_api_major() -> TestResult {
        let git = FakeGit::new().tags(&url()?, listing(&["packs-v1.0.0", "packs-v2.0.0"])?);
        let workspace = FakeWorkspace::new();
        init(&workspace, &git, &[], &strings(&["core"]))
            .map_err(|refusals| format!("{refusals:?}"))?;
        let written = workspace.project_written().ok_or("a written file")?;
        assert!(written.contains("tag = \"packs-v1.0.0\""), "{written}");
        assert!(!written.contains("packs-v2.0.0"), "{written}");
        Ok(())
    }

    #[test]
    fn init_no_major_tag_refusal() -> TestResult {
        let git = FakeGit::new().tags(&url()?, listing(&["packs-v0.2.0"])?);
        let workspace = FakeWorkspace::new();
        let result = init(&workspace, &git, &[], &strings(&["core"]));
        assert_eq!(
            refused(result)?,
            [
                "--pack \"core\": no packs-v1 release tag in https://github.com/airsstack/agentstacks"
            ]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_passes_a_fetch_error_through_at_the_flag() -> TestResult {
        let error = FetchError::GitFailed {
            subcommand: "ls-remote".to_owned(),
            stderr: "fatal: no".to_owned(),
        };
        let shown = error.to_string();
        let git = FakeGit::new().fail(&url()?, error);
        let result = init(&FakeWorkspace::new(), &git, &[], &strings(&["core"]));
        assert_eq!(refused(result)?, [format!("--pack \"core\": {shown}")]);
        Ok(())
    }

    #[test]
    fn init_merges_git_missing_into_one_refusal() -> TestResult {
        let workspace = with_pack(FakeWorkspace::new(), "./packs/team", "team")?;
        let git = FakeGit::new().git_missing_for(&url()?);
        let result = init(
            &workspace,
            &git,
            &strings(&["claude"]),
            &strings(&["core", "core", "./packs/team"]),
        );
        assert_eq!(
            refused(result)?,
            ["init: git not found on PATH (needed by --tool \"claude\", --pack \"core\")"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_refuses_a_path_that_resolves_outside() -> TestResult {
        let workspace = FakeWorkspace::new().source_outside(&LocalPath::new("./x")?);
        let result = init(&workspace, &git()?, &[], &strings(&["./x"]));
        assert_eq!(
            refused(result)?,
            ["--pack \"./x\": resolves outside the project"]
        );
        Ok(())
    }

    #[test]
    fn init_shows_every_argument_as_written() -> TestResult {
        let workspace = FakeWorkspace::new().source_outside(&LocalPath::new("./x//out")?);
        let result = init(&workspace, &git()?, &strings(&["./x//out"]), &[]);
        assert_eq!(
            refused(result)?,
            ["--tool \"./x//out\": resolves outside the project"]
        );
        Ok(())
    }

    #[test]
    fn init_passes_a_loader_refusal_through() -> TestResult {
        let workspace = FakeWorkspace::new().source(
            &LocalPath::new("./packs/x")?,
            FakeFileTree::new("packs/x").file("README.md", b"x"),
        );
        let result = init(&workspace, &git()?, &[], &strings(&["./packs/x"]));
        assert_eq!(refused(result)?, ["packs/x: not a pack (no pack.toml)"]);
        Ok(())
    }

    #[test]
    fn init_collects_every_refusal_before_writing() -> TestResult {
        let workspace = FakeWorkspace::new().source_outside(&LocalPath::new("./away")?);
        let result = init(&workspace, &git()?, &[], &strings(&["nonesuch", "./away"]));
        assert_eq!(
            refused(result)?,
            [
                "--pack \"./away\": resolves outside the project",
                "--pack \"nonesuch\": not an official pack; pass a path, or add a git source to agentstacks.toml",
            ]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn init_reports_a_failed_write() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::WriteProject);
        let result = init(&workspace, &git()?, &strings(&["claude"]), &[]);
        assert_eq!(
            refused(result)?,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn init_reports_a_path_io_error_as_written() -> TestResult {
        let dir = LocalPath::new("./packs/x")?;
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::SourceTree(dir));
        let result = init(&workspace, &git()?, &[], &strings(&["./packs/x"]));
        assert_eq!(
            refused(result)?,
            [format!("./packs/x: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn init_writes_a_path_source_as_given() -> TestResult {
        let workspace = with_pack(FakeWorkspace::new(), "./packs/team", "team")?;
        init(&workspace, &git()?, &[], &strings(&["./packs/team"]))
            .map_err(|refusals| format!("{refusals:?}"))?;
        let expected = render_project(
            &[],
            &[(
                PackName::new("team")?,
                Source::Path(LocalPath::new("./packs/team")?),
            )],
        );
        assert_eq!(workspace.project_written(), Some(expected.clone()));
        assert!(expected.contains("[packs.team]\npath = \"./packs/team\"\n"));
        Ok(())
    }
}
