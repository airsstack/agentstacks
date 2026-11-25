//! The flow of `ext add`: reading the project, resolving and loading the extension, and writing
//! the project file with the new tool.
//!
//! # Responsibilities
//!
//! - [`add()`] reads `agentstacks.toml`, refuses a legacy one, classifies the source with
//!   [`classify`], checks the options, resolves the source as a plan does and loads it with
//!   [`Extension::load`], reads the file again to see it was not edited meanwhile, and writes the
//!   text [`add_tool`] returns.
//! - It places each refusal at `ext add "<source>"`, at `ext add` for git's own failures, and at
//!   `agentstacks.toml` for the file's. Two refusals sit elsewhere: an I/O error on a directory of
//!   the project is located at that path, and a cache failure at its cache path.
//!
//! # Non-responsibilities
//!
//! No rule: the options a source takes, the newest tag and the table written belong to `types`.
//! The loaders' refusals pass through unchanged.

use std::path::Path;

use crate::app::local::local_tree;
use crate::app::newest::{NewestError, newest_release};
use crate::port::{FileTree, Git, Workspace};
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::project::{
    GitFlags, PROJECT_FILE, Pin, ProjectError, ProjectFile, Source, add_tool,
};
use crate::types::report::{Diagnostic, Diagnostics, FormatError, Location};
use crate::types::resolve::FetchError;
use crate::types::shorthand::{Argument, Flag, classify};

/// Adds the tool extension at `source` to `agentstacks.toml` and returns its name.
///
/// `source` is a git URL (given with `flags.tag` or `flags.rev`, and optionally `flags.subdir`), a
/// directory of the project, or the name of an official tool. The options apply only to a git
/// URL. An official name is written as the official repository at its newest release tag of the
/// `api` major version, with the directory inside it. A directory is written as given, under the
/// name in its manifest.
///
/// # Errors
///
/// Returns every refusal and writes nothing when `agentstacks.toml` is absent, unreadable or
/// refused, or written for the legacy `api`; when the source or an option is refused; when the
/// source cannot be fetched or its extension is refused by [`Extension::load`]; when the project
/// already lists the tool; or when the file changed since it was first read. Returns the refusal of
/// the write when the write fails.
pub fn add<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
    source: &str,
    flags: GitFlags<'_>,
) -> Result<ToolName, Diagnostics> {
    let text = read_project(workspace)?;
    let resolved = resolve(git, source, &flags)?;
    let name = load_name(workspace, git, &resolved, source)?;
    let updated = add_tool(&text, &name, &resolved).map_err(at_project)?;
    let reread = workspace.project_text().map_err(at_project)?;
    if reread != text {
        return Err(at_project(ProjectError::ChangedDuringAdd));
    }
    workspace.write_project(&updated).map_err(at_project)?;
    Ok(name)
}

/// The text of `agentstacks.toml`, checked to be a project file of the current `api`.
fn read_project<W: Workspace>(workspace: &W) -> Result<String, Diagnostics> {
    let text = workspace.project_text().map_err(at_project)?;
    let project = ProjectFile::parse(Path::new(PROJECT_FILE), &text)?;
    project.refuse_legacy().map_err(at_project)?;
    Ok(text)
}

/// The source that `source` and the options name, with every option checked.
fn resolve<G: Git>(git: &G, source: &str, flags: &GitFlags<'_>) -> Result<Source, Diagnostics> {
    let at = || at_source(source);
    let refused = |errors: Vec<ProjectError>| -> Diagnostics {
        let mut found = Diagnostics::new();
        found.extend(errors.into_iter().map(|error| Diagnostic::new(at(), error)));
        found
    };
    match classify(Flag::Ext, source)? {
        Argument::Git(url) => flags.source(url).map_err(refused),
        Argument::Path(path) => match flags.without_git() {
            options if options.is_empty() => Ok(Source::Path(path)),
            options => Err(refused(options)),
        },
        Argument::Official { url, subdir } => {
            let options = flags.without_git();
            if !options.is_empty() {
                return Err(refused(options));
            }
            let tag = newest_release(git, &url).map_err(|error| match error {
                NewestError::Fetch(error) => placed(error, source),
                NewestError::NoTag(error) => Diagnostic::new(at(), error),
            })?;
            Ok(Source::Git {
                url,
                pin: Pin::Tag(tag),
                subdir: Some(subdir),
            })
        }
    }
}

