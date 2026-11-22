//! Steps 5 and 6 of the plan flow: running each tool's extension over the rendered packs and
//! checking what it proposed.
//!
//! # Responsibilities
//!
//! - [`run_tools`] runs the listed tools in name order, then the removed tools in name order. For a
//!   listed tool it opens the extension, asks for its values, renders the packs, records the
//!   core's own writes (each pack's root files and, when a pack has a Lua hook, the hook runtime),
//!   dispatches the `plan` handler, and checks that every item got an operation and that Lua
//!   hooks come with their translator.
//! - A removed tool runs once more with no packs and no core writes. Its proposals are checked
//!   against each other on their own, and only its writes to files it read are kept.
//! - Every refusal of an extension is collected, so one tool's failure hides no other's.
//! - [`Ran`] is the flow's carrier for the proposals, each tool's reads, the skips and, per tool,
//!   the final bytes of the files an extension copies.
//!
//! # Non-responsibilities
//!
//! Which proposals clash, which items are covered and which writes a removed tool may keep are
//! rules in `types`. Running Lua is the adapter's. Looking up the binaries the extensions asked
//! for belongs to the `binaries` step.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::app::plan::load::Loaded;
use crate::app::plan::read::PROJECT_FILE;
use crate::app::plan::render::render_packs;
use crate::port::{Adapter, FileTree, Workspace};
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::op::{
    ItemId, OpError, OpPath, Operation, PlanInput, Proposal, Proposals, ValuesInput,
    check_coverage, check_translate, root_file_write, runtime_needed, runtime_write,
};
use crate::types::pack::Pack;
use crate::types::plan::{Reads, Skipped, keep_shared_writes};
use crate::types::protocol::RUNTIME_LUA;
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::SourceKey;

/// What one tool's extension read in this run.
pub(super) struct ToolRun {
    /// What each `read` returned.
    pub(super) reads: Reads,
    /// The project files the extension declared it reads.
    pub(super) declared: Vec<OpPath>,
}

/// What running every tool's extension found.
pub(super) struct Ran {
    /// The accepted proposals of every tool, with the core's writes, in the order they were made.
    pub(super) proposals: Proposals,
    /// What each tool read, listed tools and removed tools alike.
    pub(super) runs: BTreeMap<ToolName, ToolRun>,
    /// The skips: the first reason each extension gave for an item, and the core's skips.
    pub(super) skips: Vec<Skipped>,
    /// The final bytes of every file an extension may copy, by the tool it was rendered for and
    /// its id: the same skill file renders differently for each tool.
    pub(super) files: BTreeMap<(ToolName, ItemId), Vec<u8>>,
    /// Every refusal of the extensions and of what they proposed.
    pub(super) refusals: Diagnostics,
}

/// Runs the extension of every tool of the set, then of every removed tool.
pub(super) fn run_tools<T: FileTree, W: Workspace, A: Adapter>(
    loaded: &Loaded<T>,
    workspace: &W,
    adapter: &A,
) -> Ran {
    let mut ran = Ran {
        proposals: Proposals::new(),
        runs: BTreeMap::new(),
        skips: Vec::new(),
        files: BTreeMap::new(),
        refusals: Diagnostics::new(),
    };
    let packs: Vec<&Pack> = loaded.set.packs().values().collect();
    for (name, extension) in loaded.set.extensions() {
        let Some(entry) = loaded.entries.get(&SourceKey::Tool(name.clone())) else {
            continue;
        };
        run_listed(
            &mut ran,
            loaded,
            (name, extension, &entry.tree),
            &packs,
            workspace,
            adapter,
        );
    }
    for (name, removed) in &loaded.removed {
        run_removed(
            &mut ran,
            (name, &removed.extension, &removed.entry.tree),
            workspace,
            adapter,
        );
    }
    ran
}

