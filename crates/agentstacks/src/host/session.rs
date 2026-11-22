//! One tool's engine, kept alive from running its entry file to the last `plan`.
//!
//! # Responsibilities
//!
//! - [`Session::open`] reads the extension's Lua files, builds its engine and runs its entry
//!   file at the top level, which registers its handlers.
//! - [`Session::values`] dispatches the `values` handler and gives its checked answer.
//! - [`Session::plan`] dispatches the `plan` handler. It lends the caller's proposals and the
//!   answers for the declared reads to the run, and takes the proposals back whatever happened.
//! - Every call returns every refusal it found: those recorded as the extension called the API,
//!   and the failure that ended the run, if it did. A refusal recorded during a call refuses the
//!   whole call even when the extension caught the error that raised it. The collection hands
//!   them back in its own output order, where a refusal with no line comes first.
//!
//! # Non-responsibilities
//!
//! When a session is opened and in what order its calls are made belongs to the flows. The
//! session never writes.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use airsl::{Engine, Script};

use crate::host::engine::build_engine;
use crate::host::events::{Dispatched, Event, dispatch};
use crate::host::failure::{lua_failure, wrap};
use crate::host::files::LuaFiles;
use crate::host::module::AgentstacksModule;
use crate::host::payload::{plan_payload, values_payload};
use crate::host::read::ReadSnapshot;
use crate::host::state::{Loan, Phase, RunState, SharedState};
use crate::host::values::values_result;
use crate::port::{FileTree, Workspace};
use crate::types::extension::Extension;
use crate::types::name::ToolName;
use crate::types::op::{OpError, OpPath, PlanInput, Proposals, ValuesInput, ValuesResult};
use crate::types::plan::Reads;
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::tree::TreePath;

/// One tool's extension, loaded and run: its engine with the handlers its entry file registered.
pub struct Session {
    engine: Engine,
    state: SharedState,
    tool: ToolName,
    entry: TreePath,
    reads: Vec<OpPath>,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("tool", &self.tool)
            .field("entry", &self.entry)
            .field("reads", &self.reads)
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Loads the `.lua` files of `extension` from `files`, the tree it was loaded from, and runs
    /// its entry file.
    ///
    /// # Errors
    ///
    /// The refusals of reading the files, of the entry run, and of anything the entry file did
    /// that is refused.
    pub(super) fn open<T: FileTree>(extension: &Extension, files: &T) -> Result<Self, Diagnostics> {
        let tool = extension.manifest().name().clone();
        let lua_files = LuaFiles::read(files, extension)?;
        let (entry, text) = lua_files.entry();
        let entry = entry.clone();
        let text = text.to_owned();
        let state = SharedState::new(tool.clone());
        let failed = |error: &airsl::Error| -> Diagnostics {
            let mut found = Diagnostics::new();
            found.extend(wrap(&tool, &entry, error));
            found
        };
        let module = AgentstacksModule::new(state.clone(), Arc::new(lua_files))
            .map_err(|error| failed(&error))?;
        let engine = build_engine(module).map_err(|error| failed(&error))?;
        let script = Script::from_source(&text, entry.as_str()).map_err(|error| failed(&error))?;
        let outcome = engine.eval(&script);
        let reads = extension
            .reads()
            .iter()
            .filter_map(|read| OpPath::new(&read.normalized()).ok())
            .collect();
        let session = Self {
            engine,
            state,
            tool,
            entry,
            reads,
        };
        session
            .refusals(outcome.err().as_ref())
            .into_result(session)
    }

    /// Dispatches the `values` handler with `input` and checks its answer.
    ///
    /// # Errors
    ///
    /// The refusals found in the handler or in its answer.
    pub(super) fn values(&self, input: &ValuesInput) -> Result<ValuesResult, Diagnostics> {
        self.state.with(|run| run.set_phase(Phase::Values));
        let outcome = dispatch(&self.engine, Event::Values, &values_payload(input));
        self.state.with(|run| run.set_phase(Phase::Load));
        let found = self.refusals(outcome.as_ref().err());
        match outcome {
            Ok(dispatched) if found.is_empty() => {
                values_result(Path::new(&self.place()), dispatched, input)
            }
            _ => Err(found),
        }
    }

    /// Dispatches the `plan` handler with `input`, adding what it proposes to `proposals`.
    ///
    /// # Errors
    ///
    /// Every refusal found while the handler ran.
    pub(super) fn plan<W: Workspace>(
        &self,
        input: &PlanInput,
        workspace: &W,
        proposals: &mut Proposals,
    ) -> Result<Reads, Diagnostics> {
        let loan = Loan {
            input: input.clone(),
            proposals: std::mem::take(proposals),
            snapshot: ReadSnapshot::take(workspace, &self.reads),
            reads: Reads::default(),
        };
        self.state.with(|run| run.begin_plan(loan));
        let outcome = dispatch(&self.engine, Event::Plan, &plan_payload(input));
        let reads = self
            .state
            .with(RunState::end_plan)
            .map_or_else(Reads::default, |loan| {
                *proposals = loan.proposals;
                loan.reads
            });
        let mut found = self.refusals(outcome.as_ref().err());
        if matches!(outcome, Ok(Dispatched::NoHandler)) {
            found.push(Diagnostic::new(
                Location::file(self.place()),
                OpError::NoPlanHandler,
            ));
        }
        found.into_result(reads)
    }

