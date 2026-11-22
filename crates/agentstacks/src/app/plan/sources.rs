//! Steps 2 and 3 of the plan flow: reconciling the lock with the project, then fetching every
//! source it names.
//!
//! # Responsibilities
//!
//! - [`fetch_sources`] classifies every entry with `reconcile` and visits them in `SourceKey`
//!   order, packs first and then tools, each in name order. A new or stale entry resolves its pin;
//!   a locked entry is fetched at the lock's revision and never lists or fetches a tag; a removed
//!   tool is fetched at its locked revision so its extension can run; a removed pack is not
//!   fetched. A git tree that differs from the lock is refused through `check_tree`; a changed
//!   path tree is a normal change.
//! - [`Fetched`] is the flow's carrier for what the visit found: for each key either its [`Entry`]
//!   or that key's refusals, the notes for entries resolved again, and the refusals that are
//!   reported once however many sources met them.
//! - [`Tree`] lets one tree type stand for a git source and a local source.
//! - Each [`FetchError`] is placed here, because the git port returns it with no location.
//!
//! # Non-responsibilities
//!
//! Which entry is stale and whether a tree matches the lock are rules in `types`. Running `git`,
//! keeping the cache and reading a directory belong to the adapters behind the ports.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::app::plan::read::{Inputs, PROJECT_FILE};
use crate::port::{EntryKind, FileTree, Git, Workspace};
use crate::types::lock::LockedSource;
use crate::types::plan::{Note, StepError};
use crate::types::project::{CommitId, Pin, Source};
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::{FetchError, Reconciled, SourceKey, check_tree, reconcile};
use crate::types::tree::{ContentHash, tree_hash};

/// A tree read from a git source or from a directory of the project.
pub(super) enum Tree<A, B> {
    /// The files of a commit.
    Git(A),
    /// The files of a project directory.
    Local(B),
}

impl<A: FileTree, B: FileTree> FileTree for Tree<A, B> {
    fn root(&self) -> &Path {
        match self {
            Self::Git(tree) => tree.root(),
            Self::Local(tree) => tree.root(),
        }
    }

    fn kind(&self, rel: &Path) -> io::Result<EntryKind> {
        match self {
            Self::Git(tree) => tree.kind(rel),
            Self::Local(tree) => tree.kind(rel),
        }
    }

    fn read_dir(&self, rel: &Path) -> io::Result<Vec<std::ffi::OsString>> {
        match self {
            Self::Git(tree) => tree.read_dir(rel),
            Self::Local(tree) => tree.read_dir(rel),
        }
    }

    fn read_file(&self, rel: &Path) -> io::Result<Vec<u8>> {
        match self {
            Self::Git(tree) => tree.read_file(rel),
            Self::Local(tree) => tree.read_file(rel),
        }
    }
}

/// One source that was fetched.
pub(super) struct Entry<T> {
    /// The files of the source.
    pub(super) tree: T,
    /// Where the source resolved to: the commit of a git source, or the directory.
    pub(super) source: LockedSource,
    /// The hash of the tree; `None` when a file of it could not be read, which loading the tree
    /// refuses in its own words.
    pub(super) hash: Option<ContentHash>,
}

/// What fetching every source found.
pub(super) struct Fetched<T> {
    /// Each source's entry, or the refusals of fetching it, by key in `SourceKey` order. A tool
    /// the project dropped is here too, because its extension runs once more. A removed pack is
    /// not.
    pub(super) entries: BTreeMap<SourceKey, Result<Entry<T>, Diagnostics>>,
    /// One note for each entry whose source changed and was resolved again.
    pub(super) notes: Vec<Note>,
    /// The refusals reported once however many sources met them: git missing and `HOME` unset.
    pub(super) shared: Diagnostics,
}

/// The refusals that are reported once, gathered while the sources are visited.
#[derive(Default)]
struct Once {
    /// The keys of the sources that found git missing, in visiting order.
    git_missing: Vec<String>,
    /// Whether any source found `HOME` unset.
    home_unset: bool,
}

impl Once {
    fn into_diagnostics(self) -> Diagnostics {
        let mut found = Diagnostics::new();
        if self.home_unset {
            found.push(Diagnostic::new(
                Location::file(PROJECT_FILE),
                FetchError::HomeUnset,
            ));
        }
        if !self.git_missing.is_empty() {
            found.push(Diagnostic::new(
                Location::file(PROJECT_FILE),
                FetchError::GitNotFound {
                    needed_by: self.git_missing,
                },
            ));
        }
        found
    }
}

