//! A scenario builder over the port fakes, shared by every test of the plan flow.
//!
//! # Responsibilities
//!
//! - [`Scenario`] holds one [`FakeWorkspace`], [`FakeGit`], [`FakeBinary`] and [`FakeAdapter`]
//!   and edits each through a closure, so a test lays out the project, the lock, the sources, the
//!   `PATH` and the scripted extensions it needs and nothing else.
//! - Small constructors for the typed values the tests build trees and scripts from.
//! - [`shown`] turns a refusal set into the sorted lines a test compares.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only and decides nothing: every answer a test sees comes from the real
//! rules in `types` run over these fakes.

use std::error::Error;
use std::num::NonZeroUsize;

use crate::port::{FakeAdapter, FakeBinary, FakeFileTree, FakeGit, FakeWorkspace};
use std::path::Path;

use crate::app::plan::load::{Loaded, load_set};
use crate::app::plan::read::read_inputs;
use crate::app::plan::sources::{Tree, fetch_sources};
use crate::port::ScriptStep;
use crate::types::lock::{
    FileEntry, LockContents, LockedEntry, LockedSource, Lockfile, Owner, write_lock,
};
use crate::types::manifest::PackVersion;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::OpPath;
use crate::types::op::{
    ItemId, LuaType, Operation, Origin, Proposal, SkipReason, ValuesInput, ValuesResult,
};
use crate::types::pack::Pack;
use crate::types::project::{CommitId, GitTag, GitUrl, LocalPath, SubPath};
use crate::types::report::Diagnostics;
use crate::types::set::ValidSet;
use crate::types::tree::TreePath;
use crate::types::tree::{ContentHash, tree_hash};

pub(super) type TestResult = Result<(), Box<dyn Error>>;

/// The first fake commit.
pub(super) const REV_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// The second fake commit.
pub(super) const REV_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
/// The remote every git source of a test comes from.
pub(super) const URL: &str = "https://example.com/org/packs";

/// The sorted `Display` lines of `found`.
pub(super) fn shown(found: Diagnostics) -> Vec<String> {
    found
        .into_sorted()
        .iter()
        .map(ToString::to_string)
        .collect()
}

pub(super) fn tool(name: &str) -> Result<ToolName, Box<dyn Error>> {
    Ok(ToolName::new(name)?)
}

pub(super) fn pack(name: &str) -> Result<PackName, Box<dyn Error>> {
    Ok(PackName::new(name)?)
}

pub(super) fn item(name: &str) -> Result<ItemName, Box<dyn Error>> {
    Ok(ItemName::new(name)?)
}

pub(super) fn op(text: &str) -> Result<OpPath, Box<dyn Error>> {
    Ok(OpPath::new(text)?)
}

pub(super) fn rev(text: &str) -> Result<CommitId, Box<dyn Error>> {
    Ok(CommitId::new(text)?)
}

pub(super) fn url() -> Result<GitUrl, Box<dyn Error>> {
    Ok(GitUrl::new(URL)?)
}

pub(super) fn git_tag(text: &str) -> Result<GitTag, Box<dyn Error>> {
    Ok(GitTag::new(text)?)
}

pub(super) fn local(text: &str) -> Result<LocalPath, Box<dyn Error>> {
    Ok(LocalPath::new(text)?)
}

pub(super) fn sub(text: &str) -> Result<SubPath, Box<dyn Error>> {
    Ok(SubPath::new(text)?)
}

/// A tree rooted at `root` holding each of `files` as `(path, text)`.
pub(super) fn files(root: &str, files: &[(&str, &str)]) -> FakeFileTree {
    files
        .iter()
        .fold(FakeFileTree::new(root), |tree, (rel, text)| {
            tree.file(rel, text.as_bytes())
        })
}

/// A pack named `name` at version 0.1.0 holding the skill `status`, rooted at `root`.
pub(super) fn pack_files(root: &str, name: &str) -> FakeFileTree {
    FakeFileTree::new(root)
        .file(
            "pack.toml",
            format!(
                "[pack]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n"
            )
            .as_bytes(),
        )
        .file("skills/status/SKILL.md", SKILL_MD.as_bytes())
}

/// The neutral `SKILL.md` of the skill `status` that [`pack_files`] holds.
pub(super) const SKILL_MD: &str = "---\nname: status\ndescription: d\n---\nBody\n";

/// An extension for `tool` at version 0.1.0, rooted at `root`, reading the project files `reads`.
pub(super) fn ext_files_reading(root: &str, tool: &str, reads: &[&str]) -> FakeFileTree {
    let reads = if reads.is_empty() {
        String::new()
    } else {
        let quoted: Vec<String> = reads.iter().map(|read| format!("\"{read}\"")).collect();
        format!("reads = [{}]\n", quoted.join(", "))
    };
    FakeFileTree::new(root)
        .file(
            "extension.toml",
            format!(
                "[extension]\nname = \"{tool}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\n{reads}"
            )
            .as_bytes(),
        )
        .file("adapter.lua", b"")
}

