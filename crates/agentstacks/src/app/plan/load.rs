//! Step 4 of the plan flow: loading every fetched tree and checking the set as a whole.
//!
//! # Responsibilities
//!
//! - [`load_set`] hands `check` a loader for packs and a loader for tools. A loader loads the
//!   fetched tree of its key, or returns that key's fetch refusals, so `check` reports every
//!   refusal together and runs no second check on an entry that failed.
//! - Each tool the project dropped is loaded on its own, because the project no longer lists it and
//!   `check` cannot see it.
//! - It adds a note for each dot entry a pack or an extension ignored and for each `native/`
//!   directory no listed tool claims.
//! - [`Loaded`] is the flow's carrier for the checked set, the trees it was loaded from, the
//!   extensions of the removed tools and the notes.
//!
//! # Non-responsibilities
//!
//! What makes a pack, an extension or a set valid is decided by `Pack::load`, `Extension::load` and
//! `check`; this file only calls them and collects what they say.

use std::collections::BTreeMap;
use std::path::Path;

use crate::app::plan::read::Inputs;
use crate::app::plan::sources::{Entry, Fetched};
use crate::port::FileTree;
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::pack::Pack;
use crate::types::plan::Note;
use crate::types::project::PROJECT_FILE;
use crate::types::report::Diagnostics;
use crate::types::resolve::SourceKey;
use crate::types::set::{ValidSet, check};

/// A tool the project dropped, loaded from the tree its lock entry names.
pub(super) struct Removed<T> {
    /// The extension, which runs once more so the tool can take its entries out of shared files.
    pub(super) extension: Extension,
    /// The tree the extension was loaded from.
    pub(super) entry: Entry<T>,
}

/// A checked set and what it was loaded from.
pub(super) struct Loaded<T> {
    /// The project, its packs and its extensions, checked as a whole.
    pub(super) set: ValidSet,
    /// The tree, the resolved source and the hash of every pack and tool the project lists.
    pub(super) entries: BTreeMap<SourceKey, Entry<T>>,
    /// The tools the project dropped.
    pub(super) removed: BTreeMap<ToolName, Removed<T>>,
    /// The notes found while loading: dot entries left out and unused `native/` directories.
    pub(super) notes: Vec<Note>,
}

/// Loads every fetched tree and checks the set.
///
/// # Errors
///
/// Every refusal of fetching, loading and checking, together.
pub(super) fn load_set<T: FileTree>(
    inputs: &Inputs,
    fetched: Fetched<T>,
) -> Result<Loaded<T>, Diagnostics> {
    let Fetched {
        entries,
        mut notes,
        shared,
    } = fetched;
    let mut pack_sources = BTreeMap::new();
    let mut tool_sources = BTreeMap::new();
    let mut removed_sources = BTreeMap::new();
    for (key, result) in entries {
        match key {
            SourceKey::Pack(name) => {
                pack_sources.insert(name, result);
            }
            SourceKey::Tool(name) if inputs.project.tools().contains_key(&name) => {
                tool_sources.insert(name, result);
            }
            SourceKey::Tool(name) => {
                removed_sources.insert(name, result);
            }
        }
    }

    let mut kept_packs = BTreeMap::new();
    let mut kept_tools = BTreeMap::new();
    let checked = check(
        Path::new(PROJECT_FILE),
        inputs.project.clone(),
        |name, _| {
            load_entry(&mut pack_sources, &mut kept_packs, name, |tree| {
                Pack::load(tree)
            })
        },
        |name, _| {
            load_entry(&mut tool_sources, &mut kept_tools, name, |tree| {
                Extension::load(tree)
            })
        },
    );

    let mut refusals = Diagnostics::new();
    let set = match checked {
        Ok(set) => Some(set),
        Err(found) => {
            refusals.extend(found);
            None
        }
    };
    let mut removed = BTreeMap::new();
    for (name, result) in removed_sources {
        match result {
            Err(found) => refusals.extend(found),
            Ok(entry) => match Extension::load(&entry.tree) {
                Ok(extension) => {
                    removed.insert(name, Removed { extension, entry });
                }
                Err(found) => refusals.extend(found),
            },
        }
    }
    refusals.extend(shared);
    let Some(set) = set.filter(|_| refusals.is_empty()) else {
        return Err(refusals);
    };

    notes.extend(load_notes(inputs, &set));
    let mut kept = BTreeMap::new();
    kept.extend(
        kept_packs
            .into_iter()
            .map(|(name, entry)| (SourceKey::Pack(name), entry)),
    );
    kept.extend(
        kept_tools
            .into_iter()
            .map(|(name, entry)| (SourceKey::Tool(name), entry)),
    );
    Ok(Loaded {
        set,
        entries: kept,
        removed,
        notes,
    })
}

