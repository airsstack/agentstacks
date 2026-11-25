//! A scripted [`Adapter`] for tests.
//!
//! It plays a tool extension without Lua, so a flow is tested against exactly the answers and the
//! refusals a test lays out, and against the real rules for proposals and reads.
//!
//! # Responsibilities
//!
//! - [`FakeAdapter`] plays one [`ToolScript`] per tool: the refusal of opening it, the answer of
//!   its `values` handler, and the steps of its `plan` handler. The session it opens is the tool's
//!   name.
//! - [`ScriptStep`] is one step of a `plan` handler: a read, a proposal or a refusal. A read asks
//!   the workspace and records what came back as the real host does, and a proposal goes through
//!   [`Proposals::propose`], so the duplicate-path and skip and cover rules run exactly as they
//!   do with the real host.
//! - A tool scripted more than once serves its scripts in order, one for each run of the tool: the
//!   run moves on to the next script when its `plan` handler has been dispatched. The last script
//!   is used up by its first run: a later run opens, has `values` refused with no diagnostics and
//!   proposes no steps.
//! - A tool with no script is refused when opened, with no diagnostics, so a test that forgot to
//!   script it fails its exact-refusal assertion visibly.
//! - [`AdapterCall`] is one entry of the call log that [`FakeAdapter::calls`] returns, with a copy
//!   of the input each call was given.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests and under the `testing` feature.
//! How the real engine runs Lua is proven by the tests of the
//! `host` module. Each script answers once: the open refusal, the `values` answer and the `plan`
//! steps are taken when used.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

use crate::port::adapter::seam::Adapter;
use crate::port::tree::FileTree;
use crate::port::workspace::Workspace;
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::op::{OpPath, PlanInput, Proposal, Proposals, ValuesInput, ValuesResult};
use crate::types::plan::{DiskState, ReadOutcome, Reads, Resolution};
use crate::types::report::{Diagnostic, Diagnostics, Location};

/// One step of a scripted `plan` handler.
#[derive(Debug)]
pub enum ScriptStep {
    /// The handler reads `path` from the project.
    Read(OpPath),
    /// The handler proposes an operation.
    Propose(Proposal),
    /// The handler is refused with this diagnostic.
    Refuse(Diagnostic),
}

/// What one tool's extension does.
#[derive(Debug, Default)]
pub struct ToolScript {
    open: Option<Diagnostics>,
    values: Option<Result<ValuesResult, Diagnostics>>,
    steps: Vec<ScriptStep>,
}

impl ToolScript {
    /// A script that opens, has no `values` answer and does nothing in `plan`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes opening the extension fail with `refusals`.
    #[must_use]
    pub fn refuse_open(mut self, refusals: Diagnostics) -> Self {
        self.open = Some(refusals);
        self
    }

    /// Sets the answer of the `values` handler. Without one, `values` is refused with no
    /// diagnostics.
    #[must_use]
    pub fn values(mut self, answer: Result<ValuesResult, Diagnostics>) -> Self {
        self.values = Some(answer);
        self
    }

