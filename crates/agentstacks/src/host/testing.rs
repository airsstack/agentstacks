//! Builders the unit tests of the host share.
//!
//! # Responsibilities
//!
//! - [`tree`] builds an in-memory extension directory: an `extension.toml` for the tool `claude`
//!   plus the files a test gives, and [`load`] loads it through [`Extension::load`].
//! - [`lua_files`] reads the `.lua` files of such a directory into the form `require` serves.
//! - [`Rig`] runs an entry file and dispatches events in the real sandbox, the way a session
//!   does, and words a failure as the user reads it. [`Rig::plan`] runs the `plan` handler against
//!   a payload model and a workspace.
//! - [`finishes_within`] runs a closure on its own thread and gives up on it after a limit, so a
//!   test of code that could hang fails instead of hanging the suite.
//! - [`plan_input`] and [`values_input`] are the payload models of one pack, `core`, for the tool
//!   `claude`.
//! - [`Probe`] is a host module for tests of the engine pieces: it installs `airsstack.probe` with
//!   functions that report what a host function sees.
//!
//! # Non-responsibilities
//!
//! Compiled for tests only; it asserts nothing itself.

use std::error::Error;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use airsl::mlua::{self, Lua, Table, Value};
use airsl::{Engine, HostModule, InstallContext, ModuleName, Script};
use serde_json::{Value as Json, json};

use crate::host::caller::{caller, locate};
use crate::host::engine::build_engine;
use crate::host::events::{Dispatched, Event, dispatch};
use crate::host::failure::{Raised, wrap};
use crate::host::files::LuaFiles;
use crate::host::module::AgentstacksModule;
use crate::host::read::ReadSnapshot;
use crate::host::state::{Loan, Phase, RunState, SharedState};
use crate::port::{FakeFileTree, Workspace};
use crate::types::extension::Extension;
use crate::types::frontmatter::{AgentMeta, SkillMeta};
use crate::types::hooks::{HookEvent, MatchValue};
use crate::types::manifest::PackVersion;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::{
    ItemId, OpPath, PlanAction, PlanAgent, PlanFile, PlanHook, PlanInput, PlanNative,
    PlanNativeHook, PlanPack, PlanSkill, Proposals, ValuesInput, ValuesPack,
};
use crate::types::plan::Reads;
use crate::types::report::{Diagnostic, FieldError, Location};
use crate::types::tokens::Var;
use crate::types::tree::TreePath;

/// The manifest of a tool `claude` whose entry is `adapter.lua` and whose `reads` are `reads`.
fn manifest(reads: &[&str]) -> String {
    let listed: Vec<String> = reads.iter().map(|path| format!("{path:?}")).collect();
    format!(
        "[extension]\nname = \"claude\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\nreads = [{}]\n",
        listed.join(", ")
    )
}

/// An extension directory holding the manifest and `files`, each a path and its bytes.
pub(super) fn tree(reads: &[&str], files: &[(&str, &[u8])]) -> FakeFileTree {
    let mut tree = FakeFileTree::new("ext").file("extension.toml", manifest(reads).as_bytes());
    for (path, bytes) in files {
        tree = tree.file(path, bytes);
    }
    tree
}

/// The directory of `tree`, loaded.
pub(super) fn load(tree: &FakeFileTree) -> Result<Extension, Box<dyn Error>> {
    Extension::load(tree).map_err(|refusals| {
        let shown: Vec<String> = refusals.into_iter().map(|d| d.to_string()).collect();
        shown.join("; ").into()
    })
}

/// The `.lua` files of an extension for `claude` holding `files`, each a path and its bytes. An
/// entry file `adapter.lua` is added, empty, when `files` has none.
pub(super) fn lua_files(files: &[(&str, &[u8])]) -> Result<Arc<LuaFiles>, Box<dyn Error>> {
    let mut all: Vec<(&str, &[u8])> = files.to_vec();
    if !all.iter().any(|(path, _)| *path == "adapter.lua") {
        all.push(("adapter.lua", b""));
    }
    let tree = tree(&[], &all);
    let extension = load(&tree)?;
    let read = LuaFiles::read(&tree, &extension).map_err(|refusals| {
        let shown: Vec<String> = refusals.into_iter().map(|d| d.to_string()).collect();
        shown.join("; ")
    })?;
    Ok(Arc::new(read))
}