/// Fetches or reads `resolved`, loads its extension and returns the tool's name.
fn load_name<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
    resolved: &Source,
    source: &str,
) -> Result<ToolName, Diagnostics> {
    let at = || at_source(source);
    match resolved {
        Source::Git { url, pin, subdir } => {
            let rev = match pin {
                Pin::Rev(rev) => rev.clone(),
                Pin::Tag(tag) => git.fetch_tag(url, tag).map_err(|e| placed(e, source))?,
            };
            let tree = git
                .tree(url, &rev, subdir.as_ref(), pin)
                .map_err(|error| placed(error, source))?;
            extension_name(&tree)
        }
        Source::Path(path) => {
            let tree = local_tree(workspace, path, at())?;
            extension_name(&tree)
        }
    }
}

/// The name in the manifest of the extension `tree` holds.
fn extension_name<T: FileTree>(tree: &T) -> Result<ToolName, Diagnostics> {
    Extension::load(tree).map(|extension| extension.manifest().name().clone())
}

/// Puts a git failure where the author can find it: the path of a cache failure, the source for a
/// missing directory, and `ext add` for everything else. Git missing names the source that needed
/// it.
fn placed(error: FetchError, source: &str) -> Diagnostic {
    let command = || Location::verbatim(Flag::Ext.to_string());
    match error {
        FetchError::GitNotFound { .. } => Diagnostic::new(
            command(),
            FetchError::GitNotFound {
                needed_by: vec![at_source(source).to_string()],
            },
        ),
        FetchError::CacheIo { path, error } => Diagnostic::new(Location::file(path), error),
        error @ (FetchError::SubdirNotFound { .. } | FetchError::SubdirIsFile { .. }) => {
            Diagnostic::new(at_source(source), error)
        }
        error => Diagnostic::new(command(), error),
    }
}

/// The place of a refusal about `source`: `ext add "<source>"`.
fn at_source(source: &str) -> Location {
    Location::verbatim(format!("{} \"{source}\"", Flag::Ext))
}