    /// Appends a step to the `plan` handler.
    #[must_use]
    pub fn step(mut self, step: ScriptStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// One call to a [`FakeAdapter`], with a copy of its input.
#[derive(Debug, Clone)]
pub enum AdapterCall {
    /// [`Adapter::open`] of a tool's extension.
    Open(ToolName),
    /// [`Adapter::values`] for a tool.
    Values(ToolName, ValuesInput),
    /// [`Adapter::plan`] for a tool.
    Plan(ToolName, PlanInput),
}

/// A scripted engine, keyed by the tool each extension is for.
#[derive(Debug, Default)]
pub struct FakeAdapter {
    scripts: RefCell<BTreeMap<ToolName, VecDeque<ToolScript>>>,
    calls: RefCell<Vec<AdapterCall>>,
}

impl FakeAdapter {
    /// An engine with no scripts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scripts the extension of `tool`. A second script for the same tool serves its next run.
    #[must_use]
    pub fn tool(self, tool: &ToolName, script: ToolScript) -> Self {
        self.scripts
            .borrow_mut()
            .entry(tool.clone())
            .or_default()
            .push_back(script);
        self
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<AdapterCall> {
        self.calls.borrow().clone()
    }
}

impl FakeAdapter {
    /// The `plan` steps of the script serving `tool`, which then makes way for the next script
    /// when one is queued.
    fn next_run_steps(&self, tool: &ToolName) -> Vec<ScriptStep> {
        let mut scripts = self.scripts.borrow_mut();
        let Some(queue) = scripts.get_mut(tool) else {
            return Vec::new();
        };
        let steps = queue
            .front_mut()
            .map(|script| std::mem::take(&mut script.steps))
            .unwrap_or_default();
        if queue.len() > 1 {
            queue.pop_front();
        }
        steps
    }
}

impl Adapter for FakeAdapter {
    type Session = ToolName;

    fn open<T: FileTree>(
        &self,
        extension: &Extension,
        _files: &T,
    ) -> Result<Self::Session, Diagnostics> {
        let tool = extension.manifest().name().clone();
        self.calls
            .borrow_mut()
            .push(AdapterCall::Open(tool.clone()));
        match self
            .scripts
            .borrow_mut()
            .get_mut(&tool)
            .and_then(VecDeque::front_mut)
        {
            None => Err(Diagnostics::new()),
            Some(script) => script.open.take().map_or(Ok(tool), Err),
        }
    }

    fn values(
        &self,
        session: &mut Self::Session,
        input: &ValuesInput,
    ) -> Result<ValuesResult, Diagnostics> {
        self.calls
            .borrow_mut()
            .push(AdapterCall::Values(session.clone(), input.clone()));
        self.scripts
            .borrow_mut()
            .get_mut(session)
            .and_then(VecDeque::front_mut)
            .and_then(|script| script.values.take())
            .unwrap_or_else(|| Err(Diagnostics::new()))
    }

    fn plan<W: Workspace>(
        &self,
        session: &mut Self::Session,
        input: &PlanInput,
        workspace: &W,
        proposals: &mut Proposals,
    ) -> Result<Reads, Diagnostics> {
        self.calls
            .borrow_mut()
            .push(AdapterCall::Plan(session.clone(), input.clone()));
        let steps = self.next_run_steps(session);
        let mut reads = Reads::default();
        let mut found = Diagnostics::new();
        for step in steps {
            match step {
                ScriptStep::Read(path) => match workspace.resolve(&path) {
                    Ok(Resolution::Inside {
                        state: DiskState::File(file),
                        ..
                    }) => reads.record(path, ReadOutcome::Bytes(file.hash().clone())),
                    Ok(Resolution::Inside { .. }) => reads.record(path, ReadOutcome::Absent),
                    Ok(Resolution::Outside { .. }) => {}
                    Err(error) => {
                        found.push(Diagnostic::new(Location::file(path.as_str()), error));
                    }
                },
                ScriptStep::Propose(proposal) => {
                    let location = proposal
                        .origin
                        .location()
                        .unwrap_or_else(|| Location::file(session.as_str()));
                    if let Err(error) = proposals.propose(proposal) {
                        found.push(Diagnostic::new(location, error));
                    }
                }
                ScriptStep::Refuse(diagnostic) => found.push(diagnostic),
            }
        }
        found.into_result(reads)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::Path;

    use serde_json::Value;

    use super::*;
    use crate::port::{FakeFileTree, FakeWorkspace};
    use crate::types::op::{LuaType, Operation, Origin};
    use crate::types::plan::ReadOutcome;
    use crate::types::report::{FieldError, Location};
    use crate::types::tree::{ContentHash, TreePath};

    type TestResult = Result<(), Box<dyn Error>>;

    fn tool(name: &str) -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new(name)?)
    }

    fn path(text: &str) -> Result<OpPath, Box<dyn Error>> {
        Ok(OpPath::new(text)?)
    }

    fn extension(name: &str) -> Result<(Extension, FakeFileTree), Box<dyn Error>> {
        let manifest = format!(
            "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\nentry = \"adapter.lua\"\n"
        );
        let tree = FakeFileTree::new(format!("extensions/{name}"))
            .file("extension.toml", manifest.as_bytes())
            .file("adapter.lua", b"");
        let loaded = Extension::load(&tree).map_err(|found| format!("{found:?}"))?;
        Ok((loaded, tree))
    }

    fn values_input(name: &str) -> Result<ValuesInput, Box<dyn Error>> {
        Ok(ValuesInput {
            tool: tool(name)?,
            packs: Vec::new(),
            vars: Vec::new(),
        })
    }

    fn plan_input(name: &str) -> Result<PlanInput, Box<dyn Error>> {
        Ok(PlanInput {
            tool: tool(name)?,
            root: path(".claude/agentstacks")?,
            packs: Vec::new(),
        })
    }

    fn values_result(name: &str) -> Result<ValuesResult, Box<dyn Error>> {
        let value: Value = serde_json::from_str(r#"{"root":".claude/agentstacks"}"#)?;
        let object = value.as_object().ok_or("an object")?;
        ValuesResult::read(
            object,
            &values_input(name)?,
            Path::new("claude/adapter.lua"),
            |_| LuaType::Table,
        )
        .map_err(|found| format!("{found:?}").into())
    }

    fn call_at(name: &str, line: usize) -> Result<Origin, Box<dyn Error>> {
        Ok(Origin::Extension {
            tool: tool(name)?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: NonZeroUsize::new(line).ok_or("a line starts at 1")?,
        })
    }

    fn write(name: &str, line: usize, target: &str) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::Write {
                path: path(target)?,
                content: b"x".to_vec(),
                covers: Vec::new(),
            },
            origin: call_at(name, line)?,
        })
    }