/// What `work` returns, or `None` when it has not finished within `limit`.
///
/// `work` runs on a thread of its own, so a deadlock in it fails the test and does not hang the
/// suite. A thread that never finishes is left behind.
pub(super) fn finishes_within<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver.recv_timeout(limit).ok()
}

/// What a `plan` dispatch left behind.
pub(super) struct Planned {
    /// What the dispatch gave, or the text of its refusal.
    pub(super) result: Result<Dispatched, String>,
    /// The proposals after the dispatch, the ones given and the ones the handler added.
    pub(super) proposals: Proposals,
    /// What the handler's `read` calls returned.
    pub(super) reads: Reads,
}

/// The sandbox of a tool `claude`, driven the way a session drives it: an entry run, then
/// dispatches, each failure worded as the user would read it.
pub(super) struct Rig {
    engine: Engine,
    state: SharedState,
    tool: ToolName,
    entry: TreePath,
    reads: Vec<OpPath>,
}

impl Rig {
    /// A fresh sandbox for the tool `claude`, whose entry file is `adapter.lua` and which holds
    /// no other file and reads nothing.
    pub(super) fn new() -> Result<Self, Box<dyn Error>> {
        Self::with(&[], &[])
    }

    /// A fresh sandbox for the tool `claude` that declares `reads` and holds `files`, each a path
    /// and its bytes, besides its entry file `adapter.lua`.
    pub(super) fn with(reads: &[&str], files: &[(&str, &[u8])]) -> Result<Self, Box<dyn Error>> {
        let tool = ToolName::new("claude")?;
        let state = SharedState::new(tool.clone());
        let engine = build_engine(AgentstacksModule::new(state.clone(), lua_files(files)?)?)?;
        let reads = reads
            .iter()
            .map(|path| OpPath::new(path))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            engine,
            state,
            tool,
            entry: TreePath::from_parts(&["adapter.lua"]),
            reads,
        })
    }

    /// Runs `source` as the entry file, or the text of the refusal.
    pub(super) fn run(&self, source: &str) -> Result<(), String> {
        let script = Script::from_source(source, self.entry.as_str()).map_err(|e| e.to_string())?;
        self.engine.eval(&script).map_err(|e| self.shown(&e))
    }

    /// Runs `source` as the entry file and returns the string it returns, or the text of the
    /// refusal.
    pub(super) fn text(&self, source: &str) -> Result<String, String> {
        let script = Script::from_source(source, self.entry.as_str()).map_err(|e| e.to_string())?;
        self.engine
            .eval_to::<String>(&script)
            .map_err(|e| self.shown(&e))
    }

    /// How many refusals the run has recorded.
    pub(super) fn recorded(&self) -> usize {
        self.state.with(|run| run.take_recorded().len())
    }

    /// The refusals the run has recorded, in order, as the user reads them. They are forgotten.
    pub(super) fn refusals(&self) -> Vec<String> {
        let found = self.state.with(RunState::take_recorded);
        found.into_iter().map(|d| d.to_string()).collect()
    }

    /// Dispatches `event` with `payload`, or the text of the refusal. The `values` event runs in
    /// the `values` phase.
    pub(super) fn dispatch(&self, event: Event, payload: &Json) -> Result<Dispatched, String> {
        if event == Event::Values {
            self.state.with(|run| run.set_phase(Phase::Values));
        }
        let dispatched = dispatch(&self.engine, event, payload).map_err(|e| self.shown(&e));
        self.state.with(|run| run.set_phase(Phase::Load));
        dispatched
    }

    /// Dispatches `plan` with an empty payload the way a session does: `proposals`, the ids of
    /// `input` and the answers of `workspace` for the declared reads are lent to the run, and
    /// taken back whatever the handler did.
    pub(super) fn plan<W: Workspace>(
        &self,
        input: &PlanInput,
        proposals: Proposals,
        workspace: &W,
    ) -> Planned {
        let loan = Loan {
            input: input.clone(),
            proposals,
            snapshot: ReadSnapshot::take(workspace, &self.reads),
            reads: Reads::default(),
        };
        self.state.with(|run| run.begin_plan(loan));
        let result = dispatch(&self.engine, Event::Plan, &json!({})).map_err(|e| self.shown(&e));
        let loan = self.state.with(RunState::end_plan);
        let (proposals, reads) = loan.map_or_else(
            || (Proposals::new(), Reads::default()),
            |loan| (loan.proposals, loan.reads),
        );
        Planned {
            result,
            proposals,
            reads,
        }
    }

    /// The text of the refusal for `error`.
    fn shown(&self, error: &airsl::Error) -> String {
        wrap(&self.tool, &self.entry, error)
            .map_or_else(|| "(recorded)".to_owned(), |d| d.to_string())
    }
}

