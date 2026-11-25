//! Values the command tests are built from.
//!
//! # Responsibilities
//!
//! - A console over buffers, with the answers a test types, and the text it wrote.
//! - Plans made from steps, for the sequences that take a plan.
//! - [`World`]: the library fakes laid out as a project that plans to one new file, for the
//!   commands that run the whole flow.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only and decides nothing: every answer a test sees comes from the real
//! library rules run over the fakes.

use std::cell::Cell;
use std::error::Error;
use std::io::{self, Cursor};
use std::num::NonZeroUsize;
use std::path::Path;

use agentstacks::port::{
    FakeAdapter, FakeBinary, FakeFileTree, FakeGit, FakeWorkspace, ScriptStep, ToolScript,
    Workspace,
};
use agentstacks::types::lock::{LockContents, Lockfile, Owner};
use agentstacks::types::name::{ItemName, PackName, ToolName};
use agentstacks::types::op::{
    ItemId, LuaType, OpPath, Operation, Origin, Proposal, ValuesInput, ValuesResult,
};
use agentstacks::types::pack::Pack;
use agentstacks::types::plan::{Action, DiskState, Plan, Resolution, Step};
use agentstacks::types::project::{CommitId, GitTag, GitUrl, LocalPath};
use agentstacks::types::resolve::RemoteTag;
use agentstacks::types::tree::{TreePath, walk};

use super::{Console, Ports};

/// What a test body returns.
pub(super) type TestResult = Result<(), Box<dyn Error>>;

/// A console over a canned input and two buffers.
pub(super) type TestConsole = Console<Cursor<Vec<u8>>, Vec<u8>, Vec<u8>>;

/// The fakes a command runs over.
pub(super) type TestPorts = Ports<FakeWorkspace, FakeGit, FakeBinary, FakeAdapter>;

/// A console whose stdin is a terminal or not and holds `input`.
pub(super) fn console(is_terminal: bool, input: &str) -> TestConsole {
    Console {
        is_terminal,
        input: Cursor::new(input.as_bytes().to_vec()),
        out: Vec::new(),
        err: Vec::new(),
    }
}

/// What the command wrote to stdout.
pub(super) fn out_of(console: &TestConsole) -> String {
    String::from_utf8_lossy(&console.out).into_owned()
}

/// What the command wrote to stderr.
pub(super) fn err_of(console: &TestConsole) -> String {
    String::from_utf8_lossy(&console.err).into_owned()
}

/// A step of `action` on `path` with no bytes to write.
pub(super) fn step_of(path: &str, action: Action) -> Result<Step, Box<dyn Error>> {
    Ok(Step {
        path: OpPath::new(path)?,
        action,
        owner: Some(Owner::new("core")?),
        origin: None,
        new: None,
        old: None,
        decided_on: None,
    })
}

/// A create of `path` holding `new`, decided on an absent file.
pub(super) fn create_step(path: &str) -> Result<Step, Box<dyn Error>> {
    Ok(Step {
        new: Some(b"new".to_vec()),
        decided_on: Some(DiskState::Absent),
        ..step_of(path, Action::Create)?
    })
}

/// A plan of `steps` with an empty lock.
pub(super) fn plan_of(steps: Vec<Step>) -> Result<Plan, Box<dyn Error>> {
    let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
        .map_err(|refusals| format!("{refusals:?}"))?;
    Ok(Plan::new(
        steps,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        lock,
        Vec::new(),
    ))
}

/// The file a [`World`] plans to create.
pub(super) const AGENT_FILE: &str = ".claude/agents/coder.md";

/// The remote the git source of [`World::git`] comes from.
pub(super) const URL: &str = "https://example.com/org/packs";

/// A commit that [`World::git`] serves the pack at.
const REV: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// The project of the tool `claude` and the pack `core`, both read from the project directory.
pub(super) const PATH_PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";