/// An extension for `tool` at version 0.1.0 that reads no project file.
pub(super) fn ext_files(root: &str, tool: &str) -> FakeFileTree {
    ext_files_reading(root, tool, &[])
}

/// The tree hash of `tree`, as the flow computes it.
pub(super) fn hash_of(tree: &FakeFileTree) -> Result<ContentHash, Box<dyn Error>> {
    tree_hash(tree).map_err(|found| shown(found).join("; ").into())
}

/// A locked git source of `URL` at `rev_text`.
pub(super) fn git_locked(
    rev_text: &str,
    tag: Option<&str>,
    subdir: Option<&str>,
) -> Result<LockedSource, Box<dyn Error>> {
    Ok(LockedSource::Git {
        url: url()?,
        tag: tag.map(git_tag).transpose()?,
        subdir: subdir.map(sub).transpose()?,
        rev: rev(rev_text)?,
    })
}

/// A locked directory of the project.
pub(super) fn path_locked(dir: &str) -> Result<LockedSource, Box<dyn Error>> {
    Ok(LockedSource::Path(local(dir)?))
}

/// A locked pack at version 0.1.0.
pub(super) fn locked_pack(
    name: &str,
    source: LockedSource,
    tree: ContentHash,
) -> Result<LockedEntry<PackName>, Box<dyn Error>> {
    Ok(LockedEntry::new(
        pack(name)?,
        PackVersion::new("0.1.0")?,
        source,
        tree,
    ))
}

/// A locked tool at version 0.1.0.
pub(super) fn locked_tool(
    name: &str,
    source: LockedSource,
    tree: ContentHash,
) -> Result<LockedEntry<ToolName>, Box<dyn Error>> {
    Ok(LockedEntry::new(
        tool(name)?,
        PackVersion::new("0.1.0")?,
        source,
        tree,
    ))
}

/// A locked file written by `owner`, holding `bytes`.
pub(super) fn locked_file(
    path: &str,
    bytes: &[u8],
    owner: &str,
) -> Result<FileEntry, Box<dyn Error>> {
    Ok(FileEntry::new(
        sub(path)?,
        ContentHash::of_bytes(bytes),
        Owner::new(owner)?,
    ))
}

/// The text of a lock holding `contents`.
pub(super) fn lock_text(contents: LockContents) -> Result<String, Box<dyn Error>> {
    let lock = Lockfile::new(Path::new("agentstacks.lock"), contents)
        .map_err(|found| shown(found).join("; "))?;
    Ok(write_lock(&lock))
}

/// A project of the tool `claude` and the pack `core`, both read from the project directory.
pub(super) const PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";

/// What loading a scenario's sources gives.
pub(super) type LoadedTrees = Loaded<Tree<FakeFileTree, FakeFileTree>>;

/// Reads, fetches and loads the sources of `scenario`.
pub(super) fn loaded(scenario: &Scenario) -> Result<LoadedTrees, Diagnostics> {
    let inputs = read_inputs(&scenario.workspace)?;
    load_set(
        &inputs,
        fetch_sources(&inputs, &scenario.workspace, &scenario.git),
    )
}

/// A scenario of `project` whose pack `core` is `pack_tree` and whose tool is `claude`.
pub(super) fn single(project: &str, pack_tree: FakeFileTree) -> Result<Scenario, Box<dyn Error>> {
    Scenario::new(project)
        .path_source("packs/core", pack_tree)?
        .path_source(
            "extensions/claude",
            ext_files("extensions/claude", "claude"),
        )
}

/// The `values` answer `json` for `tool_name`, over the packs of `set`.
pub(super) fn values_for(
    set: &ValidSet,
    tool_name: &str,
    json: &str,
) -> Result<ValuesResult, Box<dyn Error>> {
    let packs: Vec<&Pack> = set.packs().values().collect();
    values_over(&packs, tool_name, json)
}

/// The `values` answer `json` for `tool_name`, over `packs`.
pub(super) fn values_over(
    packs: &[&Pack],
    tool_name: &str,
    json: &str,
) -> Result<ValuesResult, Box<dyn Error>> {
    let input = ValuesInput::new(tool(tool_name)?, packs);
    let value: serde_json::Value = serde_json::from_str(json)?;
    let object = value.as_object().ok_or("the answer is an object")?;
    let entry = format!("{tool_name}/adapter.lua");
    ValuesResult::read(object, &input, Path::new(&entry), |_| LuaType::Table)
        .map_err(|found| shown(found).join("; ").into())
}