/// A host module for tests, installed as `airsstack.probe`.
///
/// - `where()` returns the location of its caller, as a refusal would be reported.
/// - `origin()` returns the location and the site of its caller, joined by `|`.
/// - `module(source, path)` runs `source` as the chunk `@<path>` and returns what it returns.
/// - `raise_recorded(text)` raises a refusal that counts as recorded, with `text` as its message.
/// - `raise_ordinary()` raises an ordinary refusal, `required`, located at its caller.
#[derive(Debug)]
pub(super) struct Probe {
    name: ModuleName,
    tool: ToolName,
}

impl Probe {
    /// A probe that reports locations for `tool`.
    pub(super) fn new(tool: ToolName) -> airsl::Result<Self> {
        Ok(Self {
            name: ModuleName::new("probe")?,
            tool,
        })
    }

    fn fill(&self, lua: &Lua, table: &Table) -> mlua::Result<()> {
        let tool = self.tool.clone();
        let at = lua.create_function(move |lua, ()| Ok(locate(lua, &tool).to_string()))?;
        table.set("where", at)?;

        let tool = self.tool.clone();
        let origin = lua.create_function(move |lua, ()| {
            let shown = |place: Option<Location>| place.map(|l| l.to_string()).unwrap_or_default();
            Ok(caller(lua)
                .map(|caller| caller.origin(&tool))
                .map_or_else(String::new, |origin| {
                    format!("{}|{}", shown(origin.location()), shown(origin.site()))
                }))
        })?;
        table.set("origin", origin)?;

        let module = lua.create_function(|lua, (source, path): (String, String)| {
            lua.load(source)
                .set_name(format!("@{path}"))
                .call::<Value>(())
        })?;
        table.set("module", module)?;

        let recorded = lua.create_function(|_, text: String| -> mlua::Result<()> {
            Err(Raised::recorded(text).into_lua())
        })?;
        table.set("raise_recorded", recorded)?;

        let tool = self.tool.clone();
        let ordinary = lua.create_function(move |lua, ()| -> mlua::Result<()> {
            let refusal = Diagnostic::new(locate(lua, &tool), FieldError::Required);
            Err(Raised::ordinary(refusal).into_lua())
        })?;
        table.set("raise_ordinary", ordinary)
    }
}

impl HostModule for Probe {
    fn name(&self) -> &ModuleName {
        &self.name
    }

    fn install(
        &self,
        lua: &Lua,
        table: &Table,
        _context: &InstallContext<'_>,
    ) -> airsl::Result<()> {
        self.fill(lua, table)
            .map_err(|error| airsl::Error::lua("probe", error))
    }
}

/// A skill or agent file holding `name`, the frontmatter `lines` and a one-line body.
fn document(name: &str, lines: &str) -> String {
    format!("---\nname: {name}\n{lines}---\nbody\n")
}

/// A file of a payload with the id `id` and the path `rel`.
fn payload_file(id: ItemId, rel: &str, text: Option<&str>) -> PlanFile {
    PlanFile {
        id,
        rel: TreePath::from_parts(&[rel]),
        text: text.map(str::to_owned),
    }
}