    fn refusal(text: &str) -> Diagnostic {
        Diagnostic::new(Location::file(text), FieldError::Required)
    }

    fn shown(found: Diagnostics) -> Vec<String> {
        found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn plain(found: Diagnostics) -> Box<dyn Error> {
        shown(found).join("; ").into()
    }

    fn drive<A: Adapter, W: Workspace>(
        adapter: &A,
        extension: &Extension,
        files: &FakeFileTree,
        workspace: &W,
    ) -> Result<usize, Box<dyn Error>> {
        let name = extension.manifest().name().as_str();
        let mut session = adapter
            .open(extension, files)
            .map_err(|found| format!("{found:?}"))?;
        adapter
            .values(&mut session, &values_input(name)?)
            .map_err(|found| format!("{found:?}"))?;
        let mut proposals = Proposals::new();
        adapter
            .plan(&mut session, &plan_input(name)?, workspace, &mut proposals)
            .map_err(|found| format!("{found:?}"))?;
        Ok(proposals.iter().count())
    }

    #[test]
    fn open_values_plan_follow_the_script() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new()
            .values(Ok(values_result("claude")?))
            .step(ScriptStep::Propose(write("claude", 3, ".claude/a")?));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        let workspace = FakeWorkspace::new();

        let mut session = adapter.open(&claude, &files).map_err(plain)?;
        let values = adapter
            .values(&mut session, &values_input("claude")?)
            .map_err(plain)?;
        assert_eq!(values, values_result("claude")?);
        let mut proposals = Proposals::new();
        let reads = adapter
            .plan(
                &mut session,
                &plan_input("claude")?,
                &workspace,
                &mut proposals,
            )
            .map_err(plain)?;
        assert_eq!(reads, Reads::default());
        let paths: Vec<Option<&str>> = proposals
            .iter()
            .map(|p| p.op.path().map(OpPath::as_str))
            .collect();
        assert_eq!(paths, [Some(".claude/a")]);
        Ok(())
    }

    #[test]
    fn plan_records_what_read_returned() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new()
            .step(ScriptStep::Read(path(".claude/settings.json")?))
            .step(ScriptStep::Read(path(".claude/none.json")?))
            .step(ScriptStep::Read(path("outside.json")?));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        let workspace = FakeWorkspace::new()
            .file(&path(".claude/settings.json")?, b"{}")
            .outside(&path("outside.json")?);