/// Why one source could not be fetched.
enum Failure {
    /// The git port refused.
    Fetch(FetchError),
    /// The fetched tree differs from the lock's.
    Mismatch(Diagnostic),
    /// The directory of a path source leads outside the project.
    Outside,
    /// Looking at the directory of a path source failed.
    Io(io::Error),
}

/// Reconciles the lock with the project and fetches every source that is to run.
pub(super) fn fetch_sources<W: Workspace, G: Git>(
    inputs: &Inputs,
    workspace: &W,
    git: &G,
) -> Fetched<Tree<G::Tree, W::Tree>> {
    let mut fetched = Fetched {
        entries: BTreeMap::new(),
        notes: Vec::new(),
        shared: Diagnostics::new(),
    };
    let mut once = Once::default();
    for (key, class) in reconcile(&inputs.project, inputs.lock.as_ref()) {
        let Some(plan) = Visit::of(inputs, &key, class) else {
            continue;
        };
        if plan.stale {
            fetched.notes.push(Note::SourceChanged {
                location: key.location(),
            });
        }
        let result = plan
            .fetch(&key, workspace, git)
            .map_err(|failure| place(failure, &key, &mut once));
        fetched.entries.insert(key, result);
    }
    fetched.shared = once.into_diagnostics();
    fetched
}

/// The outcome of fetching one source over the ports `W` and `G`.
type Fetch<W, G> = Result<Entry<Tree<<G as Git>::Tree, <W as Workspace>::Tree>>, Failure>;

/// What to do for one entry.
struct Visit {
    /// The source to fetch.
    source: Source,
    /// The locked commit to fetch at instead of resolving the pin.
    at_rev: Option<CommitId>,
    /// The locked source and tree the fetched tree is checked against.
    lock: Option<(LockedSource, ContentHash)>,
    /// Whether the entry's source differs from the lock's.
    stale: bool,
}

impl Visit {
    /// The visit for an entry of class `class`; `None` for an entry that is not fetched.
    fn of(inputs: &Inputs, key: &SourceKey, class: Reconciled) -> Option<Self> {
        match class {
            Reconciled::RemovedPack => None,
            Reconciled::RemovedTool { source, tree } => Some(Self {
                source: source.to_source(),
                at_rev: None,
                lock: Some((source, tree)),
                stale: false,
            }),
            Reconciled::New => Some(Self::resolved(inputs, key, false)?),
            Reconciled::Stale => Some(Self::resolved(inputs, key, true)?),
            Reconciled::Locked { rev, tree } => {
                let mut visit = Self::resolved(inputs, key, false)?;
                visit.lock = locked_source(inputs, key).map(|source| (source, tree));
                visit.at_rev = rev;
                Some(visit)
            }
        }
    }

    /// A visit that resolves the source the project lists for `key`.
    fn resolved(inputs: &Inputs, key: &SourceKey, stale: bool) -> Option<Self> {
        let source = match key {
            SourceKey::Pack(name) => inputs.project.packs().get(name)?.source().clone(),
            SourceKey::Tool(name) => inputs.project.tools().get(name)?.clone(),
        };
        Some(Self {
            source,
            at_rev: None,
            lock: None,
            stale,
        })
    }

    /// Fetches the source and checks its tree against the lock.
    fn fetch<W: Workspace, G: Git>(self, key: &SourceKey, workspace: &W, git: &G) -> Fetch<W, G> {
        let (tree, source) = match &self.source {
            Source::Git { url, pin, subdir } => {
                let rev = match (&self.at_rev, pin) {
                    (Some(rev), _) | (None, Pin::Rev(rev)) => rev.clone(),
                    (None, Pin::Tag(tag)) => git.fetch_tag(url, tag).map_err(Failure::Fetch)?,
                };
                let tree = git
                    .tree(url, &rev, subdir.as_ref(), pin)
                    .map_err(Failure::Fetch)?;
                let tag = match pin {
                    Pin::Tag(tag) => Some(tag.clone()),
                    Pin::Rev(_) => None,
                };
                let source = LockedSource::Git {
                    url: url.clone(),
                    tag,
                    subdir: subdir.clone(),
                    rev,
                };
                (Tree::Git(tree), source)
            }
            Source::Path(dir) => {
                let tree = workspace
                    .source_tree(dir)
                    .map_err(Failure::Io)?
                    .ok_or(Failure::Outside)?;
                (Tree::Local(tree), LockedSource::Path(dir.clone()))
            }
        };
        let hash = tree_hash(&tree).ok();
        if let (Some((locked, lock_tree)), Some(found)) = (&self.lock, &hash) {
            check_tree(key, locked, found, lock_tree).map_err(Failure::Mismatch)?;
        }
        Ok(Entry { tree, source, hash })
    }
}

