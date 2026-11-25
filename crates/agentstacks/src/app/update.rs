//! The flow that finds newer tags for the packs and extensions taken from git sources, and whether
//! the project file still speaks the legacy `api`.
//!
//! It exists so a user can see, without changing anything, which git source of `agentstacks.toml`
//! has a newer released tag and whether the file's `api` has to move, and to put both into the file
//! once the user accepts them.
//!
//! # Responsibilities
//!
//! - [`update`] reads and parses `agentstacks.toml` (`api` 0 or the current one), visits every
//!   source in key order (`packs.*`, then `tools.*`), asks the update rules what each source needs,
//!   lists the tags of the git sources that need them and hands the tags back to the rules. It
//!   returns an [`Update`]: one entry per git source and whether the legacy `api` moves, or every
//!   refusal: git's failures located at their source, and one refusal for git missing from `PATH`
//!   that names every source that needed it.
//! - [`rewrite`] writes an [`Update`] into `agentstacks.toml`: it reads the file again, checks it,
//!   hands every newer tag to [`retag_project`], moves a legacy `api` with [`upgrade_project`] on
//!   the same text and writes it once. `update` itself never writes.
//!
//! # Non-responsibilities
//!
//! No rule: how a tag is split, which tag is newest, the words of every note, and how a tag or the
//! `api` is replaced in the file belong to [`UpdateQuery`],
//! [`UpdateNote`](crate::types::resolve::UpdateNote), [`retag_project`] and [`upgrade_project`].
//! No shorthand expansion.

use std::path::Path;

use crate::port::{Git, Workspace};
use crate::types::api::ApiCheck;
use crate::types::project::{PROJECT_FILE, ProjectFile, retag_project, upgrade_project};
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::{FetchError, SourceKey, UpdateEntry, UpdateOutcome, UpdateQuery};

/// What `update` found: the newest released tag for each git source, and whether the file's `api`
/// has to move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// The `api` `agentstacks.toml` declares: [`ApiCheck::Legacy`] when `rewrite` will set the
    /// current one, [`ApiCheck::Current`] otherwise. It is never [`ApiCheck::Unsupported`],
    /// because `update` refuses a project file with one.
    pub api: ApiCheck,
    /// One entry per git source in key order, `packs.*` then `tools.*`, and none for a `path`
    /// source. An entry says what the source is pinned to now and either the newer tag or why
    /// there is none.
    pub entries: Vec<UpdateEntry>,
}

/// The newest released tag for each git source of `agentstacks.toml`, and whether its `api` moves.
///
/// The entries are one per git source in key order, `packs.*` then `tools.*`, and none for a `path`
/// source. An entry says what the source is pinned to now and either the newer tag or why there is
/// none. `api` is [`ApiCheck::Legacy`] for a file written for the `0.x` releases.
///
/// # Errors
///
/// Returns every refusal, sorted, and nothing else, when the project file cannot be read or is
/// refused, or when listing the tags of a source fails. A failure of git is located at the
/// source's key; git missing from `PATH` is one refusal at the file that names every source that
/// asked for it.
pub fn update<W: Workspace, G: Git>(workspace: &W, git: &G) -> Result<Update, Diagnostics> {
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
    refusals.into_result(Update {
        api: project.api(),
        entries,
    })
}