/// Runs the extension of a tool the project lists.
fn run_listed<T: FileTree, W: Workspace, A: Adapter>(
    ran: &mut Ran,
    loaded: &Loaded<T>,
    (name, extension, files): (&ToolName, &Extension, &impl FileTree),
    packs: &[&Pack],
    workspace: &W,
    adapter: &A,
) {
    let mut session = match adapter.open(extension, files) {
        Ok(session) => session,
        Err(found) => {
            ran.refusals.extend(with_reason(found, name));
            return;
        }
    };
    let values = match adapter.values(&mut session, &ValuesInput::new(name.clone(), packs)) {
        Ok(values) => values,
        Err(found) => {
            ran.refusals.extend(with_reason(found, name));
            return;
        }
    };
    let rendered = render_packs(loaded, name, &values);
    ran.refusals.extend(rendered.refusals);
    ran.files.extend(
        rendered
            .files
            .into_iter()
            .map(|(id, bytes)| ((name.clone(), id), bytes)),
    );
    ran.skips
        .extend(rendered.skips.into_iter().map(Skipped::Core));

    let root = &rendered.input.root;
    let project_file = Path::new(PROJECT_FILE);
    let mut core_writes: Vec<Proposal> = rendered
        .root_files
        .iter()
        .map(|file| root_file_write(root, &file.pack, &file.rel, file.bytes.clone()))
        .collect();
    if runtime_needed(packs, name) {
        core_writes.push(runtime_write(root, name, RUNTIME_LUA.as_bytes().to_vec()));
    }
    for write in core_writes {
        if let Err(refusal) = ran.proposals.record_core(name, write, project_file) {
            ran.refusals.push(refusal);
        }
    }

    match adapter.plan(&mut session, &rendered.input, workspace, &mut ran.proposals) {
        Ok(reads) => {
            let entry = entry_file(name, extension);
            ran.refusals.extend(check_coverage(
                name,
                &entry,
                &rendered.input,
                &ran.proposals,
            ));
            ran.refusals.extend(check_translate(
                name,
                &entry,
                &rendered.input,
                &ran.proposals,
            ));
            ran.skips.extend(extension_skips(&ran.proposals, name));
            ran.runs.insert(
                name.clone(),
                ToolRun {
                    reads,
                    declared: declared_reads(extension),
                },
            );
        }
        Err(found) => ran.refusals.extend(with_reason(found, name)),
    }
}

/// Runs the extension of a tool the project dropped, with no packs, so it can take its entries out
/// of the files it shared with the user.
fn run_removed<W: Workspace, A: Adapter>(
    ran: &mut Ran,
    (name, extension, files): (&ToolName, &Extension, &impl FileTree),
    workspace: &W,
    adapter: &A,
) {
    let mut session = match adapter.open(extension, files) {
        Ok(session) => session,
        Err(found) => {
            ran.refusals.extend(with_reason(found, name));
            return;
        }
    };
    let values = match adapter.values(&mut session, &ValuesInput::new(name.clone(), &[])) {
        Ok(values) => values,
        Err(found) => {
            ran.refusals.extend(with_reason(found, name));
            return;
        }
    };
    let input = PlanInput {
        tool: name.clone(),
        root: values.root().clone(),
        packs: Vec::new(),
    };
    let mut own = Proposals::new();
    let reads = match adapter.plan(&mut session, &input, workspace, &mut own) {
        Ok(reads) => reads,
        Err(found) => {
            ran.refusals.extend(with_reason(found, name));
            return;
        }
    };
    let declared = declared_reads(extension);
    let made: Vec<Proposal> = own.iter().cloned().collect();
    for kept in keep_shared_writes(made, &reads, &declared) {
        let location = kept
            .origin
            .location()
            .unwrap_or_else(|| Location::file(name.as_str()));
        if let Err(error) = ran.proposals.propose(kept) {
            ran.refusals.push(Diagnostic::new(location, error));
        }
    }
    ran.runs.insert(name.clone(), ToolRun { reads, declared });
}

/// The refusals of a failed extension call, or when it gave none, one refusal at the tool, so a
/// failure is never mistaken for a tool that proposed nothing.
fn with_reason(found: Diagnostics, name: &ToolName) -> Diagnostics {
    if found.is_empty() {
        Diagnostics::from(Diagnostic::new(
            Location::file(name.as_str()),
            OpError::NoReason,
        ))
    } else {
        found
    }
}

/// Where the extension's entry file is shown in a refusal: `<tool>/<entry>`.
fn entry_file(name: &ToolName, extension: &Extension) -> PathBuf {
    PathBuf::from(format!(
        "{name}/{}",
        extension.manifest().entry().normalized()
    ))
}