/// The `plan` payload model of the pack `core` 0.3.0 for the tool `claude`, root
/// `.claude/agentstacks`:
///
/// - the skill `status`, installed in `.claude/skills/status`, with `SKILL.md` and a binary
///   `logo.png`;
/// - the agent `coder`;
/// - the neutral hooks 0 (a Lua hook on `tool.before` matching read, edit and write) and 1 (a
///   message on `session.end`);
/// - the native file `x.md` and the native hook 0 (a Lua hook on `SubagentStop`).
pub(super) fn plan_input() -> Result<PlanInput, Box<dyn Error>> {
    let core = PackName::new("core")?;
    let status = ItemName::new("status")?;
    let coder = ItemName::new("coder")?;
    let claude = ToolName::new("claude")?;
    let skill_meta = SkillMeta::parse(
        Path::new("SKILL.md"),
        &document(
            "status",
            "description: Shows the status\ninvocation: user\n",
        ),
        &status,
    )
    .map_err(|found| format!("{found:?}"))?;
    let agent_meta = AgentMeta::parse(
        Path::new("coder.md"),
        &document("coder", "description: Writes code\nmodel: balanced\n"),
        &coder,
    )
    .map_err(|found| format!("{found:?}"))?;
    let skill = PlanSkill {
        id: ItemId::skill(&core, &status),
        name: status.clone(),
        dir: OpPath::new(".claude/skills/status")?,
        meta: skill_meta,
        files: vec![
            payload_file(
                ItemId::skill_file(&core, &status, &TreePath::from_parts(&["SKILL.md"])),
                "SKILL.md",
                Some("body"),
            ),
            payload_file(
                ItemId::skill_file(&core, &status, &TreePath::from_parts(&["logo.png"])),
                "logo.png",
                None,
            ),
        ],
    };
    let agent = PlanAgent {
        id: ItemId::agent(&core, &coder),
        name: coder.clone(),
        meta: agent_meta,
        text: "body".to_owned(),
    };
    let hooks = vec![
        PlanHook {
            id: ItemId::hook(&core, 0),
            event: HookEvent::ToolBefore,
            matches: Some(vec![MatchValue::Read, MatchValue::Edit, MatchValue::Write]),
            agents: None,
            action: PlanAction::Lua {
                may_block: false,
                command: vec!["airsl".to_owned(), "run".to_owned(), "x".to_owned()],
            },
        },
        PlanHook {
            id: ItemId::hook(&core, 1),
            event: HookEvent::SessionEnd,
            matches: None,
            agents: None,
            action: PlanAction::Message {
                text: "Reminder".to_owned(),
            },
        },
    ];
    let native = PlanNative {
        files: vec![payload_file(
            ItemId::native_file(&core, &claude, &TreePath::from_parts(&["x.md"])),
            "x.md",
            Some("x"),
        )],
        hooks: vec![PlanNativeHook {
            id: ItemId::native_hook(&core, 0),
            event: "SubagentStop".to_owned(),
            matches: Some(vec!["Edit".to_owned(), "Write".to_owned()]),
            action: PlanAction::Lua {
                may_block: false,
                command: vec!["airsl".to_owned(), "run".to_owned(), "y".to_owned()],
            },
        }],
    };
    Ok(PlanInput {
        tool: claude,
        root: OpPath::new(".claude/agentstacks")?,
        packs: vec![PlanPack {
            name: core,
            version: PackVersion::new("0.3.0")?,
            skills: vec![skill],
            agents: vec![agent],
            hooks,
            native,
        }],
    })
}

/// The `values` payload model of the pack `core` 0.3.0 for the tool `claude`: the skill `status`,
/// the agent `coder`, and the vars `session_id` and `tmpdir`.
pub(super) fn values_input() -> Result<ValuesInput, Box<dyn Error>> {
    Ok(ValuesInput {
        tool: ToolName::new("claude")?,
        packs: vec![ValuesPack {
            name: PackName::new("core")?,
            version: PackVersion::new("0.3.0")?,
            skills: vec![ItemName::new("status")?],
            agents: vec![ItemName::new("coder")?],
        }],
        vars: vec![Var::SessionId, Var::Tmpdir],
    })
}

/// An extension for `claude` that declares `reads`, whose entry file `adapter.lua` is `entry` and
/// that holds `files`, each a path and its bytes, besides it. It is returned loaded with the tree
/// it was loaded from.
pub(super) fn extension(
    entry: &str,
    reads: &[&str],
    files: &[(&str, &[u8])],
) -> Result<(Extension, FakeFileTree), Box<dyn Error>> {
    let mut all: Vec<(&str, &[u8])> = vec![("adapter.lua", entry.as_bytes())];
    all.extend_from_slice(files);
    let tree = tree(reads, &all);
    let loaded = load(&tree)?;
    Ok((loaded, tree))
}