/// Loads the fetched entry of `key` with `load`, keeping the entry when it loaded. An entry whose
/// fetch was refused returns that refusal; a key with no fetched entry returns none, because
/// whatever kept it out has already refused.
fn load_entry<K: Ord + Clone, T: FileTree, V>(
    sources: &mut BTreeMap<K, Result<Entry<T>, Diagnostics>>,
    kept: &mut BTreeMap<K, Entry<T>>,
    key: &K,
    load: impl FnOnce(&T) -> Result<V, Diagnostics>,
) -> Result<V, Diagnostics> {
    match sources.remove(key) {
        None => Err(Diagnostics::new()),
        Some(Err(found)) => Err(found),
        Some(Ok(entry)) => {
            let loaded = load(&entry.tree)?;
            kept.insert(key.clone(), entry);
            Ok(loaded)
        }
    }
}

/// The notes for what loading left out: each ignored dot entry of a pack and of an extension, and
/// each `native/<dir>` no listed tool claims, once for each listed tool.
fn load_notes(inputs: &Inputs, set: &ValidSet) -> Vec<Note> {
    let tools: Vec<ToolName> = inputs.project.tools().keys().cloned().collect();
    let mut notes = Vec::new();
    for pack in set.packs().values() {
        notes.extend(pack.ignored().iter().map(|path| Note::Ignored {
            root: pack.dir().to_path_buf(),
            path: path.clone(),
        }));
        for dir in pack.unused_native(&tools) {
            notes.extend(tools.iter().map(|tool| Note::UnusedNative {
                dir: dir.clone(),
                tool: tool.clone(),
            }));
        }
    }
    for extension in set.extensions().values() {
        notes.extend(extension.ignored().iter().map(|path| Note::Ignored {
            root: extension.dir().to_path_buf(),
            path: path.clone(),
        }));
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::read::read_inputs;
    use crate::app::plan::sources::{Tree, fetch_sources};
    use crate::app::plan::testing::{
        REV_A, Scenario, TestResult, URL, ext_files, git_locked, hash_of, lock_text, locked_tool,
        pack_files, rev, shown, sub, tool, url,
    };
    use crate::port::{FakeFileTree, FakeGit};
    use crate::types::lock::LockContents;

    type TestLoaded = Loaded<Tree<FakeFileTree, FakeFileTree>>;

    const PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";

    fn load(scenario: &Scenario) -> Result<TestLoaded, Diagnostics> {
        let inputs = read_inputs(&scenario.workspace)?;
        load_set(
            &inputs,
            fetch_sources(&inputs, &scenario.workspace, &scenario.git),
        )
    }

    fn standard() -> Result<Scenario, Box<dyn std::error::Error>> {
        Scenario::new(PROJECT)
            .path_source("packs/core", pack_files("packs/core", "core"))?
            .path_source(
                "extensions/claude",
                ext_files("extensions/claude", "claude"),
            )
    }

    #[test]
    fn load_set_returns_the_valid_set_of_fetched_sources() -> TestResult {
        let loaded = load(&standard()?).map_err(|found| shown(found).join("; "))?;
        assert_eq!(loaded.set.packs().len(), 1);
        assert_eq!(loaded.set.extensions().len(), 1);
        assert_eq!(loaded.entries.len(), 2);
        assert!(loaded.removed.is_empty());
        Ok(())
    }

    #[test]
    fn load_set_reports_fetch_and_load_refusals_together() -> TestResult {
        let text = format!(
            "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.a]\ngit = \"{URL}\"\nrev = \"{REV_A}\"\n\n[packs.b]\npath = \"packs/b\"\n"
        );
        let broken = pack_files("packs/b", "b").file(
            "pack.toml",
            b"[pack]\nname = \"b\"\nversion = \"0.1.0\"\napi = 2\ndescription = \"d\"\n",
        );
        let scenario = Scenario::new(&text)
            .path_source("packs/b", broken)?
            .path_source(
                "extensions/claude",
                ext_files("extensions/claude", "claude"),
            )?
            .git(FakeGit::git_missing);
        let found = load(&scenario).err().ok_or("refusals")?;
        let lines = shown(found);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines.contains(
                &"agentstacks.toml: git not found on PATH (needed by packs.a)".to_owned()
            ),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("not supported")),
            "{lines:?}"
        );
        Ok(())
    }

    #[test]
    fn load_set_loads_a_removed_tool_outside_the_set() -> TestResult {
        let snapshot = ext_files("extensions/opencode", "opencode");
        let lock = lock_text(LockContents {
            tools: vec![locked_tool(
                "opencode",
                git_locked(REV_A, None, None)?,
                hash_of(&snapshot)?,
            )?],
            ..LockContents::default()
        })?;
        let (url, at) = (url()?, rev(REV_A)?);
        let scenario = standard()?
            .lock(&lock)
            .git(|git| git.files(&url, &at, None, snapshot.clone()));
        let loaded = load(&scenario).map_err(|found| shown(found).join("; "))?;
        assert!(loaded.removed.contains_key(&tool("opencode")?));
        assert!(!loaded.set.extensions().contains_key(&tool("opencode")?));
        Ok(())
    }

    #[test]
    fn load_set_notes_dot_entries_and_unused_native_dirs() -> TestResult {
        let tree = pack_files("packs/core", "core")
            .file(".DS_Store", b"x")
            .file("native/cluade/x.md", b"x");
        let scenario = Scenario::new(PROJECT)
            .path_source("packs/core", tree)?
            .path_source(
                "extensions/claude",
                ext_files("extensions/claude", "claude"),
            )?;
        let loaded = load(&scenario).map_err(|found| shown(found).join("; "))?;
        let notes: Vec<String> = loaded.notes.iter().map(ToString::to_string).collect();
        assert_eq!(
            notes,
            [
                "packs/core/.DS_Store: ignored (dot entry)",
                "native/cluade: not for claude"
            ]
        );
        Ok(())
    }

    #[test]
    fn load_set_labels_a_git_dot_entry_by_its_subdir() -> TestResult {
        let text = |subdir: &str| {
            format!(
                "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\ngit = \"{URL}\"\nrev = \"{REV_A}\"\n{subdir}\n"
            )
        };
        let (url, at, subdir) = (url()?, rev(REV_A)?, sub("packs/core")?);
        let with_subdir = Scenario::new(&text("subdir = \"packs/core\""))
            .path_source(
                "extensions/claude",
                ext_files("extensions/claude", "claude"),
            )?
            .git(|git| {
                git.files(
                    &url,
                    &at,
                    Some(&subdir),
                    pack_files("packs/core", "core").file(".DS_Store", b"x"),
                )
            });
        let loaded = load(&with_subdir).map_err(|found| shown(found).join("; "))?;
        let notes: Vec<String> = loaded.notes.iter().map(ToString::to_string).collect();
        assert_eq!(notes, ["packs/core/.DS_Store: ignored (dot entry)"]);

        let whole = Scenario::new(&text(""))
            .path_source(
                "extensions/claude",
                ext_files("extensions/claude", "claude"),
            )?
            .git(|git| {
                git.files(
                    &url,
                    &at,
                    None,
                    pack_files(".", "core").file(".DS_Store", b"x"),
                )
            });
        let loaded = load(&whole).map_err(|found| shown(found).join("; "))?;
        let notes: Vec<String> = loaded.notes.iter().map(ToString::to_string).collect();
        assert_eq!(notes, ["./.DS_Store: ignored (dot entry)"]);
        Ok(())
    }
}
