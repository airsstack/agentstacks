//! The flow that finds newer tags for the packs and extensions taken from git sources.
//!
//! It exists so a user can see, without changing anything, which git source of `agentstacks.toml`
//! has a newer released tag. It never writes the file: putting a new tag into it belongs to the
//! command.
//!
//! # Responsibilities
//!
//! - [`update`] reads and parses `agentstacks.toml`, visits every source in key order (`packs.*`,
//!   then `tools.*`), asks the update rules what each source needs, lists the tags of the git
//!   sources that need them and hands the tags back to the rules. It collects one entry per git
//!   source, or every refusal: git's failures located at their source, and one refusal for git
//!   missing from `PATH` that names every source that needed it.
//!
//! # Non-responsibilities
//!
//! No rule: how a tag is split, which tag is newest and the words of every note belong to
//! [`UpdateQuery`] and [`UpdateNote`](crate::types::resolve::UpdateNote). No write, and no
//! shorthand expansion.

use std::path::Path;

use crate::port::{Git, Workspace};
use crate::types::project::ProjectFile;
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::{FetchError, SourceKey, UpdateEntry, UpdateQuery};

/// The project file, the file every refusal here is located in.
const PROJECT_FILE: &str = "agentstacks.toml";

/// The newest released tag for each git source of `agentstacks.toml`.
///
/// There is one entry per git source in key order, `packs.*` then `tools.*`, and none for a `path`
/// source. An entry says what the source is pinned to now and either the newer tag or why there is
/// none.
///
/// # Errors
///
/// Returns every refusal, sorted, and no entries, when the project file cannot be read or is
/// refused, or when listing the tags of a source fails. A failure of git is located at the
/// source's key; git missing from `PATH` is one refusal at the file that names every source that
/// asked for it.
pub fn update<W: Workspace, G: Git>(
    workspace: &W,
    git: &G,
) -> Result<Vec<UpdateEntry>, Diagnostics> {
    let text = workspace
        .project_text()
        .map_err(|error| single(Diagnostic::new(Location::file(PROJECT_FILE), error)))?;
    let project = ProjectFile::parse(Path::new(PROJECT_FILE), &text)?;
    let packs = project
        .packs()
        .iter()
        .map(|(name, entry)| (SourceKey::Pack(name.clone()), entry.source()));
    let tools = project
        .tools()
        .iter()
        .map(|(name, source)| (SourceKey::Tool(name.clone()), source));

    let mut entries = Vec::new();
    let mut refusals = Diagnostics::new();
    let mut without_git = Vec::new();
    for (key, source) in packs.chain(tools) {
        let query = match UpdateQuery::for_source(key, source) {
            None => continue,
            Some(Err(entry)) => {
                entries.push(entry);
                continue;
            }
            Some(Ok(query)) => query,
        };
        match git.list_tags(&query.url, &query.prefix) {
            Ok(tags) => entries.push(query.finish(&tags)),
            Err(FetchError::GitNotFound { .. }) => without_git.push(query.key.to_string()),
            Err(error) => refusals.push(located(&query.key, error)),
        }
    }
    if !without_git.is_empty() {
        let error = FetchError::GitNotFound {
            needed_by: without_git,
        };
        refusals.push(Diagnostic::new(Location::file(PROJECT_FILE), error));
    }
    refusals.into_result(entries)
}

/// The refusal for `error`, which git failed with while listing the tags of `key`: located at the
/// path a cache failure names, and at the source's key otherwise.
fn located(key: &SourceKey, error: FetchError) -> Diagnostic {
    let location = match &error {
        FetchError::CacheIo { path, .. } => Location::file(path.clone()),
        _ => key.location(),
    };
    Diagnostic::new(location, error)
}

