//! The plan flow itself: the ten steps of building a reviewable plan, in order.
//!
//! # Responsibilities
//!
//! - [`plan()`] reads the project and the lock, fetches and loads every source, runs each tool's
//!   extension, looks up the binaries and the `airsl` version, then decides the step of every path
//!   and builds the new lock. It collects every refusal of the steps that ran and returns them
//!   all, never only the first.
//! - Deciding the paths (steps 8 and 9) runs only when everything before it succeeded, so no path
//!   is resolved against a half-built plan and no file is falsely marked for deletion.
//!
//! # Non-responsibilities
//!
//! Each step lives in its own file; this one only orders them and assembles their results.
//! Showing the plan, and writing it, belong to the commands and the apply flow.

use std::path::Path;

use crate::app::plan::binaries::{Binaries, check_binaries};
use crate::app::plan::load::{Loaded, load_set};
use crate::app::plan::read::{Inputs, LOCK_FILE, read_inputs, read_inputs_text};
use crate::app::plan::sources::fetch_sources;
use crate::app::plan::steps::decide_steps;
use crate::app::plan::tools::run_tools;
use crate::port::{Adapter, Binary, FileTree, Git, Workspace};
use crate::types::lock::{LockedEntry, Lockfile, check_airsl_presence};
use crate::types::manifest::PackManifest;
use crate::types::op::OpPath;
use crate::types::plan::{
    DiskState, Landed, Link, Plan, Resolution, Skipped, Step, airsl_pin, lock_contents,
};
use crate::types::project::{LocalPath, SourceError};
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::SourceKey;
use crate::types::tree::tree_hash;

/// Builds the plan for the project, from the project file and the lock to a step for every path.
///
/// The flow runs in this order:
///
/// 1. It reads `agentstacks.toml` and `agentstacks.lock`.
/// 2. It reconciles the lock with the project and fetches every source: a locked git source at
///    the locked commit, a new or changed one by its pin, a path source from the project, and
///    each removed tool at its locked commit.
/// 3. It checks each fetched git tree against the lock.
/// 4. It loads every pack and extension and checks the set as a whole.
/// 5. It runs the extension of each tool in name order, then of each removed tool, over packs
///    rendered with the values that tool gave, recording the core's own writes first.
/// 6. It checks that every item got an operation and that Lua hooks come with their translator.
/// 7. It looks up the binaries the packs and the extensions need, and the installed `airsl`.
/// 8. It resolves every proposed path and decides its step.
/// 9. It decides the step of every locked path nothing proposes now.
/// 10. It builds the new lock and the plan.
///
/// A refusal of steps 1 to 7 ends the flow before step 8.
///
/// # Errors
///
/// Every refusal the steps that ran found, together. Callers show them with
/// [`Diagnostics::into_sorted`]. No plan is returned alongside a refusal, so a plan that reaches
/// apply holds none.
pub fn plan<W: Workspace, G: Git, B: Binary, A: Adapter>(
    workspace: &W,
    git: &G,
    binary: &B,
    adapter: &A,
) -> Result<Plan, Diagnostics> {
    let inputs = read_inputs(workspace)?;
    build(workspace, git, binary, adapter, &inputs)
}

/// Builds the plan for `project_text` instead of the project's own file.
///
/// Path sources resolve against `workspace` and its lock is the base. Nothing is written.
/// This is what an `init --dry-run` previews.
///
/// # Errors
///
/// The refusals of parsing `project_text` (including a legacy `api`), of reading the lock,
/// and of every later plan step, as [`plan`] returns them.
pub fn plan_rendered<W: Workspace, G: Git, B: Binary, A: Adapter>(
    workspace: &W,
    git: &G,
    binary: &B,
    adapter: &A,
    project_text: &str,
) -> Result<Plan, Diagnostics> {
    let inputs = read_inputs_text(project_text)?;
    build(&Fresh(workspace), git, binary, adapter, &inputs)
}