/// The project files the extension declared it reads, as operation paths.
fn declared_reads(extension: &Extension) -> Vec<OpPath> {
    extension
        .reads()
        .iter()
        .filter_map(|read| OpPath::new(&read.normalized()).ok())
        .collect()
}

/// The skips `name`'s extension proposed, one for each item with the reason it gave first.
fn extension_skips(proposals: &Proposals, name: &ToolName) -> Vec<Skipped> {
    let mut seen = BTreeSet::new();
    let mut skips = Vec::new();
    for proposal in proposals.for_tool(name) {
        let Operation::Skip { item, .. } = &proposal.op else {
            continue;
        };
        if !seen.insert(item.clone()) {
            continue;
        }
        if let Some(reason) = proposals.skip_reason(name, item) {
            skips.push(Skipped::Extension {
                item: item.clone(),
                reason: reason.clone(),
                origin: proposal.origin.clone(),
            });
        }
    }
    skips
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::read::read_inputs;
    use crate::app::plan::steps::decide_steps;
    use crate::app::plan::testing::{
        LoadedTrees, REV_A, Scenario, TestResult, copy_step, ext_files, ext_files_reading,
        git_locked, hash_of, hook_id, loaded, lock_text, locked_tool, op, pack, pack_files, rev,
        shown, single, skill_file_id, skip_step, tool, url, values_for, values_over, write_step,
    };
    use crate::port::{AdapterCall, FakeAdapter, FakeFileTree, ScriptStep, ToolScript};
    use crate::types::lock::LockContents;
    use crate::types::op::{Origin, owner};

    const CLAUDE_ONLY: &str = "api = 0\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
    const TWO_TOOLS: &str = "api = 0\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[tools.opencode]\npath = \"extensions/opencode\"\n\n[packs.core]\npath = \"packs/core\"\n";
    const LUA_HOOK: &str =
        "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/enforce.lua\"\n";
    const MESSAGE_HOOK: &str =
        "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"t\"\n";

    /// A pack with the skill `status`, whose body uses a var, and the root file
    /// `hooks/enforce.lua`.
    fn base(manifest_extra: &str) -> FakeFileTree {
        pack_files("packs/core", "core")
            .file(
                "pack.toml",
                format!(
                    "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 0\ndescription = \"d\"\n[requires]\nairsl = \">=0.1.4\"\n{manifest_extra}"
                )
                .as_bytes(),
            )
            .file(
                "skills/status/SKILL.md",
                b"---\nname: status\ndescription: d\n---\n{{var tmpdir}}\n",
            )
            .file("hooks/enforce.lua", b"return 1")
    }

    fn values_json(home: &str, root: &str) -> String {
        format!(
            r#"{{"root":"{root}","skills":{{"core/status":{{"invoke":"/status","dir":"{home}/skills/status"}}}},"vars":{{"tmpdir":"$TMPDIR"}}}}"#
        )
    }

    fn cover(tool_name: &str, home: &str) -> Result<ScriptStep, Box<dyn std::error::Error>> {
        copy_step(
            tool_name,
            1,
            &format!("{home}/skills/status/SKILL.md"),
            skill_file_id("core", "status", "SKILL.md")?,
        )
    }

    struct Lab {
        scenario: Scenario,
        loaded: LoadedTrees,
    }

    impl Lab {
        fn new(scenario: Scenario) -> Result<Self, Box<dyn std::error::Error>> {
            let loaded = loaded(&scenario).map_err(|found| shown(found).join("; "))?;
            Ok(Self { scenario, loaded })
        }

        fn standard(
            project: &str,
            pack_tree: FakeFileTree,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let mut scenario = single(project, pack_tree)?;
            if project.contains("[tools.opencode]") {
                scenario = scenario.path_source(
                    "extensions/opencode",
                    ext_files("extensions/opencode", "opencode"),
                )?;
            }
            Self::new(scenario)
        }

        fn script(
            mut self,
            name: &str,
            home: &str,
            root: &str,
            steps: Vec<ScriptStep>,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let values = values_for(&self.loaded.set, name, &values_json(home, root))?;
            let mut script = ToolScript::new().values(Ok(values));
            for step in steps {
                script = script.step(step);
            }
            let named = tool(name)?;
            self.scenario = self
                .scenario
                .adapter(|adapter| adapter.tool(&named, script));
            Ok(self)
        }

        fn claude(self, steps: Vec<ScriptStep>) -> Result<Self, Box<dyn std::error::Error>> {
            self.script("claude", ".claude", ".claude/agentstacks", steps)
        }

        fn opencode(self, steps: Vec<ScriptStep>) -> Result<Self, Box<dyn std::error::Error>> {
            self.script("opencode", ".agents", ".agents", steps)
        }

        fn run(&self) -> Ran {
            run_tools(
                &self.loaded,
                &self.scenario.workspace,
                &self.scenario.adapter,
            )
        }
    }

    fn calls(adapter: &FakeAdapter) -> Vec<String> {
        adapter
            .calls()
            .iter()
            .map(|call| match call {
                AdapterCall::Open(name) => format!("open {name}"),
                AdapterCall::Values(name, _) => format!("values {name}"),
                AdapterCall::Plan(name, _) => format!("plan {name}"),
            })
            .collect()
    }

    fn paths(ran: &Ran) -> Vec<String> {
        ran.proposals
            .iter()
            .filter_map(|proposal| proposal.op.path().map(ToString::to_string))
            .collect()
    }

    /// `scenario` plus a locked, removed tool `aider` whose extension reads `reads`.
    fn with_aider(
        scenario: Scenario,
        reads: &[&str],
    ) -> Result<Scenario, Box<dyn std::error::Error>> {
        let snapshot = ext_files_reading("extensions/aider", "aider", reads);
        let lock = lock_text(LockContents {
            tools: vec![locked_tool(
                "aider",
                git_locked(REV_A, None, None)?,
                hash_of(&snapshot)?,
            )?],
            ..LockContents::default()
        })?;
        let (remote, at) = (url()?, rev(REV_A)?);
        Ok(scenario
            .lock(&lock)
            .git(|git| git.files(&remote, &at, None, snapshot.clone())))
    }

    fn script_aider(lab: Lab, steps: Vec<ScriptStep>) -> Result<Lab, Box<dyn std::error::Error>> {
        let values = values_over(&[], "aider", r#"{"root":".aider"}"#)?;
        let mut script = ToolScript::new().values(Ok(values));
        for step in steps {
            script = script.step(step);
        }
        let named = tool("aider")?;
        Ok(Lab {
            scenario: lab.scenario.adapter(|adapter| adapter.tool(&named, script)),
            loaded: lab.loaded,
        })
    }

    #[test]
    fn run_tools_runs_tools_in_name_order() -> TestResult {
        let lab = Lab::standard(TWO_TOOLS, base(""))?
            .opencode(vec![cover("opencode", ".agents")?])?
            .claude(vec![cover("claude", ".claude")?])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert_eq!(
            calls(&lab.scenario.adapter),
            [
                "open claude",
                "values claude",
                "plan claude",
                "open opencode",
                "values opencode",
                "plan opencode"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_runs_removed_tools_after_listed_tools_in_name_order() -> TestResult {
        let scenario = with_aider(single(CLAUDE_ONLY, base(""))?, &[])?;
        let lab = script_aider(
            Lab::new(scenario)?.claude(vec![cover("claude", ".claude")?])?,
            vec![],
        )?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert_eq!(
            calls(&lab.scenario.adapter),
            [
                "open claude",
                "values claude",
                "plan claude",
                "open aider",
                "values aider",
                "plan aider"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_dispatches_plan_with_the_rendered_payload() -> TestResult {
        let lab =
            Lab::standard(CLAUDE_ONLY, base(""))?.claude(vec![cover("claude", ".claude")?])?;
        let _ = lab.run();
        let calls = lab.scenario.adapter.calls();
        let Some(AdapterCall::Plan(_, input)) = calls.last() else {
            return Err("a plan call".into());
        };
        let skill = input
            .packs
            .first()
            .and_then(|pack| pack.skills.first())
            .and_then(|skill| skill.files.first())
            .ok_or("the skill file")?;
        assert_eq!(skill.text.as_deref(), Some("$TMPDIR\n"));
        Ok(())
    }

    #[test]
    fn run_tools_records_core_writes_just_before_the_plan_dispatch() -> TestResult {
        let lab = Lab::standard(TWO_TOOLS, base(""))?
            .claude(vec![
                cover("claude", ".claude")?,
                write_step("claude", 2, ".claude/x", b"x", vec![])?,
            ])?
            .opencode(vec![cover("opencode", ".agents")?])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert_eq!(
            paths(&ran),
            [
                ".claude/agentstacks/core/hooks/enforce.lua",
                ".claude/skills/status/SKILL.md",
                ".claude/x",
                ".agents/core/hooks/enforce.lua",
                ".agents/skills/status/SKILL.md",
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_writes_root_files_under_the_values_root() -> TestResult {
        let lab =
            Lab::standard(CLAUDE_ONLY, base(""))?.claude(vec![cover("claude", ".claude")?])?;
        let ran = lab.run();
        let write = ran
            .proposals
            .iter()
            .find(|proposal| {
                proposal.op.path().map(OpPath::as_str)
                    == Some(".claude/agentstacks/core/hooks/enforce.lua")
            })
            .ok_or("the root file write")?;
        assert_eq!(
            write.origin,
            Origin::Core {
                pack: pack("core")?
            }
        );
        assert_eq!(owner(write).map(|o| o.to_string()).as_deref(), Some("core"));
        Ok(())
    }

    #[test]
    fn run_tools_writes_runtime_lua_even_when_the_lua_hook_is_skipped() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(LUA_HOOK))?.claude(vec![
            cover("claude", ".claude")?,
            skip_step("claude", 2, hook_id("core", 0)?, "not supported")?,
        ])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        let runtime = ran
            .proposals
            .iter()
            .find(|proposal| {
                proposal.op.path().map(OpPath::as_str) == Some(".claude/agentstacks/runtime.lua")
            })
            .ok_or("the runtime write")?;
        assert_eq!(
            owner(runtime).map(|o| o.to_string()).as_deref(),
            Some("claude")
        );
        Ok(())
    }

    #[test]
    fn run_tools_writes_no_runtime_lua_without_lua_hooks() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(MESSAGE_HOOK))?.claude(vec![
            cover("claude", ".claude")?,
            skip_step("claude", 2, hook_id("core", 0)?, "not supported")?,
        ])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert!(!paths(&ran).iter().any(|path| path.ends_with("runtime.lua")));
        Ok(())
    }

    #[test]
    fn run_tools_refuses_a_path_an_earlier_tool_proposed() -> TestResult {
        let lab = Lab::standard(TWO_TOOLS, base(""))?
            .claude(vec![
                cover("claude", ".claude")?,
                write_step("claude", 4, ".claude/x", b"x", vec![])?,
            ])?
            .opencode(vec![
                cover("opencode", ".agents")?,
                write_step("opencode", 5, ".claude/x", b"y", vec![])?,
            ])?;
        assert_eq!(
            shown(lab.run().refusals),
            ["opencode/adapter.lua:5: write: path \".claude/x\" is already proposed by claude"]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_an_extension_write_onto_a_root_file() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(""))?.claude(vec![
            cover("claude", ".claude")?,
            write_step(
                "claude",
                6,
                ".claude/agentstacks/core/hooks/enforce.lua",
                b"x",
                vec![],
            )?,
        ])?;
        assert_eq!(
            shown(lab.run().refusals),
            [
                "claude/adapter.lua:6: write: path \".claude/agentstacks/core/hooks/enforce.lua\" is already proposed by core (root file of core)"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_an_extension_write_onto_runtime_lua() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(LUA_HOOK))?.claude(vec![
            cover("claude", ".claude")?,
            skip_step("claude", 2, hook_id("core", 0)?, "not supported")?,
            write_step("claude", 8, ".claude/agentstacks/runtime.lua", b"x", vec![])?,
        ])?;
        assert_eq!(
            shown(lab.run().refusals),
            [
                "claude/adapter.lua:8: write: path \".claude/agentstacks/runtime.lua\" is already proposed by core (runtime.lua of claude)"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_a_core_write_onto_an_earlier_tools_path() -> TestResult {
        let lab = Lab::standard(TWO_TOOLS, base(""))?
            .claude(vec![
                cover("claude", ".claude")?,
                write_step("claude", 2, ".agents/core/hooks/enforce.lua", b"x", vec![])?,
            ])?
            .opencode(vec![cover("opencode", ".agents")?])?;
        assert_eq!(
            shown(lab.run().refusals),
            [
                "agentstacks.toml: tools.opencode: path \".agents/core/hooks/enforce.lua\" is already proposed by claude"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_an_uncovered_item() -> TestResult {
        let tree = base("").file("skills/status/references/x.md", b"reference");
        let lab = Lab::standard(CLAUDE_ONLY, tree)?.claude(vec![cover("claude", ".claude")?])?;
        assert_eq!(
            shown(lab.run().refusals),
            [
                "claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_lua_hooks_without_translate_lua() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(LUA_HOOK))?.claude(vec![
            cover("claude", ".claude")?,
            write_step(
                "claude",
                3,
                ".claude/hooks.json",
                b"{}",
                vec![hook_id("core", 0)?],
            )?,
        ])?;
        assert_eq!(
            shown(lab.run().refusals),
            ["claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)"]
        );
        Ok(())
    }

    #[test]
    fn run_tools_needs_no_translate_lua_when_every_lua_hook_is_skipped() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(LUA_HOOK))?.claude(vec![
            cover("claude", ".claude")?,
            skip_step("claude", 2, hook_id("core", 0)?, "not supported")?,
        ])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert!(ran.proposals.has_runtime(&tool("claude")?));
        Ok(())
    }

    #[test]
    fn run_tools_collects_adapter_refusals_from_every_tool() -> TestResult {
        let lab = Lab::standard(TWO_TOOLS, base(""))?;
        let no_values = {
            let mut found = Diagnostics::new();
            found.push(Diagnostic::new(
                Location::file("claude/adapter.lua"),
                OpError::NoValuesHandler,
            ));
            found
        };
        let refuse_plan = ScriptStep::Refuse(Diagnostic::new(
            Location::file("opencode/adapter.lua"),
            OpError::NoPlanHandler,
        ));
        let values = values_for(
            &lab.loaded.set,
            "opencode",
            &values_json(".agents", ".agents"),
        )?;
        let (claude, opencode) = (tool("claude")?, tool("opencode")?);
        let scenario = lab.scenario.adapter(|adapter| {
            adapter
                .tool(&claude, ToolScript::new().values(Err(no_values)))
                .tool(
                    &opencode,
                    ToolScript::new().values(Ok(values)).step(refuse_plan),
                )
        });
        let lab = Lab {
            scenario,
            loaded: lab.loaded,
        };
        assert_eq!(
            shown(lab.run().refusals),
            [
                "claude/adapter.lua: registers no values handler",
                "opencode/adapter.lua: registers no plan handler"
            ]
        );
        assert_eq!(
            calls(&lab.scenario.adapter),
            [
                "open claude",
                "values claude",
                "open opencode",
                "values opencode",
                "plan opencode"
            ]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_a_tool_whose_open_fails_without_a_reason() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(""))?;
        assert_eq!(
            shown(lab.run().refusals),
            ["claude: extension failed without giving a reason"]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_a_tool_whose_values_fail_without_a_reason() -> TestResult {
        let lab = Lab::standard(CLAUDE_ONLY, base(""))?;
        let claude = tool("claude")?;
        let scenario = lab.scenario.adapter(|adapter| {
            adapter.tool(&claude, ToolScript::new().values(Err(Diagnostics::new())))
        });
        let lab = Lab {
            scenario,
            loaded: lab.loaded,
        };
        assert_eq!(
            shown(lab.run().refusals),
            ["claude: extension failed without giving a reason"]
        );
        Ok(())
    }

    #[test]
    fn with_reason_keeps_the_refusals_it_was_given_and_names_none() -> TestResult {
        let name = tool("claude")?;
        assert_eq!(
            shown(with_reason(Diagnostics::new(), &name)),
            ["claude: extension failed without giving a reason"]
        );
        let given = Diagnostics::from(Diagnostic::new(
            Location::file("claude/adapter.lua"),
            OpError::NoPlanHandler,
        ));
        assert_eq!(
            shown(with_reason(given, &name)),
            ["claude/adapter.lua: registers no plan handler"]
        );
        Ok(())
    }

    #[test]
    fn run_tools_refuses_a_removed_tool_that_fails_without_a_reason() -> TestResult {
        let scenario = with_aider(single(CLAUDE_ONLY, base(""))?, &[])?;
        let lab = Lab::new(scenario)?.claude(vec![cover("claude", ".claude")?])?;
        assert_eq!(
            shown(lab.run().refusals),
            ["aider: extension failed without giving a reason"]
        );
        Ok(())
    }

    #[test]
    fn run_tools_runs_a_removed_tool_with_zero_packs() -> TestResult {
        let config = op(".aider/config.json")?;
        let scenario = with_aider(single(CLAUDE_ONLY, base(""))?, &[".aider/config.json"])?
            .workspace(|workspace| workspace.file(&config, b"{}"));
        let lab = Lab::new(scenario)?.claude(vec![cover("claude", ".claude")?])?;
        let lab = script_aider(
            lab,
            vec![
                ScriptStep::Read(config),
                write_step("aider", 2, ".aider/config.json", b"{\"a\":1}", vec![])?,
                write_step("aider", 3, ".aider/other.json", b"x", vec![])?,
            ],
        )?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        let adapter_calls = lab.scenario.adapter.calls();
        let values_input = adapter_calls
            .iter()
            .find_map(|call| match call {
                AdapterCall::Values(name, input) if name.as_str() == "aider" => Some(input),
                _ => None,
            })
            .ok_or("the aider values call")?;
        assert!(values_input.packs.is_empty());
        let aider: Vec<String> = ran
            .proposals
            .for_tool(&tool("aider")?)
            .filter_map(|proposal| proposal.op.path().map(ToString::to_string))
            .collect();
        assert_eq!(aider, [".aider/config.json"]);
        assert!(
            !paths(&ran)
                .iter()
                .any(|path| path.starts_with(".aider/core"))
        );
        Ok(())
    }

    #[test]
    fn run_tools_keeps_a_removed_tools_dropped_write_out_of_the_duplicate_check() -> TestResult {
        let scenario = with_aider(single(CLAUDE_ONLY, base(""))?, &[])?;
        let lab = Lab::new(scenario)?.claude(vec![cover("claude", ".claude")?])?;
        let lab = script_aider(
            lab,
            vec![write_step(
                "aider",
                2,
                ".claude/skills/status/SKILL.md",
                b"x",
                vec![],
            )?],
        )?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        assert_eq!(ran.proposals.for_tool(&tool("aider")?).count(), 0);
        Ok(())
    }

    #[test]
    fn run_tools_keeps_each_tools_rendered_bytes_for_the_files_it_copies() -> TestResult {
        let skill = "---\nname: status\ndescription: d\n---\ndir={{skill_dir core/status}} {{var tmpdir}}\n";
        let lab = Lab::standard(
            TWO_TOOLS,
            base("").file("skills/status/SKILL.md", skill.as_bytes()),
        )?
        .claude(vec![cover("claude", ".claude")?])?
        .opencode(vec![cover("opencode", ".agents")?])?;
        let ran = lab.run();
        assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
        let inputs =
            read_inputs(&lab.scenario.workspace).map_err(|found| shown(found).join("; "))?;
        let decided = decide_steps(&inputs, &ran, &lab.scenario.workspace);
        assert!(decided.refusals.is_empty(), "{:?}", decided.refusals);
        let bytes_of = |path: &str| {
            decided
                .steps
                .iter()
                .find(|step| step.path.as_str() == path)
                .and_then(|step| step.new.as_deref())
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        };
        assert_eq!(
            bytes_of(".claude/skills/status/SKILL.md").as_deref(),
            Some("---\nname: status\ndescription: d\n---\ndir=.claude/skills/status $TMPDIR\n")
        );
        assert_eq!(
            bytes_of(".agents/skills/status/SKILL.md").as_deref(),
            Some("---\nname: status\ndescription: d\n---\ndir=.agents/skills/status $TMPDIR\n")
        );
        Ok(())
    }
}