/// The project of [`PATH_PROJECT`] with the pack `core` taken from [`URL`] at `packs-v0.1.0`.
pub(super) const GIT_PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\ngit = \"https://example.com/org/packs\"\ntag = \"packs-v0.1.0\"  # pinned\n";

/// The tree of a pack `core` at 0.1.0 holding the agent `coder`.
pub(super) fn core_tree() -> FakeFileTree {
    FakeFileTree::new("packs/core")
        .file(
            "pack.toml",
            b"[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n",
        )
        .file(
            "agents/coder.md",
            b"---\nname: coder\ndescription: d\n---\nagent body\n",
        )
}

/// The tree of the extension of the tool `claude`.
fn claude_tree() -> FakeFileTree {
    FakeFileTree::new("extensions/claude")
        .file(
            "extension.toml",
            b"[extension]\nname = \"claude\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\n",
        )
        .file("adapter.lua", b"")
}

/// The path of the entry file inside the extension of [`claude_tree`].
fn adapter_path() -> Result<TreePath, Box<dyn Error>> {
    let (walked, _) = walk(&claude_tree());
    walked
        .files()
        .iter()
        .find(|path| path.as_str() == "adapter.lua")
        .cloned()
        .ok_or_else(|| "the extension has an entry file".into())
}

/// The git remote of [`URL`]: it lists `packs-v0.1.0` and `packs-v0.2.0`, and serves the pack at
/// both when `serve` is true.
pub(super) fn remote(serve: bool) -> Result<FakeGit, Box<dyn Error>> {
    let url = GitUrl::new(URL)?;
    let rev = CommitId::new(REV)?;
    let tags = ["packs-v0.1.0", "packs-v0.2.0"]
        .iter()
        .map(|tag| {
            Ok(RemoteTag {
                rev: rev.clone(),
                tag: GitTag::new(tag)?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let mut git = FakeGit::new().tags(&url, tags);
    if serve {
        git = git
            .tag(&url, &GitTag::new("packs-v0.1.0")?, &rev)
            .tag(&url, &GitTag::new("packs-v0.2.0")?, &rev)
            .files(&url, &rev, None, core_tree());
    }
    Ok(git)
}

/// The library fakes laid out as a project that plans to create [`AGENT_FILE`].
pub(super) struct World {
    pub(super) ports: TestPorts,
}

impl World {
    /// A project with the tool `claude` and the pack `core` served from the project directory.
    /// `project` is the text of `agentstacks.toml`, or `None` for a project without one.
    fn build(project: Option<&str>, git: FakeGit) -> Result<Self, Box<dyn Error>> {
        let core = PackName::new("core")?;
        let coder = ItemName::new("coder")?;
        let claude = ToolName::new("claude")?;

        let pack = Pack::load(&core_tree()).map_err(|found| format!("{found:?}"))?;
        let values = {
            let object = serde_json::json!({ "root": ".claude/agentstacks" });
            let object = object.as_object().ok_or("an object")?;
            let input = ValuesInput::new(claude.clone(), &[&pack]);
            ValuesResult::read(object, &input, Path::new("claude/adapter.lua"), |_| {
                LuaType::Table
            })
            .map_err(|found| format!("{found:?}"))?
        };
        let proposal = Proposal {
            op: Operation::Write {
                path: OpPath::new(AGENT_FILE)?,
                content: b"agent".to_vec(),
                covers: vec![ItemId::agent(&core, &coder)],
            },
            origin: Origin::Extension {
                tool: claude.clone(),
                file: adapter_path()?,
                line: NonZeroUsize::new(1).ok_or("a line starts at 1")?,
            },
        };
        let script = ToolScript::new()
            .values(Ok(values))
            .step(ScriptStep::Propose(proposal));

        let mut workspace = FakeWorkspace::new()
            .source(&LocalPath::new("packs/core")?, core_tree())
            .source(&LocalPath::new("extensions/claude")?, claude_tree())
            .source(&LocalPath::new("./packs/core")?, core_tree())
            .source(&LocalPath::new("./extensions/claude")?, claude_tree());
        if let Some(project) = project {
            workspace = workspace.project(project);
        }
        Ok(Self {
            ports: Ports {
                workspace,
                git,
                binary: FakeBinary::new(),
                adapter: FakeAdapter::new().tool(&claude, script),
            },
        })
    }

    /// The project of [`PATH_PROJECT`].
    pub(super) fn paths() -> Result<Self, Box<dyn Error>> {
        Self::build(Some(PATH_PROJECT), FakeGit::new())
    }

    /// The project of [`PATH_PROJECT`] written for the `0.x` releases, with `api = 0`.
    pub(super) fn legacy() -> Result<Self, Box<dyn Error>> {
        Self::build(
            Some(&PATH_PROJECT.replacen("api = 1", "api = 0", 1)),
            FakeGit::new(),
        )
    }

    /// The project of [`GIT_PROJECT`], whose remote serves both tags.
    pub(super) fn git() -> Result<Self, Box<dyn Error>> {
        Self::build(Some(GIT_PROJECT), remote(true)?)
    }

    /// The project of [`GIT_PROJECT`], whose remote lists both tags but serves neither, so a plan
    /// of it is refused.
    pub(super) fn unserved() -> Result<Self, Box<dyn Error>> {
        Self::build(Some(GIT_PROJECT), remote(false)?)
    }

    /// A project with no `agentstacks.toml`, for `init`.
    pub(super) fn empty() -> Result<Self, Box<dyn Error>> {
        Self::build(None, FakeGit::new())
    }
}

/// A workspace that answers like `inner` except for what changes after the first read: the text of
/// `agentstacks.toml` when `project_after` is set, and a failing lock read when `lock_fails`.
pub(super) struct Shifting {
    inner: FakeWorkspace,
    project_after: Option<String>,
    lock_fails: bool,
    project_reads: Cell<usize>,
    lock_reads: Cell<usize>,
}

impl Shifting {
    /// `inner` with the later project reads giving `project_after` and, with `lock_fails`, the
    /// later lock reads failing.
    pub(super) fn new(inner: FakeWorkspace, project_after: Option<&str>, lock_fails: bool) -> Self {
        Self {
            inner,
            project_after: project_after.map(str::to_owned),
            lock_fails,
            project_reads: Cell::new(0),
            lock_reads: Cell::new(0),
        }
    }
}

impl Workspace for Shifting {
    type Tree = FakeFileTree;

    fn project_text(&self) -> io::Result<String> {
        let reads = self.project_reads.get();
        self.project_reads.set(reads + 1);
        match (&self.project_after, reads) {
            (Some(text), 1..) => Ok(text.clone()),
            _ => self.inner.project_text(),
        }
    }

    fn lock_text(&self) -> io::Result<Option<String>> {
        let reads = self.lock_reads.get();
        self.lock_reads.set(reads + 1);
        if self.lock_fails && reads >= 1 {
            return Err(FakeWorkspace::denied());
        }
        self.inner.lock_text()
    }

    fn resolve(&self, path: &OpPath) -> io::Result<Resolution> {
        self.inner.resolve(path)
    }

    fn source_tree(&self, dir: &LocalPath) -> io::Result<Option<Self::Tree>> {
        self.inner.source_tree(dir)
    }

    fn stage(&self, path: &OpPath, bytes: &[u8]) -> io::Result<()> {
        self.inner.stage(path, bytes)
    }

    fn discard(&self, path: &OpPath) -> io::Result<()> {
        self.inner.discard(path)
    }

    fn swap(&self, path: &OpPath) -> io::Result<()> {
        self.inner.swap(path)
    }

    fn delete(&self, path: &OpPath) -> io::Result<()> {
        self.inner.delete(path)
    }

    fn write_project(&self, text: &str) -> io::Result<()> {
        self.inner.write_project(text)
    }

    fn write_lock(&self, text: &str) -> io::Result<()> {
        self.inner.write_lock(text)
    }
}