/// A workspace that reads sources from the real project but sees no lock and no file at
/// any target path, so a preview plans a fresh project while path sources still resolve.
struct Fresh<'a, W>(&'a W);

impl<W: Workspace> Workspace for Fresh<'_, W> {
    type Tree = W::Tree;

    fn project_text(&self) -> std::io::Result<String> {
        self.0.project_text()
    }

    fn lock_text(&self) -> std::io::Result<Option<String>> {
        Ok(None)
    }

    fn resolve(&self, _path: &OpPath) -> std::io::Result<Resolution> {
        Ok(Resolution::Inside {
            link: Link::Direct,
            state: DiskState::Absent,
        })
    }

    fn source_tree(&self, dir: &LocalPath) -> std::io::Result<Option<Self::Tree>> {
        self.0.source_tree(dir)
    }

    fn stage(&self, path: &OpPath, bytes: &[u8]) -> std::io::Result<()> {
        self.0.stage(path, bytes)
    }

    fn discard(&self, path: &OpPath) -> std::io::Result<()> {
        self.0.discard(path)
    }

    fn swap(&self, path: &OpPath) -> std::io::Result<()> {
        self.0.swap(path)
    }

    fn delete(&self, path: &OpPath) -> std::io::Result<()> {
        self.0.delete(path)
    }

    fn write_project(&self, text: &str) -> std::io::Result<()> {
        self.0.write_project(text)
    }

    fn write_lock(&self, text: &str) -> std::io::Result<()> {
        self.0.write_lock(text)
    }
}

/// Steps 2 to 10 of the flow, over inputs read by the caller.
fn build<W: Workspace, G: Git, B: Binary, A: Adapter>(
    workspace: &W,
    git: &G,
    binary: &B,
    adapter: &A,
    inputs: &Inputs,
) -> Result<Plan, Diagnostics> {
    let fetched = fetch_sources(inputs, workspace, git);
    let loaded = load_set(inputs, fetched)?;
    let mut ran = run_tools(&loaded, workspace, adapter);
    let mut binaries = check_binaries(inputs, &loaded, &ran, binary);

    let mut refusals = std::mem::take(&mut ran.refusals);
    refusals.extend(std::mem::take(&mut binaries.refusals));
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let decided = decide_steps(inputs, &ran, workspace);
    if !decided.refusals.is_empty() {
        return Err(decided.refusals);
    }

    let lock = build_lock(&loaded, &binaries, &decided.steps)?;
    let skips: Vec<Skipped> = ran.skips;
    Ok(Plan::new(
        decided.steps,
        skips,
        binaries.checks,
        loaded.notes,
        binaries.pin,
        lock,
        old_files(inputs),
    ))
}

/// The `[[file]]` entries of the lock the plan was made against; none without a lock.
fn old_files(inputs: &Inputs) -> Vec<crate::types::lock::FileEntry> {
    inputs
        .lock
        .as_ref()
        .map_or_else(Vec::new, |lock| lock.files().to_vec())
}