/// Writes the newer tags of `found`, and the current `api` when it moves, into `agentstacks.toml`,
/// once.
///
/// Returns `Ok(false)` and writes nothing when no entry carries a newer tag and the `api` does not
/// move. Otherwise the file is read again and checked, every `(key, current tag, new tag)` is
/// applied, then the `api` is set, all in one edit that keeps the comments, and the text is
/// written; `Ok(true)` says it was.
///
/// # Errors
///
/// Returns the refusal, with nothing written, when the file cannot be read or no longer parses as a
/// project file, when an entry has no tag now or a tag other than the one `update` listed, when
/// the `api` is no longer the legacy one (the file was edited since), or when the write fails. A
/// read or write failure is located at the file.
pub fn rewrite<W: Workspace>(workspace: &W, found: &Update) -> Result<bool, Diagnostics> {
    let changes: Vec<_> = found
        .entries
        .iter()
        .filter_map(|entry| match (&entry.outcome, &entry.current) {
            (UpdateOutcome::NewTag(new), Some(current)) => {
                Some((entry.key.clone(), current.clone(), new.clone()))
            }
            _ => None,
        })
        .collect();
    let api_moves = found.api == ApiCheck::Legacy;
    if changes.is_empty() && !api_moves {
        return Ok(false);
    }
    let at_file =
        |error: std::io::Error| single(Diagnostic::new(Location::file(PROJECT_FILE), error));
    let text = workspace.project_text().map_err(at_file)?;
    ProjectFile::parse(Path::new(PROJECT_FILE), &text)?;
    let mut edited = retag_project(&text, &changes).map_err(single)?;
    if api_moves {
        edited = upgrade_project(&edited).map_err(single)?;
    }
    workspace.write_project(&edited).map_err(at_file)?;
    Ok(true)
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
        std::iter::once("api = 1\n".to_owned())
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
        result: Result<Update, Diagnostics>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let diagnostics = result.err().ok_or("update succeeded")?;
        Ok(diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }

    fn found(result: Result<Update, Diagnostics>) -> Result<Update, Box<dyn std::error::Error>> {
        result.map_err(|d| {
            let shown: Vec<String> = d.into_sorted().iter().map(ToString::to_string).collect();
            format!("update refused: {shown:?}").into()
        })
    }

    fn entries(
        result: Result<Update, Diagnostics>,
    ) -> Result<Vec<UpdateEntry>, Box<dyn std::error::Error>> {
        Ok(found(result)?.entries)
    }

    /// An update of a project at the current `api` that holds `entries`.
    fn at_current(entries: &[UpdateEntry]) -> Update {
        Update {
            api: ApiCheck::Current,
            entries: entries.to_vec(),
        }
    }

    fn legacy(text: &str) -> String {
        text.replacen("api = 1", "api = 0", 1)
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
        let workspace = FakeWorkspace::new().project("api = 1\n");
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

    fn newer(
        key: SourceKey,
        current: &str,
        new: &str,
    ) -> Result<UpdateEntry, Box<dyn std::error::Error>> {
        Ok(UpdateEntry {
            key,
            current: Some(GitTag::new(current)?),
            outcome: UpdateOutcome::NewTag(GitTag::new(new)?),
        })
    }

    /// The tool the rewrite tests list beside their packs, a project needing one of each.
    fn claude_path() -> (&'static str, String) {
        ("claude", "path = \"extensions/claude\"\n".to_owned())
    }

    fn lint() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("lint")?))
    }

    fn two_packs() -> String {
        project(
            &[
                ("core", tagged(CORE_URL, "packs-v0.1.0")),
                ("lint", tagged(LINT_URL, "packs-v0.1.0")),
            ],
            &[claude_path()],
        )
    }

    #[test]
    fn rewrite_writes_every_newer_tag_once() -> TestResult {
        let workspace = FakeWorkspace::new().project(&two_packs());
        let entries = [
            newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?,
            newer(lint()?, "packs-v0.1.0", "packs-v0.3.0")?,
        ];
        assert!(rewrite(&workspace, &at_current(&entries)).map_err(|d| format!("{d:?}"))?);
        let written = workspace.project_written().ok_or("a write")?;
        assert_eq!(
            written,
            two_packs()
                .replacen("packs-v0.1.0", "packs-v0.2.0", 1)
                .replacen("packs-v0.1.0", "packs-v0.3.0", 1)
        );
        let writes = workspace
            .calls()
            .iter()
            .filter(|call| **call == WorkspaceCall::WriteProject)
            .count();
        assert_eq!(writes, 1);
        Ok(())
    }

    #[test]
    fn rewrite_with_no_newer_tag_writes_nothing() -> TestResult {
        let workspace = FakeWorkspace::new().project(&two_packs());
        let no_newer = UpdateEntry {
            key: core()?,
            current: Some(GitTag::new("packs-v0.1.0")?),
            outcome: UpdateOutcome::Note(UpdateNote::NoNewer {
                key: core()?,
                tag: GitTag::new("packs-v0.1.0")?,
            }),
        };
        assert!(!rewrite(&workspace, &at_current(&[no_newer])).map_err(|d| format!("{d:?}"))?);
        assert!(!rewrite(&workspace, &at_current(&[])).map_err(|d| format!("{d:?}"))?);
        assert_eq!(workspace.project_written(), None);
        assert!(workspace.calls().is_empty());
        Ok(())
    }

    #[test]
    fn rewrite_refuses_text_that_no_longer_parses() -> TestResult {
        let workspace = FakeWorkspace::new().project("[packs.core\n");
        let entries = [newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?];
        let shown = refused(rewrite(&workspace, &at_current(&entries)))?;
        assert_eq!(shown, ["agentstacks.toml:1: unclosed table, expected `]`"]);
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn rewrite_refuses_a_tag_changed_at_re_read() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("core", tagged(CORE_URL, "packs-v0.1.5"))],
            &[claude_path()],
        ));
        let entries = [newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?];
        assert_eq!(
            refused(rewrite(&workspace, &at_current(&entries)))?,
            ["agentstacks.toml: packs.core: tag changed during update (run update again)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn rewrite_reports_a_failed_write() -> TestResult {
        let workspace = FakeWorkspace::new()
            .project(&two_packs())
            .fail(WorkspaceCall::WriteProject);
        let entries = [newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?];
        assert_eq!(
            refused(rewrite(&workspace, &at_current(&entries)))?,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn rewrite_reports_a_failed_re_read() -> TestResult {
        let workspace = FakeWorkspace::new()
            .project(&two_packs())
            .fail(WorkspaceCall::ProjectText);
        let entries = [newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?];
        assert_eq!(
            refused(rewrite(&workspace, &at_current(&entries)))?,
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn rewrite_refuses_a_key_missing_at_re_read() -> TestResult {
        let workspace = FakeWorkspace::new().project(&project(
            &[("lint", tagged(LINT_URL, "packs-v0.1.0"))],
            &[claude_path()],
        ));
        let entries = [newer(core()?, "packs-v0.1.0", "packs-v0.2.0")?];
        assert_eq!(
            refused(rewrite(&workspace, &at_current(&entries)))?,
            ["agentstacks.toml: packs.core: no tag to update (run update again)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn legacy_project_upgrades_with_no_newer_tag() -> TestResult {
        let text = legacy(&project(
            &[("core", "path = \"packs/core\"\n".to_owned())],
            &[claude_path()],
        ));
        let workspace = FakeWorkspace::new().project(&text);
        let result = found(update(&workspace, &FakeGit::new()))?;
        assert_eq!(result.api, ApiCheck::Legacy);
        assert!(result.entries.is_empty());
        assert!(rewrite(&workspace, &result).map_err(|d| format!("{d:?}"))?);
        assert_eq!(
            workspace.project_written(),
            Some(text.replacen("api = 0", "api = 1", 1))
        );
        Ok(())
    }

    #[test]
    fn legacy_and_new_tags_rewrite_both() -> TestResult {
        let text = legacy(&two_packs());
        let workspace = FakeWorkspace::new().project(&text);
        let git = FakeGit::new()
            .tags(&url(CORE_URL)?, remotes(&["packs-v0.1.0", "packs-v0.2.0"])?)
            .tags(&url(LINT_URL)?, remotes(&["packs-v0.1.0"])?);
        let result = found(update(&workspace, &git))?;
        assert_eq!(result.api, ApiCheck::Legacy);
        assert!(rewrite(&workspace, &result).map_err(|d| format!("{d:?}"))?);
        assert_eq!(
            workspace.project_written(),
            Some(text.replacen("api = 0", "api = 1", 1).replacen(
                "packs-v0.1.0",
                "packs-v0.2.0",
                1
            ))
        );
        let writes = workspace
            .calls()
            .iter()
            .filter(|call| **call == WorkspaceCall::WriteProject)
            .count();
        assert_eq!(writes, 1);
        Ok(())
    }

    #[test]
    fn current_project_unchanged_behaviour() -> TestResult {
        let workspace = FakeWorkspace::new().project(&two_packs());
        let git = FakeGit::new()
            .tags(&url(CORE_URL)?, remotes(&["packs-v0.1.0"])?)
            .tags(&url(LINT_URL)?, remotes(&["packs-v0.1.0"])?);
        let result = found(update(&workspace, &git))?;
        assert_eq!(result.api, ApiCheck::Current);
        assert!(!rewrite(&workspace, &result).map_err(|d| format!("{d:?}"))?);
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    #[test]
    fn api_edited_between_read_and_rewrite() -> TestResult {
        let workspace = FakeWorkspace::new().project(&two_packs());
        let result = Update {
            api: ApiCheck::Legacy,
            entries: Vec::new(),
        };
        assert_eq!(
            refused(rewrite(&workspace, &result))?,
            ["agentstacks.toml: api changed during update (run update again)"]
        );
        assert_eq!(workspace.project_written(), None);
        Ok(())
    }

    fn refused(
        result: Result<bool, Diagnostics>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(result
            .err()
            .ok_or("rewrite succeeded")?
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect())
    }
}