/// A collection holding only `diagnostic`.
fn single(diagnostic: Diagnostic) -> Diagnostics {
    let mut refusals = Diagnostics::new();
    refusals.push(diagnostic);
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{FakeGit, FakeWorkspace, GitCall, WorkspaceCall};
    use crate::types::name::PackName;
    use crate::types::project::{CommitId, GitTag, GitUrl};
    use crate::types::resolve::{RemoteTag, UpdateNote, UpdateOutcome};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const CORE_URL: &str = "https://github.com/airsstack/core";
    const LINT_URL: &str = "https://github.com/airsstack/lint";
    const TOOL_URL: &str = "https://github.com/airsstack/claude";
    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    fn tagged(url: &str, tag: &str) -> String {
        format!("git = \"{url}\"\ntag = \"{tag}\"\n")
    }

    fn pinned(url: &str) -> String {
        format!("git = \"{url}\"\nrev = \"{REV}\"\n")
    }

    /// A project with the given `[packs.*]` and `[tools.*]` bodies, each as a name and its lines.
    fn project(packs: &[(&str, String)], tools: &[(&str, String)]) -> String {
        let packs = packs
            .iter()
            .map(|(name, body)| format!("[packs.{name}]\n{body}"));
        let tools = tools
            .iter()
            .map(|(name, body)| format!("[tools.{name}]\n{body}"));
        std::iter::once("api = 0\n".to_owned())
            .chain(packs)
            .chain(tools)
            .collect()
    }

    fn url(text: &str) -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(text)?)
    }

    fn remote(tag: &str) -> Result<RemoteTag, Box<dyn std::error::Error>> {
        Ok(RemoteTag {
            rev: CommitId::new(REV)?,
            tag: GitTag::new(tag)?,
        })
    }

    fn remotes(tags: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        tags.iter().map(|tag| remote(tag)).collect()
    }

    fn core() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("core")?))
    }

    fn refusals(
        result: Result<Vec<UpdateEntry>, Diagnostics>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let diagnostics = result.err().ok_or("update succeeded")?;
        Ok(diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    fn entries(
        result: Result<Vec<UpdateEntry>, Diagnostics>,
    ) -> Result<Vec<UpdateEntry>, Box<dyn std::error::Error>> {
        result.map_err(|d| {
            let shown: Vec<String> = d.into_sorted().iter().map(ToString::to_string).collect();
            format!("update refused: {shown:?}").into()
        })
    }

    fn note(entry: &UpdateEntry) -> Result<String, Box<dyn std::error::Error>> {
        match &entry.outcome {
            UpdateOutcome::Note(note) => Ok(note.to_string()),
            UpdateOutcome::NewTag(tag) => Err(format!("a new tag {tag}").into()),
        }
    }

    #[test]
    fn update_reports_a_missing_project_file() -> TestResult {
        let workspace = FakeWorkspace::new();
        let git = FakeGit::new();
        let missing = workspace.project_text().err().ok_or("a project")?;
        let found = refusals(update(&workspace, &git))?;
        assert_eq!(found, [format!("agentstacks.toml: {missing}")]);
        assert!(git.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_reports_an_unreadable_project_file() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::ProjectText);
        let found = refusals(update(&workspace, &FakeGit::new()))?;
        assert_eq!(
            found,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn update_passes_project_refusals_through() -> TestResult {
        let workspace = FakeWorkspace::new().project("api = 0\n");
        let git = FakeGit::new();
        let found = refusals(update(&workspace, &git))?;
        assert_eq!(found, ["agentstacks.toml: nothing to install"]);
        assert!(git.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_gives_no_entry_for_a_path_source() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", "path = \"packs/core\"\n".to_owned())],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new();
        assert!(entries(update(&workspace, &git))?.is_empty());
        assert!(git.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_notes_a_rev_pin() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", pinned(CORE_URL))],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new();
        let found = entries(update(&workspace, &git))?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, core()?);
        assert_eq!(
            note(&found[0])?,
            format!("agentstacks.toml: packs.core: pinned to rev {REV}, not updated")
        );
        assert!(git.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_notes_a_tag_with_no_version() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "main-latest"))],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new();
        let found = entries(update(&workspace, &git))?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            note(&found[0])?,
            "agentstacks.toml: packs.core: tag \"main-latest\" has no version to update from"
        );
        assert!(git.calls().is_empty());
        Ok(())
    }

    #[test]
    fn update_orders_entries_by_key() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("lint", pinned(LINT_URL)), ("core", pinned(CORE_URL))],
            &[("claude", pinned(TOOL_URL))],
        ));
        let found = entries(update(&workspace, &FakeGit::new()))?;
        let keys: Vec<String> = found.iter().map(|entry| entry.key.to_string()).collect();
        assert_eq!(keys, ["packs.core", "packs.lint", "tools.claude"]);
        Ok(())
    }

    #[test]
    fn update_asks_git_with_the_tag_prefix() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.0"))],
            &[("claude", tagged(TOOL_URL, "x-1.2.3-4.5.6"))],
        ));
        let git = FakeGit::new()
            .tags(&url(CORE_URL)?, Vec::new())
            .tags(&url(TOOL_URL)?, Vec::new());
        entries(update(&workspace, &git))?;
        assert_eq!(
            git.calls(),
            [
                GitCall::ListTags {
                    url: url(CORE_URL)?,
                    prefix: "packs-v".to_owned()
                },
                GitCall::ListTags {
                    url: url(TOOL_URL)?,
                    prefix: "x-".to_owned()
                },
            ]
        );
        Ok(())
    }

    fn one_core(
        tag: &str,
        listed: &[&str],
    ) -> Result<Vec<UpdateEntry>, Box<dyn std::error::Error>> {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, tag))],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new().tags(&url(CORE_URL)?, remotes(listed)?);
        entries(update(&workspace, &git))
    }

    #[test]
    fn update_offers_the_highest_released_tag() -> TestResult {
        let found = one_core(
            "packs-v0.1.0",
            &[
                "packs-v0.1.0",
                "packs-v0.10.0",
                "packs-v0.2.0",
                "packs-v1.0.0-rc.1",
                "packs-vnext",
            ],
        )?;
        assert_eq!(
            found,
            [UpdateEntry {
                key: core()?,
                current: Some(GitTag::new("packs-v0.1.0")?),
                outcome: UpdateOutcome::NewTag(GitTag::new("packs-v0.10.0")?),
            }]
        );
        Ok(())
    }

    #[test]
    fn update_notes_no_newer_tag() -> TestResult {
        let found = one_core("packs-v0.1.0", &["packs-v0.1.0"])?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].outcome,
            UpdateOutcome::Note(UpdateNote::NoNewer {
                key: core()?,
                tag: GitTag::new("packs-v0.1.0")?,
            })
        );
        assert_eq!(
            note(&found[0])?,
            "agentstacks.toml: packs.core: no newer tag than packs-v0.1.0"
        );
        Ok(())
    }

    #[test]
    fn update_notes_no_newer_tag_when_nothing_matches() -> TestResult {
        let found = one_core("packs-v0.1.0", &[])?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            note(&found[0])?,
            "agentstacks.toml: packs.core: no newer tag than packs-v0.1.0"
        );
        Ok(())
    }

    #[test]
    fn update_leaves_the_project_untouched() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.0"))],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new().tags(&url(CORE_URL)?, remotes(&["packs-v0.2.0"])?);
        let found = entries(update(&workspace, &git))?;
        assert!(matches!(found[0].outcome, UpdateOutcome::NewTag(_)));
        assert_eq!(workspace.calls(), [WorkspaceCall::ProjectText]);
        Ok(())
    }

    #[test]
    fn update_passes_a_git_failure_through() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.0"))],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let stderr =
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled";
        let git = FakeGit::new().fail(
            &url(CORE_URL)?,
            FetchError::GitFailed {
                subcommand: "ls-remote".to_owned(),
                stderr: stderr.to_owned(),
            },
        );
        assert_eq!(
            refusals(update(&workspace, &git))?,
            [format!(
                "agentstacks.toml: packs.core: git ls-remote failed: {stderr}"
            )]
        );
        Ok(())
    }

    #[test]
    fn update_passes_a_git_spawn_failure_and_unreadable_output_through() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[
                ("core", tagged(CORE_URL, "packs-v0.1.0")),
                ("lint", tagged(LINT_URL, "packs-v0.1.0")),
            ],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let git = FakeGit::new()
            .fail(
                &url(CORE_URL)?,
                FetchError::GitSpawn(FakeWorkspace::denied()),
            )
            .fail(
                &url(LINT_URL)?,
                FetchError::GitUnreadable {
                    subcommand: "ls-remote".to_owned(),
                    first_line: "garbage".to_owned(),
                },
            );
        assert_eq!(
            refusals(update(&workspace, &git))?,
            [
                format!("agentstacks.toml: packs.core: git: {}", FakeWorkspace::denied()),
                "agentstacks.toml: packs.lint: git ls-remote printed output agentstacks cannot read: \"garbage\"".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn update_reports_missing_git_once() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.0"))],
            &[("claude", tagged(TOOL_URL, "x-1.2.3"))],
        ));
        let git = FakeGit::new().git_missing();
        assert_eq!(
            refusals(update(&workspace, &git))?,
            ["agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)"]
        );
        Ok(())
    }

    #[test]
    fn update_reports_missing_git_only_for_sources_that_asked() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.0"))],
            &[("claude", pinned(TOOL_URL))],
        ));
        let git = FakeGit::new().git_missing();
        assert_eq!(
            refusals(update(&workspace, &git))?,
            ["agentstacks.toml: git not found on PATH (needed by packs.core)"]
        );
        Ok(())
    }

    #[test]
    fn update_collects_every_git_failure() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[
                ("core", tagged(CORE_URL, "packs-v0.1.0")),
                ("lint", tagged(LINT_URL, "packs-v0.1.0")),
            ],
            &[("claude", "path = \"extensions/claude\"\n".to_owned())],
        ));
        let failed = |stderr: &str| FetchError::GitFailed {
            subcommand: "ls-remote".to_owned(),
            stderr: stderr.to_owned(),
        };
        let git = FakeGit::new()
            .fail(&url(CORE_URL)?, failed("first"))
            .fail(&url(LINT_URL)?, failed("second"));
        assert_eq!(
            refusals(update(&workspace, &git))?,
            [
                "agentstacks.toml: packs.core: git ls-remote failed: first",
                "agentstacks.toml: packs.lint: git ls-remote failed: second",
            ]
        );
        Ok(())
    }

    #[test]
    fn update_names_a_tool_in_its_refusal() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", pinned(CORE_URL))],
            &[("claude", tagged(TOOL_URL, "x-1.2.3"))],
        ));
        let git = FakeGit::new().fail(
            &url(TOOL_URL)?,
            FetchError::GitFailed {
                subcommand: "ls-remote".to_owned(),
                stderr: "no".to_owned(),
            },
        );
        assert_eq!(
            refusals(update(&workspace, &git))?,
            ["agentstacks.toml: tools.claude: git ls-remote failed: no"]
        );
        Ok(())
    }
}