/// A collection holding one refusal located at `agentstacks.toml`.
fn at_project(error: impl Into<FormatError>) -> Diagnostics {
    Diagnostic::new(Location::file(PROJECT_FILE), error).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{FakeFileTree, FakeGit, FakeWorkspace, GitCall, WorkspaceCall};
    use crate::types::project::{CommitId, GitTag, GitUrl, LocalPath, SubPath};
    use crate::types::report::Quoted;
    use crate::types::resolve::RemoteTag;
    use crate::types::shorthand::OFFICIAL_REPOSITORY;
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";
    const EXAMPLE: &str = "https://example.com/x.git";
    const PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"./ext/claude\"\n\n[packs.core]\npath = \"./packs/core\"\n";

    fn error_of(error: impl std::error::Error) -> Box<dyn std::error::Error> {
        error.to_string().into()
    }

    fn example() -> Result<GitUrl, Box<dyn std::error::Error>> {
        GitUrl::new(EXAMPLE).map_err(error_of)
    }

    fn official() -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(OFFICIAL_REPOSITORY)?)
    }

    fn extension_tree(root: &str, name: &str, api: i64) -> FakeFileTree {
        let manifest = format!(
            "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = {api}\ndescription = \"d\"\nentry = \"adapter.lua\"\n"
        );
        FakeFileTree::new(root)
            .file("extension.toml", manifest.as_bytes())
            .file("adapter.lua", b"return {}\n")
    }

    fn with_path_tool(
        workspace: FakeWorkspace,
    ) -> Result<FakeWorkspace, Box<dyn std::error::Error>> {
        Ok(workspace.source(
            &LocalPath::new("./ext/mytool")?,
            extension_tree("ext/mytool", "mytool", 1),
        ))
    }

    fn tags(names: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
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

    /// Git serving `extension` for `url` at `subdir` of commit `rev`.
    fn serving(
        url: &GitUrl,
        rev: &str,
        subdir: &str,
        extension: FakeFileTree,
    ) -> Result<FakeGit, Box<dyn std::error::Error>> {
        Ok(FakeGit::new().files(
            url,
            &CommitId::new(rev)?,
            Some(&SubPath::new(subdir)?),
            extension,
        ))
    }

    fn refused(
        result: Result<ToolName, Diagnostics>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(result
            .err()
            .ok_or("expected refusals")?
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    fn add_with<W: Workspace, G: Git>(
        workspace: &W,
        git: &G,
        source: &str,
        tag: Option<&str>,
        rev: Option<&str>,
        subdir: Option<&str>,
    ) -> Result<ToolName, Diagnostics> {
        add(workspace, git, source, GitFlags { tag, rev, subdir })
    }

    fn added(
        workspace: &FakeWorkspace,
        git: &FakeGit,
        source: &str,
    ) -> Result<ToolName, Box<dyn std::error::Error>> {
        add_with(workspace, git, source, None, None, None)
            .map_err(|found| format!("{:?}", found.into_sorted()).into())
    }

    fn tools_of(text: &str) -> Result<Vec<(String, Source)>, Box<dyn std::error::Error>> {
        let project = ProjectFile::parse(Path::new(PROJECT_FILE), text)
            .map_err(|found| format!("{:?}", found.into_sorted()))?;
        let mut tools: Vec<(String, Source)> = project
            .tools()
            .iter()
            .map(|(name, source)| (name.as_str().to_owned(), source.clone()))
            .collect();
        tools.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(tools)
    }

    fn written(workspace: &FakeWorkspace) -> Result<String, Box<dyn std::error::Error>> {
        Ok(workspace.project_written().ok_or("a written file")?)
    }

    #[test]
    fn adds_a_path_extension() -> TestResult {
        let workspace = with_path_tool(FakeWorkspace::new().project(PROJECT))?;
        let name = added(&workspace, &FakeGit::new(), "./ext/mytool")?;
        assert_eq!(name.as_str(), "mytool");
        assert_eq!(
            written(&workspace)?,
            "api = 1\n\n[tools.claude]\npath = \"./ext/claude\"\n\n[tools.mytool]\npath = \"./ext/mytool\"\n\n[packs.core]\npath = \"./packs/core\"\n"
        );
        Ok(())
    }

    #[test]
    fn adds_a_git_extension_with_tag() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let tag = GitTag::new("v1.2.0")?;
        let git = serving(
            &example()?,
            REV,
            "ext/mytool",
            extension_tree("ext/mytool", "mytool", 1),
        )?
        .tag(&example()?, &tag, &CommitId::new(REV)?);
        add_with(
            &workspace,
            &git,
            EXAMPLE,
            Some("v1.2.0"),
            None,
            Some("ext/mytool"),
        )
        .map_err(|found| format!("{:?}", found.into_sorted()))?;
        let text = written(&workspace)?;
        assert!(
            text.contains(&format!(
                "[tools.mytool]\ngit = \"{EXAMPLE}\"\ntag = \"v1.2.0\"\nsubdir = \"ext/mytool\"\n"
            )),
            "{text}"
        );
        let tools = tools_of(&text)?;
        assert_eq!(tools.len(), 2);
        Ok(())
    }

    #[test]
    fn adds_a_git_extension_with_rev() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let git = serving(
            &example()?,
            REV,
            "ext/mytool",
            extension_tree("ext/mytool", "mytool", 1),
        )?;
        add_with(
            &workspace,
            &git,
            EXAMPLE,
            None,
            Some(REV),
            Some("ext/mytool"),
        )
        .map_err(|found| format!("{:?}", found.into_sorted()))?;
        let text = written(&workspace)?;
        assert!(
            text.contains(&format!(
                "[tools.mytool]\ngit = \"{EXAMPLE}\"\nrev = \"{REV}\"\nsubdir = \"ext/mytool\"\n"
            )),
            "{text}"
        );
        assert!(
            !git.calls()
                .iter()
                .any(|call| matches!(call, GitCall::FetchTag { .. }))
        );
        Ok(())
    }

    #[test]
    fn adds_the_official_extension() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let url = official()?;
        let git = serving(
            &url,
            REV,
            "extensions/opencode",
            extension_tree("extensions/opencode", "opencode", 1),
        )?
        .tags(
            &url,
            tags(&["packs-v1.0.0", "packs-v1.2.0", "packs-v2.0.0"])?,
        )
        .tag(&url, &GitTag::new("packs-v1.2.0")?, &CommitId::new(REV)?);
        let name = added(&workspace, &git, "opencode")?;
        assert_eq!(name.as_str(), "opencode");
        let text = written(&workspace)?;
        assert!(
            text.contains(&format!(
                "[tools.opencode]\ngit = \"{OFFICIAL_REPOSITORY}\"\ntag = \"packs-v1.2.0\"\nsubdir = \"extensions/opencode\"\n"
            )),
            "{text}"
        );
        Ok(())
    }

    #[test]
    fn no_project_file() -> TestResult {
        let workspace = FakeWorkspace::new();
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/mytool",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            [format!("agentstacks.toml: {}", FakeFileTree::not_found())]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn legacy_project_refused() -> TestResult {
        let workspace = with_path_tool(
            FakeWorkspace::new().project(&PROJECT.replacen("api = 1", "api = 0", 1)),
        )?;
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/mytool",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            ["agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn unparsable_project_refused() -> TestResult {
        let workspace = FakeWorkspace::new().project(&PROJECT.replacen("api = 1", "api = 2", 1));
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/mytool",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            ["agentstacks.toml: api 2 not supported (this agentstacks speaks 1)"]
        );
        Ok(())
    }

    #[test]
    fn unknown_bare_name() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let result = add_with(&workspace, &FakeGit::new(), "x", None, None, None);
        assert_eq!(
            refused(result)?,
            ["ext add \"x\": not an official tool; pass a path or a git URL"]
        );
        Ok(())
    }

    #[test]
    fn git_url_needs_tag_or_rev() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let git = FakeGit::new();
        let result = add_with(
            &workspace,
            &git,
            "https://example.com/x.git",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            ["ext add \"https://example.com/x.git\": a git URL needs --tag or --rev"]
        );
        assert!(git.calls().is_empty());
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn flag_only_for_git() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        for source in ["./ext/x", "claude"] {
            for (flag, tag, rev, subdir) in [
                ("--tag", Some("v1"), None, None),
                ("--rev", None, Some(REV), None),
                ("--subdir", None, None, Some("x")),
            ] {
                let result = add_with(&workspace, &FakeGit::new(), source, tag, rev, subdir);
                assert_eq!(
                    refused(result)?,
                    [format!(
                        "ext add \"{source}\": {flag} applies only to a git URL"
                    )]
                );
            }
        }
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/x",
            Some("v1"),
            None,
            Some("x"),
        );
        assert_eq!(
            refused(result)?,
            [
                "ext add \"./ext/x\": --tag applies only to a git URL",
                "ext add \"./ext/x\": --subdir applies only to a git URL",
            ]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn rev_not_a_commit() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            EXAMPLE,
            None,
            Some("abc"),
            None,
        );
        assert_eq!(
            refused(result)?,
            [format!("ext add \"{EXAMPLE}\": --rev: not a commit id")]
        );
        Ok(())
    }

    #[test]
    fn subdir_leaves_repository() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            EXAMPLE,
            Some("v1"),
            None,
            Some("../x"),
        );
        assert_eq!(
            refused(result)?,
            [format!(
                "ext add \"{EXAMPLE}\": --subdir: must stay inside the repository"
            )]
        );
        Ok(())
    }

    #[test]
    fn empty_flag_value() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        for (flag, tag, rev, subdir) in [
            ("--tag", Some(""), None, None),
            ("--rev", None, Some(""), None),
            ("--subdir", Some("v1"), None, Some("")),
        ] {
            let result = add_with(&workspace, &FakeGit::new(), EXAMPLE, tag, rev, subdir);
            assert_eq!(
                refused(result)?,
                [format!("ext add \"{EXAMPLE}\": {flag}: empty")]
            );
        }
        Ok(())
    }

    #[test]
    fn path_absolute() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let result = add_with(&workspace, &FakeGit::new(), "/x", None, None, None);
        assert_eq!(
            refused(result)?,
            ["ext add \"/x\": absolute paths are not portable"]
        );
        Ok(())
    }

    #[test]
    fn path_climbs() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let result = add_with(&workspace, &FakeGit::new(), "../x", None, None, None);
        assert_eq!(
            refused(result)?,
            ["ext add \"../x\": must stay inside the project"]
        );
        Ok(())
    }

    #[test]
    fn path_out_through_symlink() -> TestResult {
        let workspace = FakeWorkspace::new()
            .project(PROJECT)
            .source_outside(&LocalPath::new("./x")?);
        let result = add_with(&workspace, &FakeGit::new(), "./x", None, None, None);
        assert_eq!(
            refused(result)?,
            ["ext add \"./x\": resolves outside the project"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn path_io_error_is_located_at_the_path() -> TestResult {
        let dir = LocalPath::new("./ext/mytool")?;
        let workspace = FakeWorkspace::new()
            .project(PROJECT)
            .fail(WorkspaceCall::SourceTree(dir));
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/mytool",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            [format!("./ext/mytool: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn write_fails() -> TestResult {
        let workspace = with_path_tool(
            FakeWorkspace::new()
                .project(PROJECT)
                .fail(WorkspaceCall::WriteProject),
        )?;
        let result = add_with(
            &workspace,
            &FakeGit::new(),
            "./ext/mytool",
            None,
            None,
            None,
        );
        assert_eq!(
            refused(result)?,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn git_missing() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let git = FakeGit::new().git_missing();
        for (source, tag) in [("claude", None), (EXAMPLE, Some("v1"))] {
            let result = add_with(&workspace, &git, source, tag, None, None);
            assert_eq!(
                refused(result)?,
                [format!(
                    "ext add: git not found on PATH (needed by ext add \"{source}\")"
                )]
            );
        }
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn git_fails() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let failure = || FetchError::GitFailed {
            subcommand: "ls-remote".to_owned(),
            stderr: "fatal: no".to_owned(),
        };
        let listing = FakeGit::new().fail(&official()?, failure());
        let result = add_with(&workspace, &listing, "claude", None, None, None);
        assert_eq!(
            refused(result)?,
            ["ext add: git ls-remote failed: fatal: no"]
        );
        let fetching = FakeGit::new().fail(&example()?, failure());
        let result = add_with(&workspace, &fetching, EXAMPLE, Some("v1"), None, None);
        assert_eq!(
            refused(result)?,
            ["ext add: git ls-remote failed: fatal: no"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn subdir_not_found_is_located_at_the_source() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let missing = FetchError::SubdirNotFound {
            subdir: Quoted::new("ext/mytool"),
            at: REV.to_owned(),
        };
        let is_file = FetchError::SubdirIsFile {
            subdir: Quoted::new("ext/mytool"),
        };
        for (error, shown) in [
            (missing, format!("\"ext/mytool\" not found at {REV}")),
            (
                is_file,
                "\"ext/mytool\" is a file, not a directory".to_owned(),
            ),
        ] {
            let git = FakeGit::new().fail(&example()?, error);
            let result = add_with(
                &workspace,
                &git,
                EXAMPLE,
                None,
                Some(REV),
                Some("ext/mytool"),
            );
            assert_eq!(
                refused(result)?,
                [format!("ext add \"{EXAMPLE}\": {shown}")]
            );
        }
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn cache_io_is_located_at_its_path() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let error = FetchError::CacheIo {
            path: PathBuf::from("cache/agentstacks/x"),
            error: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let git = FakeGit::new().fail(&example()?, error);
        let result = add_with(&workspace, &git, EXAMPLE, None, Some(REV), None);
        assert_eq!(refused(result)?, ["cache/agentstacks/x: denied"]);
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn official_without_major_tag() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let git = FakeGit::new().tags(&official()?, tags(&["packs-v0.2.0", "packs-v2.0.0"])?);
        let result = add_with(&workspace, &git, "claude", None, None, None);
        assert_eq!(
            refused(result)?,
            [
                "ext add \"claude\": no packs-v1 release tag in https://github.com/airsstack/agentstacks"
            ]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn legacy_or_too_new_extension_refused() -> TestResult {
        for (api, shown) in [
            (
                0,
                "ext/mytool/extension.toml: extension.api: 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)",
            ),
            (
                2,
                "ext/mytool/extension.toml: extension.api: 2 not supported (this agentstacks speaks 1)",
            ),
        ] {
            let workspace = FakeWorkspace::new().project(PROJECT).source(
                &LocalPath::new("./ext/mytool")?,
                extension_tree("ext/mytool", "mytool", api),
            );
            let result = add_with(
                &workspace,
                &FakeGit::new(),
                "./ext/mytool",
                None,
                None,
                None,
            );
            assert_eq!(refused(result)?, [shown]);
            assert_eq!(workspace.project_written(), None);
        }
        Ok(())
    }

    #[test]
    fn tool_exists() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT).source(
            &LocalPath::new("./ext/other")?,
            extension_tree("ext/other", "claude", 1),
        );
        let result = add_with(&workspace, &FakeGit::new(), "./ext/other", None, None, None);
        assert_eq!(
            refused(result)?,
            ["agentstacks.toml: tools.claude already exists (edit it there)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    /// A git that edits `agentstacks.toml` while it serves a tree, as an editor would during a
    /// slow fetch.
    struct Editing<'a> {
        git: FakeGit,
        workspace: &'a FakeWorkspace,
        edited: &'a str,
    }

    impl Git for Editing<'_> {
        type Tree = FakeFileTree;

        fn list_tags(&self, url: &GitUrl, prefix: &str) -> Result<Vec<RemoteTag>, FetchError> {
            self.git.list_tags(url, prefix)
        }

        fn fetch_tag(&self, url: &GitUrl, tag: &GitTag) -> Result<CommitId, FetchError> {
            self.git.fetch_tag(url, tag)
        }

        fn tree(
            &self,
            url: &GitUrl,
            rev: &CommitId,
            subdir: Option<&SubPath>,
            at: &Pin,
        ) -> Result<Self::Tree, FetchError> {
            self.workspace.set_project(self.edited);
            self.git.tree(url, rev, subdir, at)
        }
    }

    #[test]
    fn changed_during_fetch() -> TestResult {
        let workspace = FakeWorkspace::new().project(PROJECT);
        let edited = format!("{PROJECT}# edited meanwhile\n");
        let editing = Editing {
            git: serving(
                &example()?,
                REV,
                "ext/mytool",
                extension_tree("ext/mytool", "mytool", 1),
            )?,
            workspace: &workspace,
            edited: &edited,
        };
        let result = add_with(
            &workspace,
            &editing,
            EXAMPLE,
            None,
            Some(REV),
            Some("ext/mytool"),
        );
        assert_eq!(
            refused(result)?,
            ["agentstacks.toml: changed during ext add (run it again)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }
}