/// The lock a full apply writes: the resolved tools and packs, the file each step installs, and
/// the installed `airsl` when a pack needs it.
fn build_lock<T: FileTree>(
    loaded: &Loaded<T>,
    binaries: &Binaries,
    steps: &[Step],
) -> Result<Lockfile, Diagnostics> {
    let lock_file = Path::new(LOCK_FILE);
    let mut refusals = Diagnostics::new();

    let mut tools = Vec::new();
    for (name, extension) in loaded.set.extensions() {
        if let Some(entry) = loaded.entries.get(&SourceKey::Tool(name.clone())) {
            tools.push(LockedEntry::new(
                name.clone(),
                extension.manifest().version().clone(),
                entry.source.clone(),
                extension.tree().clone(),
            ));
        }
    }
    let mut packs = Vec::new();
    for (name, pack) in loaded.set.packs() {
        let Some(entry) = loaded.entries.get(&SourceKey::Pack(name.clone())) else {
            continue;
        };
        let hash = entry
            .hash
            .clone()
            .map_or_else(|| tree_hash(&entry.tree), Ok);
        match hash {
            Ok(hash) => packs.push(LockedEntry::new(
                name.clone(),
                pack.manifest().version().clone(),
                entry.source.clone(),
                hash,
            )),
            Err(found) => refusals.extend(found),
        }
    }
    let manifests: Vec<&PackManifest> = loaded
        .set
        .packs()
        .values()
        .map(crate::types::pack::Pack::manifest)
        .collect();
    let contents = match lock_contents(
        tools,
        packs,
        steps,
        airsl_pin(manifests.iter().copied(), binaries.installed.as_ref()),
        Landed::All,
    ) {
        Ok(contents) => contents,
        Err(error) => {
            refusals.push(Diagnostic::new(
                Location::file(LOCK_FILE),
                SourceError::from(error),
            ));
            return Err(refusals);
        }
    };
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let lock = Lockfile::new(lock_file, contents)?;
    check_airsl_presence(lock_file, &lock, manifests.iter().copied()).map_err(Diagnostics::from)?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::testing::{
        REV_A, Scenario, TestResult, URL, copy_step, ext_files_reading, git_locked, git_tag,
        hash_of, item, lock_text, locked_file, locked_pack, locked_tool, op, pack, pack_files,
        path_locked, read_step, rev, shown, skill_file_id, url, values_over, write_step,
    };
    use crate::port::{FakeFileTree, ScriptStep, ToolScript, WorkspaceCall};
    use crate::types::lock::{FileEntry, LockContents};
    use crate::types::manifest::PackVersion;
    use crate::types::op::ItemId;
    use crate::types::pack::Pack;
    use crate::types::plan::Action;
    use crate::types::resolve::FetchError;

    const CLAUDE_ONLY: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
    const STATUS_FILE: &str = ".claude/skills/status/SKILL.md";
    const AGENT_FILE: &str = ".claude/agents/coder.md";
    const SKILL_BYTES: &[u8] = b"---\nname: status\ndescription: d\n---\nBody\n";
    const VALUES: &str = r#"{"root":".claude/agentstacks","skills":{"core/status":{"invoke":"/status","dir":".claude/skills/status"}}}"#;

    fn core_tree(requires: &str) -> FakeFileTree {
        pack_files("packs/core", "core")
            .file(
                "pack.toml",
                format!(
                    "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n{requires}"
                )
                .as_bytes(),
            )
            .file(
                "agents/coder.md",
                b"---\nname: coder\ndescription: d\n---\nagent body\n",
            )
    }

    fn agent_id() -> Result<ItemId, Box<dyn std::error::Error>> {
        Ok(ItemId::agent(&pack("core")?, &item("coder")?))
    }

    /// The steps that account for the skill file and the agent.
    fn standard_steps() -> Result<Vec<ScriptStep>, Box<dyn std::error::Error>> {
        Ok(vec![
            copy_step(
                "claude",
                1,
                STATUS_FILE,
                skill_file_id("core", "status", "SKILL.md")?,
            )?,
            write_step("claude", 2, AGENT_FILE, b"agent", vec![agent_id()?])?,
        ])
    }

    struct World {
        scenario: Scenario,
        pack: Pack,
    }

    impl World {
        fn new(
            project: &str,
            pack_tree: FakeFileTree,
            reads: &[&str],
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let pack = Pack::load(&pack_tree).map_err(|found| shown(found).join("; "))?;
            let scenario = Scenario::new(project)
                .path_source("packs/core", pack_tree)?
                .path_source(
                    "extensions/claude",
                    ext_files_reading("extensions/claude", "claude", reads),
                )?;
            Ok(Self { scenario, pack })
        }

        fn standard() -> Result<Self, Box<dyn std::error::Error>> {
            Self::new(CLAUDE_ONLY, core_tree(""), &[])?.claude(standard_steps()?)
        }

        fn claude(mut self, steps: Vec<ScriptStep>) -> Result<Self, Box<dyn std::error::Error>> {
            let values = values_over(&[&self.pack], "claude", VALUES)?;
            let mut script = ToolScript::new().values(Ok(values));
            for step in steps {
                script = script.step(step);
            }
            let claude = crate::app::plan::testing::tool("claude")?;
            self.scenario = self
                .scenario
                .adapter(|adapter| adapter.tool(&claude, script));
            Ok(self)
        }

        fn edit(mut self, edit: impl FnOnce(Scenario) -> Scenario) -> Self {
            self.scenario = edit(self.scenario);
            self
        }

        fn plan(&self) -> Result<Plan, Diagnostics> {
            plan(
                &self.scenario.workspace,
                &self.scenario.git,
                &self.scenario.binary,
                &self.scenario.adapter,
            )
        }

        fn plans(&self) -> Result<Plan, Box<dyn std::error::Error>> {
            self.plan().map_err(|found| shown(found).join("; ").into())
        }
    }

    fn lock_with(
        files: Vec<FileEntry>,
        airsl: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        lock_text(LockContents {
            airsl: airsl.map(PackVersion::new).transpose()?,
            packs: vec![locked_pack(
                "core",
                path_locked("packs/core")?,
                hash_of(&core_tree(""))?,
            )?],
            files,
            ..LockContents::default()
        })
    }

    fn action_of(plan: &Plan, path: &str) -> Option<Action> {
        plan.steps()
            .iter()
            .find(|step| step.path.as_str() == path)
            .map(|step| step.action)
    }

    #[test]
    fn plan_creates_an_absent_skill_file_on_first_apply() -> TestResult {
        let plan = World::standard()?.plans()?;
        assert_eq!(action_of(&plan, STATUS_FILE), Some(Action::Create));
        let entry = plan
            .lock()
            .files()
            .iter()
            .find(|entry| entry.path().as_str() == STATUS_FILE)
            .ok_or("the locked file")?;
        assert_eq!(entry.by().as_str(), "core");
        Ok(())
    }

    #[test]
    fn legacy_project_refused() -> TestResult {
        let legacy = CLAUDE_ONLY.replacen("api = 1", "api = 0", 1);
        assert_ne!(legacy, CLAUDE_ONLY);
        let world = World::new(&legacy, core_tree(""), &[])?;
        let found = world.plan().err().ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            ["agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)"]
        );
        Ok(())
    }

    #[test]
    fn plan_refuses_instead_of_deleting_when_an_extension_fails_without_a_reason() -> TestResult {
        let lock = lock_with(vec![locked_file(AGENT_FILE, b"agent", "core")?], None)?;
        let agent = op(AGENT_FILE)?;
        let world = World::new(CLAUDE_ONLY, core_tree(""), &[])?
            .edit(|s| s.lock(&lock).workspace(|w| w.file(&agent, b"agent")));
        let found = world.plan().err().ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            ["claude: extension failed without giving a reason"]
        );
        Ok(())
    }

    #[test]
    fn plan_refuses_a_skill_file_edited_after_apply() -> TestResult {
        let lock = lock_with(vec![locked_file(STATUS_FILE, b"old", "core")?], None)?;
        let status = op(STATUS_FILE)?;
        let world =
            World::standard()?.edit(|s| s.lock(&lock).workspace(|w| w.file(&status, b"user edit")));
        let found = world.plan().err().ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            [format!("{STATUS_FILE}: changed since the last apply")]
        );
        Ok(())
    }

    #[test]
    fn plan_restores_a_deleted_skill_file() -> TestResult {
        let lock = lock_with(vec![locked_file(STATUS_FILE, SKILL_BYTES, "core")?], None)?;
        let plan = World::standard()?.edit(|s| s.lock(&lock)).plans()?;
        assert_eq!(action_of(&plan, STATUS_FILE), Some(Action::Restore));
        Ok(())
    }

    #[test]
    fn plan_deletes_the_untouched_file_of_a_removed_pack() -> TestResult {
        let lint_tree = FakeFileTree::new("packs/team-lint");
        let lock = lock_text(LockContents {
            packs: vec![locked_pack(
                "team-lint",
                path_locked("packs/team-lint")?,
                hash_of(&lint_tree)?,
            )?],
            files: vec![locked_file(
                ".claude/skills/lint/SKILL.md",
                b"lint",
                "team-lint",
            )?],
            ..LockContents::default()
        })?;
        let lint_file = op(".claude/skills/lint/SKILL.md")?;
        let world =
            World::standard()?.edit(|s| s.lock(&lock).workspace(|w| w.file(&lint_file, b"lint")));
        let plan = world.plans()?;
        assert_eq!(
            action_of(&plan, ".claude/skills/lint/SKILL.md"),
            Some(Action::Delete)
        );
        assert!(
            plan.lock()
                .packs()
                .iter()
                .all(|p| p.name().as_str() != "team-lint")
        );
        assert!(
            plan.lock()
                .files()
                .iter()
                .all(|file| file.path().as_str() != ".claude/skills/lint/SKILL.md")
        );
        Ok(())
    }

    fn settings_world(user_settings: Option<&[u8]>) -> Result<World, Box<dyn std::error::Error>> {
        let mut steps = standard_steps()?;
        steps.push(read_step(".claude/settings.json")?);
        steps.push(write_step(
            "claude",
            5,
            ".claude/settings.json",
            b"{\"hooks\":1}",
            vec![],
        )?);
        let settings = op(".claude/settings.json")?;
        let user = user_settings.map(<[u8]>::to_vec);
        Ok(
            World::new(CLAUDE_ONLY, core_tree(""), &[".claude/settings.json"])?
                .claude(steps)?
                .edit(|s| match user {
                    Some(bytes) => s.workspace(|w| w.file(&settings, &bytes)),
                    None => s,
                }),
        )
    }

    #[test]
    fn plan_updates_a_shared_settings_file_without_locking_it() -> TestResult {
        let plan = settings_world(Some(b"{\"permissions\":1}"))?.plans()?;
        let step = plan
            .steps()
            .iter()
            .find(|step| step.path.as_str() == ".claude/settings.json")
            .ok_or("the settings step")?;
        assert_eq!(step.action, Action::Update);
        assert_eq!(step.old.as_deref(), Some(&b"{\"permissions\":1}"[..]));
        assert!(
            plan.lock()
                .files()
                .iter()
                .all(|file| file.path().as_str() != ".claude/settings.json")
        );
        Ok(())
    }

    #[test]
    fn plan_creates_an_absent_shared_settings_file() -> TestResult {
        let plan = settings_world(None)?.plans()?;
        assert_eq!(
            action_of(&plan, ".claude/settings.json"),
            Some(Action::Create)
        );
        assert!(
            plan.lock()
                .files()
                .iter()
                .all(|file| file.path().as_str() != ".claude/settings.json")
        );
        Ok(())
    }

    #[test]
    fn plan_refuses_an_agent_file_not_written_by_agentstacks() -> TestResult {
        let agent = op(AGENT_FILE)?;
        let world = World::standard()?.edit(|s| s.workspace(|w| w.file(&agent, b"the user's own")));
        assert_eq!(
            shown(world.plan().err().ok_or("a refusal")?),
            [format!(
                "{AGENT_FILE}: exists and was not written by agentstacks"
            )]
        );
        Ok(())
    }

    #[test]
    fn plan_refuses_a_proposed_symlink_to_etc_hosts() -> TestResult {
        let agent = op(AGENT_FILE)?;
        let world = World::standard()?.edit(|s| s.workspace(|w| w.outside(&agent)));
        assert_eq!(
            shown(world.plan().err().ok_or("a refusal")?),
            [format!("{AGENT_FILE}: resolves outside the project")]
        );
        Ok(())
    }

    /// A world whose lock holds the removed tool `opencode` and `.agents/old.md`, which it wrote,
    /// and whose extension reads and rewrites `.agents/config.json`.
    fn removal_world() -> Result<(World, FakeFileTree), Box<dyn std::error::Error>> {
        let snapshot =
            ext_files_reading("extensions/opencode", "opencode", &[".agents/config.json"]);
        let lock = lock_text(LockContents {
            tools: vec![locked_tool(
                "opencode",
                git_locked(REV_A, None, None)?,
                hash_of(&snapshot)?,
            )?],
            files: vec![locked_file(".agents/old.md", b"old", "opencode")?],
            ..LockContents::default()
        })?;
        let (remote, at) = (url()?, rev(REV_A)?);
        let (config, old) = (op(".agents/config.json")?, op(".agents/old.md")?);
        let values = values_over(&[], "opencode", r#"{"root":".agents"}"#)?;
        let script = ToolScript::new()
            .values(Ok(values))
            .step(read_step(".agents/config.json")?)
            .step(write_step(
                "opencode",
                3,
                ".agents/config.json",
                b"{\"a\":1}",
                vec![],
            )?);
        let opencode = crate::app::plan::testing::tool("opencode")?;
        let tree = snapshot.clone();
        let world = World::standard()?.edit(|s| {
            s.lock(&lock)
                .git(|git| git.files(&remote, &at, None, tree))
                .workspace(|w| w.file(&config, b"{\"a\":1,\"x\":2}").file(&old, b"old"))
                .adapter(|adapter| adapter.tool(&opencode, script))
        });
        Ok((world, snapshot))
    }

    #[test]
    fn plan_removes_a_tool_by_running_its_locked_extension() -> TestResult {
        let (world, _) = removal_world()?;
        let plan = world.plans()?;
        assert_eq!(
            action_of(&plan, ".agents/config.json"),
            Some(Action::Update)
        );
        assert_eq!(action_of(&plan, ".agents/old.md"), Some(Action::Delete));
        assert!(
            plan.lock()
                .tools()
                .iter()
                .all(|t| t.name().as_str() != "opencode")
        );
        assert!(
            plan.lock()
                .files()
                .iter()
                .all(|file| !file.path().as_str().starts_with(".agents/"))
        );
        Ok(())
    }

    #[test]
    fn plan_refuses_while_a_removed_tool_source_is_unreachable() -> TestResult {
        let (world, _) = removal_world()?;
        let remote = url()?;
        let error = FetchError::GitFailed {
            subcommand: "fetch".to_owned(),
            stderr: "boom".to_owned(),
        };
        let world = world.edit(|s| s.git(|git| git.fail(&remote, error)));
        let found = world.plan().err().ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            ["agentstacks.toml: tools.opencode: git fetch failed: boom"]
        );
        Ok(())
    }

    #[test]
    fn plan_skips_deciding_after_an_earlier_refusal() -> TestResult {
        let lock = lock_with(vec![locked_file(STATUS_FILE, SKILL_BYTES, "core")?], None)?;
        let refuse = ScriptStep::Refuse(Diagnostic::new(
            Location::file("claude/adapter.lua"),
            crate::types::op::OpError::NoPlanHandler,
        ));
        let world = World::new(CLAUDE_ONLY, core_tree(""), &[])?
            .claude(vec![refuse])?
            .edit(|s| s.lock(&lock));
        assert_eq!(
            shown(world.plan().err().ok_or("a refusal")?),
            ["claude/adapter.lua: registers no plan handler"]
        );
        assert!(
            !world
                .scenario
                .workspace
                .calls()
                .iter()
                .any(|call| matches!(call, WorkspaceCall::Resolve(_)))
        );
        Ok(())
    }

    #[test]
    fn plan_deletes_a_skill_a_new_pack_version_dropped() -> TestResult {
        let lock = lock_with(
            vec![locked_file(".claude/skills/old/SKILL.md", b"old", "core")?],
            None,
        )?;
        let old = op(".claude/skills/old/SKILL.md")?;
        let plan = World::standard()?
            .edit(|s| s.lock(&lock).workspace(|w| w.file(&old, b"old")))
            .plans()?;
        assert_eq!(
            action_of(&plan, ".claude/skills/old/SKILL.md"),
            Some(Action::Delete)
        );
        assert!(
            plan.lock()
                .files()
                .iter()
                .all(|file| file.path().as_str() != ".claude/skills/old/SKILL.md")
        );
        Ok(())
    }

    #[test]
    fn plan_locks_resolved_revs_trees_and_airsl() -> TestResult {
        let requires = "[requires]\nairsl = \">=0.1.4\"\n";
        let tree = core_tree(requires);
        let project = format!(
            "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\ngit = \"{URL}\"\ntag = \"packs-v0.1.0\"\n"
        );
        let (remote, tag, at) = (url()?, git_tag("packs-v0.1.0")?, rev(REV_A)?);
        let served = tree.clone();
        let world = World::new(&project, tree.clone(), &[])?
            .claude(standard_steps()?)?
            .edit(|s| {
                s.git(|git| {
                    git.tag(&remote, &tag, &at)
                        .files(&remote, &at, None, served)
                })
                .binary(|binary| binary.airsl("airsl 0.1.5"))
            });
        let plan = world.plans()?;
        let packs = plan.lock().packs();
        let core = packs.first().ok_or("the locked pack")?;
        assert_eq!(
            core.source(),
            &git_locked(REV_A, Some("packs-v0.1.0"), None)?
        );
        assert_eq!(core.tree(), &hash_of(&tree)?);
        let tools = plan.lock().tools();
        let claude = tools.first().ok_or("the locked tool")?;
        assert_eq!(claude.source(), &path_locked("extensions/claude")?);
        assert_eq!(
            claude.tree(),
            &hash_of(&ext_files_reading("extensions/claude", "claude", &[]))?
        );
        assert_eq!(plan.lock().airsl(), Some(&PackVersion::new("0.1.5")?));
        Ok(())
    }

    #[test]
    fn plan_carries_the_old_file_entries() -> TestResult {
        let fresh = World::standard()?.plans()?;
        assert!(fresh.old_files().is_empty());

        let entry = locked_file(STATUS_FILE, SKILL_BYTES, "core")?;
        let lock = lock_with(vec![entry.clone()], None)?;
        let plan = World::standard()?.edit(|s| s.lock(&lock)).plans()?;
        assert_eq!(plan.old_files(), [entry]);
        Ok(())
    }

    #[test]
    fn plan_checks_airsl_presence_on_the_new_lock_only() -> TestResult {
        let lock = lock_with(vec![], Some("0.1.4"))?;
        let plan = World::standard()?.edit(|s| s.lock(&lock)).plans()?;
        assert!(plan.lock().airsl().is_none());
        Ok(())
    }

    #[test]
    fn plan_collects_every_refusal_from_the_deciding_step_sorted() -> TestResult {
        let lock = lock_with(
            vec![
                locked_file(STATUS_FILE, b"old", "core")?,
                locked_file(AGENT_FILE, b"old", "core")?,
            ],
            None,
        )?;
        let (status, agent) = (op(STATUS_FILE)?, op(AGENT_FILE)?);
        let world = World::standard()?.edit(|s| {
            s.lock(&lock)
                .workspace(|w| w.file(&status, b"edit one").file(&agent, b"edit two"))
        });
        assert_eq!(
            shown(world.plan().err().ok_or("refusals")?),
            [
                format!("{AGENT_FILE}: changed since the last apply"),
                format!("{STATUS_FILE}: changed since the last apply"),
            ]
        );
        Ok(())
    }

    #[test]
    fn plan_with_any_refusal_returns_no_plan() -> TestResult {
        let agent = op(AGENT_FILE)?;
        let world = World::standard()?.edit(|s| s.workspace(|w| w.file(&agent, b"theirs")));
        assert!(world.plan().is_err());
        Ok(())
    }
}