/// The source the lock recorded for `key`.
fn locked_source(inputs: &Inputs, key: &SourceKey) -> Option<LockedSource> {
    let lock = inputs.lock.as_ref()?;
    match key {
        SourceKey::Pack(name) => lock
            .packs()
            .iter()
            .find(|entry| entry.name() == name)
            .map(|entry| entry.source().clone()),
        SourceKey::Tool(name) => lock
            .tools()
            .iter()
            .find(|entry| entry.name() == name)
            .map(|entry| entry.source().clone()),
    }
}

/// Puts a failure at the entry it belongs to. A failure reported once is recorded in `once` and
/// leaves the entry with no refusal of its own.
fn place(failure: Failure, key: &SourceKey, once: &mut Once) -> Diagnostics {
    let diagnostic = match failure {
        Failure::Mismatch(diagnostic) => Some(diagnostic),
        Failure::Outside => Some(Diagnostic::new(
            Location::at_key(PROJECT_FILE, format!("{key}.path")),
            StepError::Outside,
        )),
        Failure::Io(error) => Some(Diagnostic::new(
            Location::at_key(PROJECT_FILE, format!("{key}.path")),
            error,
        )),
        Failure::Fetch(FetchError::GitNotFound { .. }) => {
            once.git_missing.push(key.to_string());
            None
        }
        Failure::Fetch(FetchError::HomeUnset) => {
            once.home_unset = true;
            None
        }
        Failure::Fetch(FetchError::CacheIo { path, error }) => {
            Some(Diagnostic::new(Location::file(path), error))
        }
        Failure::Fetch(
            error @ (FetchError::SubdirNotFound { .. } | FetchError::SubdirIsFile { .. }),
        ) => Some(Diagnostic::new(key.subdir_location(), error)),
        Failure::Fetch(error) => Some(Diagnostic::new(key.location(), error)),
    };
    let mut found = Diagnostics::new();
    found.extend(diagnostic);
    found
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::plan::read::read_inputs;
    use crate::app::plan::testing::{
        REV_A, REV_B, Scenario, TestResult, URL, files, git_locked, git_tag, hash_of, local,
        lock_text, locked_pack, locked_tool, pack, path_locked, rev, shown, sub, tool, url,
    };
    use crate::port::{FakeFileTree, FakeGit, FakeWorkspace, GitCall};
    use crate::types::lock::{LockContents, LockedEntry};
    use crate::types::name::{PackName, ToolName};
    use crate::types::resolve::{TreeRecord, check_subdir};

    type TestFetched = Fetched<Tree<FakeFileTree, FakeFileTree>>;

    const EXT_URL: &str = "https://example.com/org/ext";
    const PATH_TOOL: &str = "path = \"extensions/claude\"";
    const PATH_PACK: &str = "path = \"packs/core\"";

    fn project(tool_source: &str, pack_source: &str) -> String {
        format!("api = 0\n\n[tools.claude]\n{tool_source}\n\n[packs.core]\n{pack_source}\n")
    }

    fn git_pack(tag: &str) -> String {
        format!("git = \"{URL}\"\ntag = \"{tag}\"")
    }

    fn fetch(scenario: &Scenario) -> Result<TestFetched, Box<dyn std::error::Error>> {
        let inputs = read_inputs(&scenario.workspace).map_err(|found| shown(found).join("; "))?;
        Ok(fetch_sources(&inputs, &scenario.workspace, &scenario.git))
    }

    fn with_tool_tree(scenario: Scenario) -> Result<Scenario, Box<dyn std::error::Error>> {
        scenario.path_source(
            "extensions/claude",
            files("extensions/claude", &[("extension.toml", "x")]),
        )
    }

    fn pack_key() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(pack("core")?))
    }

    fn entry_of<'a>(
        fetched: &'a TestFetched,
        key: &SourceKey,
    ) -> Result<&'a Entry<Tree<FakeFileTree, FakeFileTree>>, Box<dyn std::error::Error>> {
        match fetched.entries.get(key) {
            Some(Ok(entry)) => Ok(entry),
            Some(Err(found)) => Err(format!("{found:?}").into()),
            None => Err("no entry".into()),
        }
    }

    /// Every refusal the fetch raised, entries and shared alike.
    fn refused(fetched: TestFetched) -> Vec<String> {
        let mut all = fetched.shared;
        for (_, result) in fetched.entries {
            if let Err(found) = result {
                all.extend(found);
            }
        }
        shown(all)
    }

    fn lock_with(
        packs: Vec<LockedEntry<PackName>>,
        tools: Vec<LockedEntry<ToolName>>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        lock_text(LockContents {
            tools,
            packs,
            ..LockContents::default()
        })
    }

    #[test]
    fn fetch_sources_resolves_a_new_git_tag_to_its_rev() -> TestResult {
        let snapshot = files("packs/core", &[("pack.toml", "a")]);
        let (url, tag, at) = (url()?, git_tag("packs-v0.1.0")?, rev(REV_A)?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v0.1.0"))).git(|git| {
                git.tag(&url, &tag, &at)
                    .files(&url, &at, None, snapshot.clone())
            }),
        )?;
        let fetched = fetch(&scenario)?;
        assert_eq!(
            scenario.git.calls(),
            [
                GitCall::FetchTag {
                    url: url.clone(),
                    tag
                },
                GitCall::Tree {
                    url,
                    rev: at,
                    subdir: None
                },
            ]
        );
        let entry = entry_of(&fetched, &pack_key()?)?;
        assert_eq!(entry.source, git_locked(REV_A, Some("packs-v0.1.0"), None)?);
        assert_eq!(entry.hash, Some(hash_of(&snapshot)?));
        Ok(())
    }

    #[test]
    fn fetch_sources_keeps_the_locked_rev_when_the_tag_moved() -> TestResult {
        let snapshot = files("packs/core", &[("pack.toml", "a")]);
        let lock = lock_with(
            vec![locked_pack(
                "core",
                git_locked(REV_A, Some("packs-v0.1.0"), None)?,
                hash_of(&snapshot)?,
            )?],
            vec![],
        )?;
        let (url, tag, old, moved) = (url()?, git_tag("packs-v0.1.0")?, rev(REV_A)?, rev(REV_B)?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v0.1.0")))
                .lock(&lock)
                .git(|git| {
                    git.tag(&url, &tag, &moved)
                        .files(&url, &old, None, snapshot.clone())
                }),
        )?;
        let fetched = fetch(&scenario)?;
        assert_eq!(
            scenario.git.calls(),
            [GitCall::Tree {
                url,
                rev: old,
                subdir: None
            }]
        );
        assert_eq!(
            entry_of(&fetched, &pack_key()?)?.source,
            git_locked(REV_A, Some("packs-v0.1.0"), None)?
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_resolves_a_stale_entry_again_with_a_note() -> TestResult {
        let snapshot = files("packs/core", &[("pack.toml", "a")]);
        let lock = lock_with(
            vec![locked_pack(
                "core",
                git_locked(REV_A, Some("packs-v0.1.0"), None)?,
                hash_of(&snapshot)?,
            )?],
            vec![],
        )?;
        let (url, tag, newer) = (url()?, git_tag("packs-v0.2.0")?, rev(REV_B)?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v0.2.0")))
                .lock(&lock)
                .git(|git| {
                    git.tag(&url, &tag, &newer)
                        .files(&url, &newer, None, snapshot.clone())
                }),
        )?;
        let fetched = fetch(&scenario)?;
        let notes: Vec<String> = fetched.notes.iter().map(ToString::to_string).collect();
        assert_eq!(
            notes,
            ["agentstacks.toml: packs.core: source changed, resolved again"]
        );
        assert_eq!(
            entry_of(&fetched, &pack_key()?)?.source,
            git_locked(REV_B, Some("packs-v0.2.0"), None)?
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_refuses_a_locked_git_tree_that_differs() -> TestResult {
        let locked = files("packs/core", &[("pack.toml", "a")]);
        let served = files("packs/core", &[("pack.toml", "other")]);
        let lock = lock_with(
            vec![locked_pack(
                "core",
                git_locked(REV_A, Some("packs-v0.1.0"), None)?,
                hash_of(&locked)?,
            )?],
            vec![],
        )?;
        let (url, at) = (url()?, rev(REV_A)?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v0.1.0")))
                .lock(&lock)
                .git(|git| git.files(&url, &at, None, served.clone())),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [format!(
                "agentstacks.toml: packs.core: tree {} at rev {REV_A} differs from the lock's {}",
                hash_of(&served)?,
                hash_of(&locked)?
            )]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_accepts_a_changed_path_tree() -> TestResult {
        let old = files("packs/core", &[("pack.toml", "a")]);
        let new = files("packs/core", &[("pack.toml", "changed")]);
        let lock = lock_with(
            vec![locked_pack(
                "core",
                path_locked("packs/core")?,
                hash_of(&old)?,
            )?],
            vec![],
        )?;
        let scenario = with_tool_tree(Scenario::new(&project(PATH_TOOL, PATH_PACK)).lock(&lock))?
            .path_source("packs/core", new.clone())?;
        let fetched = fetch(&scenario)?;
        assert_eq!(entry_of(&fetched, &pack_key()?)?.hash, Some(hash_of(&new)?));
        assert!(refused(fetched).is_empty());
        Ok(())
    }

    #[test]
    fn fetch_sources_refuses_a_path_source_outside_the_project() -> TestResult {
        let text = format!(
            "api = 0\n\n[tools.claude]\n{PATH_TOOL}\n\n[packs.team-lint]\npath = \"packs/team-lint\"\n"
        );
        let outside = local("packs/team-lint")?;
        let scenario = with_tool_tree(Scenario::new(&text))?
            .workspace(|workspace| workspace.source_outside(&outside));
        assert_eq!(
            refused(fetch(&scenario)?),
            ["agentstacks.toml: packs.team-lint.path: resolves outside the project"]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_reports_git_missing_once_for_every_source() -> TestResult {
        let text = project(
            &format!("git = \"{URL}\"\ntag = \"ext-v1\""),
            &git_pack("packs-v1"),
        );
        let scenario = Scenario::new(&text).git(FakeGit::git_missing);
        assert_eq!(
            refused(fetch(&scenario)?),
            ["agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)"]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_reports_home_unset_once() -> TestResult {
        let text = project(
            &format!("git = \"{URL}\"\ntag = \"ext-v1\""),
            &git_pack("packs-v1"),
        );
        let scenario = Scenario::new(&text).git(FakeGit::home_unset);
        assert_eq!(
            refused(fetch(&scenario)?),
            ["agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home"]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_places_git_failed_at_the_key() -> TestResult {
        let url = url()?;
        let error = FetchError::GitFailed {
            subcommand: "fetch".to_owned(),
            stderr: "fatal: couldn't find remote ref refs/tags/packs-v9.9.9".to_owned(),
        };
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v9.9.9")))
                .git(|git| git.fail(&url, error)),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [
                "agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9"
            ]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_places_git_spawn_at_the_key() -> TestResult {
        let ext_url = crate::types::project::GitUrl::new(EXT_URL)?;
        let text = project(&format!("git = \"{EXT_URL}\"\ntag = \"ext-v1\""), PATH_PACK);
        let scenario = Scenario::new(&text)
            .path_source("packs/core", files("packs/core", &[("pack.toml", "a")]))?
            .git(|git| git.fail(&ext_url, FetchError::GitSpawn(FakeWorkspace::denied())));
        assert_eq!(
            refused(fetch(&scenario)?),
            [format!(
                "agentstacks.toml: tools.claude: git: {}",
                FakeWorkspace::denied()
            )]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_places_git_unreadable_at_the_key() -> TestResult {
        let url = url()?;
        let error = FetchError::GitUnreadable {
            subcommand: "rev-parse".to_owned(),
            first_line: "nope".to_owned(),
        };
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v1")))
                .git(|git| git.fail(&url, error)),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [
                "agentstacks.toml: packs.core: git rev-parse printed output agentstacks cannot read: \"nope\""
            ]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_places_subdir_refusals_at_the_subdir_key() -> TestResult {
        let pin = Pin::Tag(git_tag("packs-v0.1.0")?);
        let url = url()?;
        let source = format!("{}\nsubdir = \"packs/nope\"", git_pack("packs-v0.1.0"));
        let not_found = check_subdir(&[], &sub("packs/nope")?, &pin)
            .err()
            .ok_or("a refusal")?;
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &source)).git(|git| git.fail(&url, not_found)),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            ["agentstacks.toml: packs.core.subdir: \"packs/nope\" not found at packs-v0.1.0"]
        );

        let blob = TreeRecord {
            mode: "100644".to_owned(),
            object: "blob".to_owned(),
            oid: "0".repeat(40),
            path: b"packs/core/pack.toml".to_vec(),
        };
        let is_file = check_subdir(&[blob], &sub("packs/core/pack.toml")?, &pin)
            .err()
            .ok_or("a refusal")?;
        let source = format!(
            "{}\nsubdir = \"packs/core/pack.toml\"",
            git_pack("packs-v0.1.0")
        );
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &source)).git(|git| git.fail(&url, is_file)),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [
                "agentstacks.toml: packs.core.subdir: \"packs/core/pack.toml\" is a file, not a directory"
            ]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_places_cache_io_at_its_own_path() -> TestResult {
        let url = url()?;
        let path = "/h/.airsstack/cache/git/abc/def.git";
        let error = FetchError::CacheIo {
            path: PathBuf::from(path),
            error: FakeWorkspace::denied(),
        };
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, &git_pack("packs-v1")))
                .git(|git| git.fail(&url, error)),
        )?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [format!("{path}: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_fetches_a_removed_tool_by_its_locked_rev() -> TestResult {
        let snapshot = files("extensions/opencode", &[("extension.toml", "x")]);
        let lock = lock_with(
            vec![],
            vec![locked_tool(
                "opencode",
                git_locked(REV_A, None, Some("extensions/opencode"))?,
                hash_of(&snapshot)?,
            )?],
        )?;
        let (url, at, subdir) = (url()?, rev(REV_A)?, sub("extensions/opencode")?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, PATH_PACK))
                .lock(&lock)
                .git(|git| git.files(&url, &at, Some(&subdir), snapshot.clone())),
        )?
        .path_source("packs/core", files("packs/core", &[("pack.toml", "a")]))?;
        let fetched = fetch(&scenario)?;
        assert_eq!(
            scenario.git.calls(),
            [GitCall::Tree {
                url,
                rev: at,
                subdir: Some(subdir)
            }]
        );
        assert!(matches!(
            fetched.entries.get(&SourceKey::Tool(tool("opencode")?)),
            Some(Ok(_))
        ));
        Ok(())
    }

    #[test]
    fn fetch_sources_checks_a_removed_tools_tree_against_the_lock() -> TestResult {
        let locked = files("extensions/opencode", &[("extension.toml", "x")]);
        let served = files("extensions/opencode", &[("extension.toml", "other")]);
        let lock = lock_with(
            vec![],
            vec![locked_tool(
                "opencode",
                git_locked(REV_A, None, None)?,
                hash_of(&locked)?,
            )?],
        )?;
        let (url, at) = (url()?, rev(REV_A)?);
        let scenario = with_tool_tree(
            Scenario::new(&project(PATH_TOOL, PATH_PACK))
                .lock(&lock)
                .git(|git| git.files(&url, &at, None, served.clone())),
        )?
        .path_source("packs/core", files("packs/core", &[("pack.toml", "a")]))?;
        assert_eq!(
            refused(fetch(&scenario)?),
            [format!(
                "agentstacks.toml: tools.opencode: tree {} at rev {REV_A} differs from the lock's {}",
                hash_of(&served)?,
                hash_of(&locked)?
            )]
        );
        Ok(())
    }

    #[test]
    fn fetch_sources_fetches_nothing_for_a_removed_pack() -> TestResult {
        let lock = lock_with(
            vec![locked_pack(
                "old",
                git_locked(REV_A, Some("old-v1"), None)?,
                hash_of(&files("old", &[("pack.toml", "a")]))?,
            )?],
            vec![],
        )?;
        let scenario = with_tool_tree(Scenario::new(&project(PATH_TOOL, PATH_PACK)).lock(&lock))?
            .path_source("packs/core", files("packs/core", &[("pack.toml", "a")]))?;
        let fetched = fetch(&scenario)?;
        assert!(scenario.git.calls().is_empty());
        assert!(!fetched.entries.contains_key(&SourceKey::Pack(pack("old")?)));
        Ok(())
    }
}