    /// Where the entry file is reported: `<tool>/<entry file>`.
    fn place(&self) -> String {
        format!("{}/{}", self.tool, self.entry)
    }

    /// The refusals recorded since the last call, and the one `failure` is, if the run ended in
    /// one. A failure whose refusal was recorded or handed out already adds nothing, unless
    /// nothing else was found: a failed call is always refused.
    fn refusals(&self, failure: Option<&airsl::Error>) -> Diagnostics {
        let mut found = self.state.with(RunState::take_recorded);
        if let Some(error) = failure {
            match wrap(&self.tool, &self.entry, error) {
                Some(refusal) => found.push(refusal),
                None if found.is_empty() => {
                    found.push(lua_failure(&self.tool, &self.entry, error));
                }
                None => {}
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;

    use super::*;
    use crate::host::testing::{extension, plan_input, values_input};
    use crate::port::FakeWorkspace;
    use crate::types::name::{ItemName, PackName};
    use crate::types::op::{ItemId, Operation, Origin, Proposal};
    use crate::types::plan::ReadOutcome;
    use crate::types::tokens::Var;

    type TestResult = Result<(), Box<dyn Error>>;

    /// A `values` handler of a tool extension.
    const VALUES: &str = "agentstacks.on('values', function(q)
  local out = { root = '.claude/agentstacks', skills = {}, agents = {}, vars = { tmpdir = '$TMPDIR' } }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. '/' .. s] = { invoke = '/' .. s, dir = '.claude/skills/' .. s }
    end
    for _, a in ipairs(p.agents) do out.agents[p.name .. '/' .. a] = a end
  end
  return out
end)
";

    /// A `plan` handler that copies a skill file and skips an agent.
    const PLAN: &str = "agentstacks.on('plan', function()
  agentstacks.copy{path='.claude/skills/status/SKILL.md', file='core/skill/status/SKILL.md'}
  agentstacks.skip{item='core/agent/coder', reason='not for claude'}
end)
";

    fn open(entry: &str) -> Result<Session, Box<dyn Error>> {
        let (loaded, tree) = extension(entry, &[], &[])?;
        Session::open(&loaded, &tree).map_err(plain)
    }

    fn plain(found: Diagnostics) -> Box<dyn Error> {
        refusals(found).join("; ").into()
    }

    fn refusals(found: Diagnostics) -> Vec<String> {
        found.into_iter().map(|d| d.to_string()).collect()
    }

    /// What each proposal is and where it was made, in order.
    fn described(proposals: &Proposals) -> Vec<String> {
        proposals
            .iter()
            .map(|proposal| {
                let what = match &proposal.op {
                    Operation::Write { path, .. } => format!("write {path}"),
                    Operation::Copy { path, file } => format!("copy {file} to {path}"),
                    Operation::Skip { item, reason } => format!("skip {item} ({reason})"),
                    Operation::RequireBinary { name } => format!("binary {name}"),
                    Operation::Delete { path } => format!("delete {path}"),
                };
                let site = proposal.origin.site().map(|site| site.to_string());
                format!("{what} @ {}", site.unwrap_or_default())
            })
            .collect()
    }