        let mut session = adapter.open(&claude, &files).map_err(plain)?;
        let reads = adapter
            .plan(
                &mut session,
                &plan_input("claude")?,
                &workspace,
                &mut Proposals::new(),
            )
            .map_err(plain)?;
        let every = [
            path(".claude/settings.json")?,
            path(".claude/none.json")?,
            path("outside.json")?,
        ];
        assert_eq!(
            reads.shared(&path(".claude/settings.json")?, &every),
            Some(ReadOutcome::Bytes(ContentHash::of_bytes(b"{}")))
        );
        assert_eq!(
            reads.shared(&path(".claude/none.json")?, &every),
            Some(ReadOutcome::Absent)
        );
        assert_eq!(reads.shared(&path("outside.json")?, &every), None);
        Ok(())
    }

    #[test]
    fn propose_runs_the_duplicate_rule() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new().step(ScriptStep::Propose(write("claude", 7, ".claude/x")?));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        let mut proposals = Proposals::new();
        proposals.propose(write("opencode", 2, ".claude/x")?)?;

        let mut session = adapter.open(&claude, &files).map_err(plain)?;
        let found = adapter
            .plan(
                &mut session,
                &plan_input("claude")?,
                &FakeWorkspace::new(),
                &mut proposals,
            )
            .err()
            .ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            ["claude/adapter.lua:7: write: path \".claude/x\" is already proposed by opencode"]
        );
        assert_eq!(proposals.iter().count(), 1);
        Ok(())
    }

    #[test]
    fn refuse_collects_and_fails_the_call() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new()
            .step(ScriptStep::Refuse(refusal("a")))
            .step(ScriptStep::Propose(write("claude", 3, ".claude/a")?))
            .step(ScriptStep::Refuse(refusal("b")));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        let mut session = adapter.open(&claude, &files).map_err(plain)?;
        let found = adapter
            .plan(
                &mut session,
                &plan_input("claude")?,
                &FakeWorkspace::new(),
                &mut Proposals::new(),
            )
            .err()
            .ok_or("refusals")?;
        assert_eq!(shown(found), ["a: required", "b: required"]);
        Ok(())
    }

    #[test]
    fn open_refusal_is_returned() -> TestResult {
        let (claude, files) = extension("claude")?;
        let mut found = Diagnostics::new();
        found.push(refusal("claude/adapter.lua"));
        let adapter =
            FakeAdapter::new().tool(&tool("claude")?, ToolScript::new().refuse_open(found));
        let refused = adapter.open(&claude, &files).err().ok_or("a refusal")?;
        assert_eq!(shown(refused), ["claude/adapter.lua: required"]);
        assert_eq!(adapter.calls().len(), 1);
        assert!(matches!(
            adapter.calls().first(),
            Some(AdapterCall::Open(_))
        ));
        Ok(())
    }

    #[test]
    fn unscripted_tool_is_refused_empty() -> TestResult {
        let (other, files) = extension("other")?;
        let adapter = FakeAdapter::new();
        let refused = adapter.open(&other, &files).err().ok_or("a refusal")?;
        assert!(refused.is_empty());
        Ok(())
    }

    #[test]
    fn calls_log_inputs_in_order() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new().values(Ok(values_result("claude")?));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        let mut session = adapter.open(&claude, &files).map_err(plain)?;
        let _ = adapter.values(&mut session, &values_input("claude")?);
        let _ = adapter.plan(
            &mut session,
            &plan_input("claude")?,
            &FakeWorkspace::new(),
            &mut Proposals::new(),
        );
        let calls = adapter.calls();
        let [
            AdapterCall::Open(opened),
            AdapterCall::Values(by_values, values),
            AdapterCall::Plan(by_plan, plan),
        ] = calls.as_slice()
        else {
            return Err(format!("{calls:?}").into());
        };
        assert_eq!(opened, &tool("claude")?);
        assert_eq!(by_values, opened);
        assert_eq!(by_plan, opened);
        assert_eq!(values, &values_input("claude")?);
        assert_eq!(plan.root, path(".claude/agentstacks")?);
        Ok(())
    }

    #[test]
    fn a_second_script_serves_the_next_run() -> TestResult {
        let (claude, files) = extension("claude")?;
        let first = ToolScript::new()
            .values(Ok(values_result("claude")?))
            .step(ScriptStep::Propose(write("claude", 1, ".claude/a")?));
        let second = ToolScript::new()
            .values(Ok(values_result("claude")?))
            .step(ScriptStep::Propose(write("claude", 2, ".claude/b")?))
            .step(ScriptStep::Propose(write("claude", 3, ".claude/c")?));
        let adapter = FakeAdapter::new()
            .tool(&tool("claude")?, first)
            .tool(&tool("claude")?, second);
        let workspace = FakeWorkspace::new();
        assert_eq!(drive(&adapter, &claude, &files, &workspace)?, 1);
        assert_eq!(drive(&adapter, &claude, &files, &workspace)?, 2);
        Ok(())
    }

    #[test]
    fn fake_is_usable_through_a_generic_bound() -> TestResult {
        let (claude, files) = extension("claude")?;
        let script = ToolScript::new()
            .values(Ok(values_result("claude")?))
            .step(ScriptStep::Propose(write("claude", 1, ".claude/a")?));
        let adapter = FakeAdapter::new().tool(&tool("claude")?, script);
        assert_eq!(drive(&adapter, &claude, &files, &FakeWorkspace::new())?, 1);
        Ok(())
    }
}