/// The place of a call at `line` of the entry file of `tool_name`.
pub(super) fn origin(tool_name: &str, line: usize) -> Result<Origin, Box<dyn Error>> {
    Ok(Origin::Extension {
        tool: tool(tool_name)?,
        file: TreePath::from_parts(&["adapter.lua"]),
        line: NonZeroUsize::new(line).ok_or("a line starts at 1")?,
    })
}

fn propose(
    tool_name: &str,
    line: usize,
    operation: Operation,
) -> Result<ScriptStep, Box<dyn Error>> {
    Ok(ScriptStep::Propose(Proposal {
        op: operation,
        origin: origin(tool_name, line)?,
    }))
}

/// A `write` of `content` to `path` that covers `covers`, called at `line`.
pub(super) fn write_step(
    tool_name: &str,
    line: usize,
    path: &str,
    content: &[u8],
    covers: Vec<ItemId>,
) -> Result<ScriptStep, Box<dyn Error>> {
    propose(
        tool_name,
        line,
        Operation::Write {
            path: op(path)?,
            content: content.to_vec(),
            covers,
        },
    )
}

/// A `copy` of the file `file` to `path`, called at `line`.
pub(super) fn copy_step(
    tool_name: &str,
    line: usize,
    path: &str,
    file: ItemId,
) -> Result<ScriptStep, Box<dyn Error>> {
    propose(
        tool_name,
        line,
        Operation::Copy {
            path: op(path)?,
            file,
        },
    )
}

/// A `skip` of `item` for `reason`, called at `line`.
pub(super) fn skip_step(
    tool_name: &str,
    line: usize,
    item: ItemId,
    reason: &str,
) -> Result<ScriptStep, Box<dyn Error>> {
    propose(
        tool_name,
        line,
        Operation::Skip {
            item,
            reason: SkipReason::new(reason)?,
        },
    )
}

/// A `read` of `path`.
pub(super) fn read_step(path: &str) -> Result<ScriptStep, Box<dyn Error>> {
    Ok(ScriptStep::Read(op(path)?))
}

/// The id of the file `rel` of the skill `skill_name` of `pack_name`.
pub(super) fn skill_file_id(
    pack_name: &str,
    skill_name: &str,
    rel: &str,
) -> Result<ItemId, Box<dyn Error>> {
    Ok(ItemId::skill_file(
        &pack(pack_name)?,
        &item(skill_name)?,
        &TreePath::from_parts(&[rel]),
    ))
}

/// The id of the hook at `index` of `pack_name`.
pub(super) fn hook_id(pack_name: &str, index: usize) -> Result<ItemId, Box<dyn Error>> {
    Ok(ItemId::hook(&pack(pack_name)?, index))
}

/// The ports a flow runs over, edited one closure at a time.
#[derive(Default)]
pub(super) struct Scenario {
    pub(super) workspace: FakeWorkspace,
    pub(super) git: FakeGit,
    pub(super) binary: FakeBinary,
    pub(super) adapter: FakeAdapter,
}

impl Scenario {
    /// A scenario whose `agentstacks.toml` holds `project`.
    pub(super) fn new(project: &str) -> Self {
        Self {
            workspace: FakeWorkspace::new().project(project),
            ..Self::default()
        }
    }

    /// Edits the workspace.
    pub(super) fn workspace(mut self, edit: impl FnOnce(FakeWorkspace) -> FakeWorkspace) -> Self {
        self.workspace = edit(std::mem::take(&mut self.workspace));
        self
    }

    /// Edits the git remote.
    pub(super) fn git(mut self, edit: impl FnOnce(FakeGit) -> FakeGit) -> Self {
        self.git = edit(std::mem::take(&mut self.git));
        self
    }

    /// Edits the `PATH`.
    pub(super) fn binary(mut self, edit: impl FnOnce(FakeBinary) -> FakeBinary) -> Self {
        self.binary = edit(std::mem::take(&mut self.binary));
        self
    }

    /// Edits the scripted extensions.
    pub(super) fn adapter(mut self, edit: impl FnOnce(FakeAdapter) -> FakeAdapter) -> Self {
        self.adapter = edit(std::mem::take(&mut self.adapter));
        self
    }

    /// Sets the text of `agentstacks.lock`.
    pub(super) fn lock(self, text: &str) -> Self {
        self.workspace(|workspace| workspace.lock(text))
    }

    /// Serves `tree` for the local source directory `dir`.
    pub(super) fn path_source(self, dir: &str, tree: FakeFileTree) -> Result<Self, Box<dyn Error>> {
        let dir = local(dir)?;
        Ok(self.workspace(|workspace| workspace.source(&dir, tree)))
    }
}