    fn earlier_write(path: &str) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            op: Operation::Write {
                path: OpPath::new(path)?,
                content: Vec::new(),
                covers: Vec::new(),
            },
            origin: Origin::Extension {
                tool: ToolName::new("opencode")?,
                file: TreePath::from_parts(&["adapter.lua"]),
                line: NonZeroUsize::new(3).ok_or("a line starts at 1")?,
            },
        })
    }

    #[test]
    fn full_run_values_then_plan() -> TestResult {
        let session = open(&format!("{VALUES}{PLAN}"))?;
        let values = session.values(&values_input()?).map_err(plain)?;
        let core = PackName::new("core")?;
        assert_eq!(values.root().as_str(), ".claude/agentstacks");
        assert_eq!(
            values
                .skill(&core, &ItemName::new("status")?)
                .map(|skill| skill.invoke.as_str()),
            Some("/status")
        );
        assert_eq!(values.var(Var::Tmpdir), Some("$TMPDIR"));

        let mut proposals = Proposals::new();
        let reads = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut proposals)
            .map_err(plain)?;
        assert_eq!(reads, Reads::default());
        assert_eq!(
            described(&proposals),
            [
                "copy core/skill/status/SKILL.md to .claude/skills/status/SKILL.md @ adapter.lua:12",
                "skip core/agent/coder (not for claude) @ adapter.lua:13",
            ]
        );
        Ok(())
    }

    #[test]
    fn proposals_come_back_after_a_failed_dispatch() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function()\n  agentstacks.write{path='.claude/a.md', content='a'}\n  error('boom')\nend)",
        )?;
        let mut proposals = Proposals::new();
        proposals.propose(earlier_write(".claude/other.md")?)?;
        let refused = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut proposals)
            .err()
            .ok_or("the plan should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua: lua error in plan: runtime error: adapter.lua:3: boom"]
        );
        assert_eq!(
            described(&proposals),
            [
                "write .claude/other.md @ adapter.lua:3",
                "write .claude/a.md @ adapter.lua:2",
            ]
        );
        Ok(())
    }

    #[test]
    fn missing_plan_handler_is_refused() -> TestResult {
        let session = open(VALUES)?;
        let refused = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut Proposals::new())
            .err()
            .ok_or("the plan should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua: registers no plan handler"]
        );
        Ok(())
    }

    #[test]
    fn plan_return_value_is_ignored() -> TestResult {
        let body = "  agentstacks.skip{item='core/agent/coder', reason='r'}\n";
        let returning = open(&format!(
            "agentstacks.on('plan', function()\n{body}  return {{ x = 1 }}\nend)"
        ))?;
        let silent = open(&format!("agentstacks.on('plan', function()\n{body}end)"))?;
        let (mut first, mut second) = (Proposals::new(), Proposals::new());
        returning
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut first)
            .map_err(plain)?;
        silent
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut second)
            .map_err(plain)?;
        assert_eq!(described(&first), described(&second));
        assert_eq!(described(&first).len(), 1);
        Ok(())
    }

    #[test]
    fn removed_tool_runs_with_zero_packs() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function(q)\n  assert(#q.packs == 0)\n  agentstacks.write{path='.claude/settings.json', content='{}'}\nend)",
        )?;
        let mut input = plan_input()?;
        input.packs.clear();
        let mut proposals = Proposals::new();
        session
            .plan(&input, &FakeWorkspace::new(), &mut proposals)
            .map_err(plain)?;
        assert_eq!(
            described(&proposals),
            ["write .claude/settings.json @ adapter.lua:3"]
        );
        Ok(())
    }

    #[test]
    fn entry_failure_refuses_open() -> TestResult {
        let (loaded, tree) = extension("local function f(", &[], &[])?;
        let refused = Session::open(&loaded, &tree)
            .err()
            .ok_or("opening should be refused")?;
        assert_eq!(
            refusals(refused),
            [
                "claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>"
            ]
        );
        Ok(())
    }

    #[test]
    fn a_lua_file_that_is_not_utf8_refuses_open() -> TestResult {
        let (loaded, tree) = extension("", &[], &[("lib/bad.lua", &[0xff])])?;
        let refused = Session::open(&loaded, &tree)
            .err()
            .ok_or("opening should be refused")?;
        assert_eq!(refusals(refused), ["claude/lib/bad.lua: not valid UTF-8"]);
        Ok(())
    }

    #[test]
    fn an_op_at_top_level_refuses_open() -> TestResult {
        let (loaded, tree) = extension("agentstacks.write{path='a', content='x'}", &[], &[])?;
        let refused = Session::open(&loaded, &tree)
            .err()
            .ok_or("opening should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua:1: write: only allowed inside the plan handler"]
        );
        Ok(())
    }

    #[test]
    fn fresh_state_per_session() -> TestResult {
        let handler = "agentstacks.on('plan', function()\n  agentstacks.skip{item='core/agent/coder', reason=type(leaked)}\nend)";
        let first = open(&format!("leaked = 'yes'\n{handler}"))?;
        let second = open(handler)?;
        let claude = ToolName::new("claude")?;
        let coder = ItemId::agent(&PackName::new("core")?, &ItemName::new("coder")?);
        let mut reasons = Vec::new();
        for session in [&first, &second] {
            let mut proposals = Proposals::new();
            session
                .plan(&plan_input()?, &FakeWorkspace::new(), &mut proposals)
                .map_err(plain)?;
            reasons.push(
                proposals
                    .skip_reason(&claude, &coder)
                    .map(ToString::to_string),
            );
        }
        assert_eq!(reasons, [Some("string".to_owned()), Some("nil".to_owned())]);
        Ok(())
    }

    #[test]
    fn uncaught_op_refusal_is_reported_once() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function()\n  agentstacks.write{path='/x', content=''}\nend)",
        )?;
        let refused = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut Proposals::new())
            .err()
            .ok_or("the plan should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua:2: write: path \"/x\" must be relative"]
        );
        Ok(())
    }

    #[test]
    fn a_caught_refusal_still_refuses_the_plan() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function()\n  pcall(agentstacks.write, {path='/x', content=''})\nend)",
        )?;
        let mut proposals = Proposals::new();
        let refused = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut proposals)
            .err()
            .ok_or("the plan should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua:2: write: path \"/x\" must be relative"]
        );
        assert_eq!(proposals.iter().count(), 0);
        Ok(())
    }

    #[test]
    fn a_refusal_and_the_failure_after_it_are_both_reported() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function()\n  pcall(agentstacks.write, {path='/x', content=''})\n  error('boom')\nend)",
        )?;
        let refused = session
            .plan(&plan_input()?, &FakeWorkspace::new(), &mut Proposals::new())
            .err()
            .ok_or("the plan should be refused")?;
        assert_eq!(
            refusals(refused),
            [
                "claude/adapter.lua: lua error in plan: runtime error: adapter.lua:3: boom",
                "claude/adapter.lua:2: write: path \"/x\" must be relative",
            ]
        );
        Ok(())
    }

    #[test]
    fn an_op_in_values_refuses_values() -> TestResult {
        let session = open(
            "agentstacks.on('values', function()\n  agentstacks.skip{item='core/agent/coder', reason='r'}\n  return { root = '.x' }\nend)",
        )?;
        let refused = session
            .values(&values_input()?)
            .err()
            .ok_or("values should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua:2: skip: only allowed inside the plan handler"]
        );
        Ok(())
    }

    #[test]
    fn a_failure_in_values_is_worded_as_the_user_reads_it() -> TestResult {
        let session = open("agentstacks.on('values', function()\n  error('no')\nend)")?;
        let refused = session
            .values(&values_input()?)
            .err()
            .ok_or("values should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua: lua error in values: runtime error: adapter.lua:2: no"]
        );
        Ok(())
    }

    #[test]
    fn a_missing_values_handler_is_refused() -> TestResult {
        let session = open(PLAN)?;
        let refused = session
            .values(&values_input()?)
            .err()
            .ok_or("values should be refused")?;
        assert_eq!(
            refusals(refused),
            ["claude/adapter.lua: registers no values handler"]
        );
        Ok(())
    }

    #[test]
    fn what_plan_read_is_returned() -> TestResult {
        let (loaded, tree) = extension(
            "agentstacks.on('plan', function()\n  agentstacks.read('.claude/settings.json')\n  agentstacks.read('opencode.json')\nend)",
            &[".claude/settings.json", "opencode.json", "unread.json"],
            &[],
        )?;
        let session = Session::open(&loaded, &tree).map_err(plain)?;
        let settings = OpPath::new(".claude/settings.json")?;
        let workspace = FakeWorkspace::new().file(&settings, b"{}");
        let reads = session
            .plan(&plan_input()?, &workspace, &mut Proposals::new())
            .map_err(plain)?;
        let declared = [
            settings.clone(),
            OpPath::new("opencode.json")?,
            OpPath::new("unread.json")?,
        ];
        assert_eq!(
            reads.shared(&settings, &declared),
            Some(ReadOutcome::Bytes(
                crate::types::tree::ContentHash::of_bytes(b"{}")
            ))
        );
        assert_eq!(
            reads.shared(&declared[1], &declared),
            Some(ReadOutcome::Absent)
        );
        assert_eq!(reads.shared(&declared[2], &declared), None);
        Ok(())
    }

    #[test]
    fn a_session_plans_again_from_a_clean_start() -> TestResult {
        let session = open(
            "agentstacks.on('plan', function()\n  agentstacks.skip{item='core/agent/coder', reason='r'}\nend)",
        )?;
        for _ in 0..2 {
            let mut proposals = Proposals::new();
            session
                .plan(&plan_input()?, &FakeWorkspace::new(), &mut proposals)
                .map_err(plain)?;
            assert_eq!(described(&proposals).len(), 1);
        }
        Ok(())
    }

    #[test]
    fn a_session_is_shown_by_its_tool() -> TestResult {
        let session = open("")?;
        assert!(format!("{session:?}").starts_with("Session { tool: ToolName(\"claude\")"));
        Ok(())
    }

    #[test]
    fn a_refusal_already_handed_out_still_refuses_the_next_plan() -> TestResult {
        let session = open(
            "local ok, saved = pcall(agentstacks.require, 'x y')\n\
             agentstacks.on('plan', function() error(saved) end)",
        )?;
        for _ in 0..2 {
            let refused = session
                .plan(&plan_input()?, &FakeWorkspace::new(), &mut Proposals::new())
                .err()
                .ok_or("the plan should be refused")?;
            assert!(!refusals(refused).is_empty());
        }
        Ok(())
    }
}
