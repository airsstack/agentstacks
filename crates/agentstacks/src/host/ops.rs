//! The four functions an extension proposes operations with.
//!
//! # Responsibilities
//!
//! - [`install`] puts `write`, `copy`, `skip` and `require_binary` into the `agentstacks` table.
//! - A call works only while the `plan` handler runs, which is checked before anything else, a
//!   call with no Lua frame included. It reads its arguments strictly, runs the
//!   checks of the operation types in the order of their table, and hands the operation to the
//!   proposals the handler works on, so a path proposed twice, an id the payload lacks and a
//!   skip that clashes with a cover are refused when the call is made.
//! - The first failing check is the refusal of the call. It is located at the nearest Lua frame
//!   and recorded, so the plan is refused even when the extension catches the error, and the
//!   refused operation is not recorded.
//!
//! # Non-responsibilities
//!
//! Every rule an operation must satisfy belongs to the operation types and the proposal record.

use airsl::mlua::{self, Lua, Table, Value};

use crate::host::args::{ArgTable, Field};
use crate::host::caller::{Caller, caller};
use crate::host::failure::Raised;
use crate::host::state::{Loan, RunState, SharedState};
use crate::types::op::{ApiFunction, OpError, OpPath, Operation, Origin, Proposal, SkipReason};
use crate::types::report::{Diagnostic, Location};

/// The fields of `write`.
const WRITE: [Field; 3] = [
    Field::text("path"),
    Field::bytes("content"),
    Field::list("covers"),
];

/// The fields of `copy`.
const COPY: [Field; 2] = [Field::text("path"), Field::text("file")];

/// The fields of `skip`.
const SKIP: [Field; 2] = [Field::text("item"), Field::text("reason")];

/// The field of `require_binary`.
const BINARY: [Field; 1] = [Field::text("name")];

/// The functions that propose an operation.
const OPERATIONS: [ApiFunction; 4] = [
    ApiFunction::Write,
    ApiFunction::Copy,
    ApiFunction::Skip,
    ApiFunction::RequireBinary,
];

/// Installs the four functions into `table`, each recording its proposals in `state`.
pub(super) fn install(lua: &Lua, table: &Table, state: &SharedState) -> mlua::Result<()> {
    for function in OPERATIONS {
        let state = state.clone();
        let installed = lua
            .create_function(move |lua, argument: Value| call(lua, &state, function, &argument))?;
        table.set(function.to_string(), installed)?;
    }
    Ok(())
}

/// Makes one call of `function` with `argument`.
///
/// The state is locked only for steps that touch no Lua value. The argument table is read before
/// the lock is taken, because reading it can run the garbage collector, and a finalizer that
/// calls back into the API would wait on the lock for itself.
fn call(
    lua: &Lua,
    state: &SharedState,
    function: ApiFunction,
    argument: &Value,
) -> mlua::Result<()> {
    let tool = state.with(|run| run.tool().clone());
    let site = caller(lua);
    let located = |site: Option<&Caller>| {
        site.map_or_else(
            || Location::file(tool.as_str()),
            |site| site.location(&tool),
        )
    };
    if let Err(error) = state.with(|run| run.plan_loan(function).map(|_| ())) {
        return Err(refuse(
            state,
            Diagnostic::new(located(site.as_ref()), error),
        ));
    }
    let Some(site) = site else {
        let error = OpError::Lua {
            first_line: format!("{function} must be called from the extension's Lua code"),
        };
        return Err(refuse(state, Diagnostic::new(located(None), error)));
    };
    let origin = site.origin(&tool);
    read_arguments(function, argument)
        .and_then(|args| state.with(|run| propose(run, function, &args, origin)))
        .map_err(|error| refuse(state, Diagnostic::new(site.location(&tool), error)))
}

/// Records `refusal`, which holds even if the extension catches the error, and gives the error
/// the call raises.
fn refuse(state: &SharedState, refusal: Diagnostic) -> mlua::Error {
    let text = refusal.to_string();
    state.with(|run| run.record(refusal));
    Raised::recorded(text).into_lua()
}

/// The fields of `argument`, read as the table `function` takes.
fn read_arguments(function: ApiFunction, argument: &Value) -> Result<ArgTable, OpError> {
    let fields: &[Field] = match function {
        ApiFunction::Write => &WRITE,
        ApiFunction::Copy => &COPY,
        ApiFunction::Skip => &SKIP,
        ApiFunction::RequireBinary | ApiFunction::Read | ApiFunction::Require => &BINARY,
    };
    ArgTable::read(function, argument, fields)
}

/// Checks the call, whose arguments are `args`, and adds its operation to the proposals of the
/// run. No Lua value is touched.
fn propose(
    run: &mut RunState,
    function: ApiFunction,
    args: &ArgTable,
    origin: Origin,
) -> Result<(), OpError> {
    let loan = run.plan_loan(function)?;
    let op = match function {
        ApiFunction::Write => write(loan, args, &origin)?,
        ApiFunction::Copy => copy(loan, args, &origin)?,
        ApiFunction::Skip => skip(loan, args)?,
        ApiFunction::RequireBinary | ApiFunction::Read | ApiFunction::Require => {
            Operation::require_binary(args.string("name"))?
        }
    };
    loan.proposals.propose(Proposal { op, origin })
}

/// The `write` the call asks for.
fn write(loan: &Loan, args: &ArgTable, origin: &Origin) -> Result<Operation, OpError> {
    let function = ApiFunction::Write;
    let path = OpPath::proposed(args.string("path"), function)?;
    loan.proposals.check_path(&path, function, origin)?;
    let covers = args
        .string_list("covers")
        .iter()
        .enumerate()
        .map(|(index, id)| loan.input.cover(index + 1, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Operation::Write {
        path,
        content: args.bytes("content").to_vec(),
        covers,
    })
}

/// The `copy` the call asks for.
fn copy(loan: &Loan, args: &ArgTable, origin: &Origin) -> Result<Operation, OpError> {
    let function = ApiFunction::Copy;
    let path = OpPath::proposed(args.string("path"), function)?;
    loan.proposals.check_path(&path, function, origin)?;
    let file = loan.input.copy_file(args.string("file"))?;
    Ok(Operation::Copy { path, file })
}

/// The `skip` the call asks for.
fn skip(loan: &Loan, args: &ArgTable) -> Result<Operation, OpError> {
    let item = loan.input.skip_item(args.string("item"))?;
    let reason = SkipReason::new(args.string("reason"))?;
    Ok(Operation::Skip { item, reason })
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::Duration;

    use crate::host::events::Event;
    use crate::host::testing::{Planned, Rig, finishes_within, plan_input};
    use crate::port::FakeWorkspace;
    use crate::types::name::ToolName;
    use crate::types::op::{OpPath, Operation, Origin, Proposal, Proposals};
    use crate::types::tree::TreePath;
    use serde_json::json;

    type TestResult = Result<(), Box<dyn Error>>;

    /// A `plan` handler that makes each of `calls`, a line number and the code on that line,
    /// counting from the first line of the entry file, which holds the `agentstacks.on` call.
    fn handler(calls: &[(usize, &str)]) -> String {
        let mut source = String::from("agentstacks.on('plan', function()");
        let mut line = 1;
        for (number, code) in calls {
            source.push_str(&"\n".repeat(number - line));
            source.push_str(code);
            line = *number;
        }
        source.push_str("\nend)");
        source
    }

    /// Runs `calls` in the `plan` handler on top of `proposals`.
    fn run_on(
        calls: &[(usize, &str)],
        proposals: Proposals,
    ) -> Result<(Rig, Planned), Box<dyn Error>> {
        let rig = Rig::new()?;
        rig.run(&handler(calls))?;
        let planned = rig.plan(&plan_input()?, proposals, &FakeWorkspace::new());
        Ok((rig, planned))
    }

    fn run(calls: &[(usize, &str)]) -> Result<(Rig, Planned), Box<dyn Error>> {
        run_on(calls, Proposals::new())
    }

    /// The refusals a `plan` handler of `calls` is refused with.
    fn refused(calls: &[(usize, &str)]) -> Result<Vec<String>, Box<dyn Error>> {
        let (rig, _) = run(calls)?;
        Ok(rig.refusals())
    }

    fn call_at(line: usize) -> Result<Origin, Box<dyn Error>> {
        Ok(Origin::Extension {
            tool: ToolName::new("claude")?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: std::num::NonZeroUsize::new(line).ok_or("a line starts at 1")?,
        })
    }

    fn recorded(planned: &Planned) -> Vec<(&Operation, &Origin)> {
        planned
            .proposals
            .iter()
            .map(|p| (&p.op, &p.origin))
            .collect()
    }

    #[test]
    fn write_inside_plan_is_recorded_with_its_origin() -> TestResult {
        let (rig, planned) = run(&[(2, "agentstacks.write{path='.claude/x.md', content='a'}")])?;
        assert!(rig.refusals().is_empty());
        let expected = Operation::Write {
            path: OpPath::new(".claude/x.md")?,
            content: b"a".to_vec(),
            covers: Vec::new(),
        };
        assert_eq!(recorded(&planned), [(&expected, &call_at(2)?)]);
        Ok(())
    }

    #[test]
    fn copy_skip_and_require_binary_are_recorded() -> TestResult {
        let (rig, planned) = run(&[
            (
                2,
                "agentstacks.copy{path='a.md', file='core/skill/status/SKILL.md'}",
            ),
            (
                3,
                "agentstacks.skip{item='core/agent/coder', reason='not for this tool'}",
            ),
            (4, "agentstacks.require_binary{name='git'}"),
        ])?;
        assert!(rig.refusals().is_empty());
        let kinds: Vec<&str> = planned
            .proposals
            .iter()
            .map(|p| match p.op {
                Operation::Copy { .. } => "copy",
                Operation::Skip { .. } => "skip",
                Operation::RequireBinary { .. } => "binary",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["copy", "skip", "binary"]);
        let lines: Vec<Option<String>> = planned
            .proposals
            .iter()
            .map(|p| p.origin.site().map(|site| site.to_string()))
            .collect();
        assert_eq!(
            lines,
            [
                Some("adapter.lua:2".to_owned()),
                Some("adapter.lua:3".to_owned()),
                Some("adapter.lua:4".to_owned())
            ]
        );
        Ok(())
    }

    #[test]
    fn ops_at_top_level_are_refused() -> TestResult {
        let rig = Rig::new()?;
        let refused = rig.run("agentstacks.write{path='a', content='x'}").err();
        assert_eq!(refused.as_deref(), Some("(recorded)"));
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:1: write: only allowed inside the plan handler"]
        );
        Ok(())
    }

    #[test]
    fn ops_in_values_are_refused() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('values', function()\n\n  agentstacks.skip{item='core/agent/coder', reason='r'}\nend)",
        )?;
        let _ = rig.dispatch(Event::Values, &json!({}));
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:3: skip: only allowed inside the plan handler"]
        );
        Ok(())
    }

    #[test]
    fn argument_refusal_carries_location_and_function() -> TestResult {
        assert_eq!(
            refused(&[(12, "agentstacks.write{path=5, content='x'}")])?,
            ["claude/adapter.lua:12: write: path: expected string, got number"]
        );
        Ok(())
    }

    #[test]
    fn path_rule_refusal_carries_location() -> TestResult {
        assert_eq!(
            refused(&[(12, "agentstacks.write{path='/etc/x', content='x'}")])?,
            ["claude/adapter.lua:12: write: path \"/etc/x\" must be relative"]
        );
        Ok(())
    }

    #[test]
    fn a_path_agentstacks_writes_itself_is_refused() -> TestResult {
        assert_eq!(
            refused(&[(
                2,
                "agentstacks.write{path='./agentstacks.lock', content=''}"
            )])?,
            ["claude/adapter.lua:2: write: path \"agentstacks.lock\" is managed by agentstacks"]
        );
        Ok(())
    }

    #[test]
    fn id_refusal_carries_location() -> TestResult {
        assert_eq!(
            refused(&[(
                2,
                "agentstacks.copy{path='a', file='core/skill/x/SKILL.md'}"
            )])?,
            ["claude/adapter.lua:2: copy: file \"core/skill/x/SKILL.md\" is not in this plan"]
        );
        assert_eq!(
            refused(&[(2, "agentstacks.skip{item='core/agent/nope', reason='r'}")])?,
            ["claude/adapter.lua:2: skip: item \"core/agent/nope\" is not in this plan"]
        );
        Ok(())
    }

    #[test]
    fn copy_of_a_non_file_is_refused() -> TestResult {
        assert_eq!(
            refused(&[(2, "agentstacks.copy{path='a', file='core/agent/coder'}")])?,
            ["claude/adapter.lua:2: copy: file \"core/agent/coder\" is not a file"]
        );
        Ok(())
    }

    #[test]
    fn covers_naming_a_skill_group_is_refused() -> TestResult {
        assert_eq!(
            refused(&[(
                7,
                "agentstacks.write{path='a', content='', covers={'core/skill/status'}}"
            )])?,
            [
                "claude/adapter.lua:7: write: covers[1]: \"core/skill/status\" is a skill; list its files"
            ]
        );
        Ok(())
    }

    #[test]
    fn cover_after_skip_names_the_skip() -> TestResult {
        assert_eq!(
            refused(&[
                (9, "agentstacks.skip{item='core/agent/coder', reason='r'}"),
                (
                    14,
                    "agentstacks.write{path='a', content='', covers={'core/agent/coder'}}"
                ),
            ])?,
            [
                "claude/adapter.lua:14: write: covers \"core/agent/coder\", which adapter.lua:9 skipped"
            ]
        );
        Ok(())
    }

    #[test]
    fn duplicate_path_names_the_first_call() -> TestResult {
        let (rig, planned) = run(&[
            (9, "agentstacks.write{path='.claude/x', content='1'}"),
            (14, "agentstacks.write{path='.claude/x', content='2'}"),
        ])?;
        assert_eq!(
            rig.refusals(),
            [
                "claude/adapter.lua:14: write: path \".claude/x\" is already proposed at adapter.lua:9"
            ]
        );
        assert_eq!(planned.proposals.iter().count(), 1);
        Ok(())
    }

    #[test]
    fn duplicate_path_precedes_id_rows() -> TestResult {
        assert_eq!(
            refused(&[
                (9, "agentstacks.write{path='.claude/x', content='1'}"),
                (
                    14,
                    "agentstacks.write{path='.claude/x', content='2', covers={'core/agent/nope'}}"
                ),
            ])?,
            [
                "claude/adapter.lua:14: write: path \".claude/x\" is already proposed at adapter.lua:9"
            ]
        );
        Ok(())
    }

    #[test]
    fn duplicate_against_an_earlier_tool_names_it() -> TestResult {
        let mut earlier = Proposals::new();
        earlier.propose(Proposal {
            op: Operation::Write {
                path: OpPath::new(".claude/x")?,
                content: Vec::new(),
                covers: Vec::new(),
            },
            origin: Origin::Extension {
                tool: ToolName::new("opencode")?,
                file: TreePath::from_parts(&["adapter.lua"]),
                line: std::num::NonZeroUsize::new(3).ok_or("a line starts at 1")?,
            },
        })?;
        let (rig, planned) = run_on(
            &[(5, "agentstacks.write{path='.claude/x', content=''}")],
            earlier,
        )?;
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:5: write: path \".claude/x\" is already proposed by opencode"]
        );
        assert_eq!(planned.proposals.iter().count(), 1);
        Ok(())
    }

    #[test]
    fn skip_after_copy_conflict_names_the_first_call() -> TestResult {
        assert_eq!(
            refused(&[
                (
                    9,
                    "agentstacks.copy{path='a', file='core/skill/status/SKILL.md'}"
                ),
                (14, "agentstacks.skip{item='core/skill/status', reason='r'}"),
            ])?,
            [
                "claude/adapter.lua:14: skip: item \"core/skill/status\" is already covered at adapter.lua:9"
            ]
        );
        Ok(())
    }

    #[test]
    fn refusal_sticks_under_pcall() -> TestResult {
        let (rig, planned) = run(&[(
            2,
            "pcall(function() agentstacks.write{path='/x', content=''} end)",
        )])?;
        assert!(planned.result.is_ok());
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:2: write: path \"/x\" must be relative"]
        );
        assert_eq!(planned.proposals.iter().count(), 0);
        Ok(())
    }

    #[test]
    fn pcall_of_the_function_is_located_at_the_pcall_line() -> TestResult {
        assert_eq!(
            refused(&[(6, "pcall(agentstacks.write, {path='/x', content=''})")])?,
            ["claude/adapter.lua:6: write: path \"/x\" must be relative"]
        );
        Ok(())
    }

    #[test]
    fn uncaught_op_refusal_is_reported_once() -> TestResult {
        let (rig, planned) = run(&[(2, "agentstacks.write{path='/x', content=''}")])?;
        assert_eq!(planned.result.err().as_deref(), Some("(recorded)"));
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:2: write: path \"/x\" must be relative"]
        );
        Ok(())
    }

    #[test]
    fn one_refusal_per_call() -> TestResult {
        assert_eq!(
            refused(&[(2, "agentstacks.write{path=5}")])?,
            ["claude/adapter.lua:2: write: content: required"]
        );
        Ok(())
    }

    #[test]
    fn a_blank_reason_is_refused() -> TestResult {
        assert_eq!(
            refused(&[(2, "agentstacks.skip{item='core/agent/coder', reason='  '}")])?,
            ["claude/adapter.lua:2: skip: reason: empty"]
        );
        Ok(())
    }

    #[test]
    fn a_binary_name_is_checked() -> TestResult {
        assert_eq!(
            refused(&[(2, "agentstacks.require_binary{name='Git'}")])?,
            ["claude/adapter.lua:2: require_binary: name: \"Git\" is not a valid name"]
        );
        Ok(())
    }

    #[test]
    fn a_path_is_recorded_normalised() -> TestResult {
        let (_, planned) = run(&[(2, "agentstacks.write{path='./.claude//x.md', content=''}")])?;
        let paths: Vec<Option<&str>> = planned
            .proposals
            .iter()
            .map(|p| p.op.path().map(OpPath::as_str))
            .collect();
        assert_eq!(paths, [Some(".claude/x.md")]);
        Ok(())
    }

    #[test]
    fn a_call_with_no_lua_frame_is_refused_by_name() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('plan', agentstacks.write)")?;
        let planned = rig.plan(&plan_input()?, Proposals::new(), &FakeWorkspace::new());
        assert_eq!(planned.result.err().as_deref(), Some("(recorded)"));
        assert_eq!(
            rig.refusals(),
            ["claude: write must be called from the extension's Lua code"]
        );
        assert_eq!(planned.proposals.iter().count(), 0);
        Ok(())
    }

    #[test]
    fn op_inside_required_module_names_the_module() -> TestResult {
        let settings: &[u8] =
            b"local M = {}\nfunction M.go()\n  agentstacks.write{path='/x', content=''}\nend\nreturn M";
        let rig = Rig::with(&[], &[("lib/settings.lua", settings)])?;
        rig.run(
            "local settings = agentstacks.require('lib.settings')\nagentstacks.on('plan', function()\n  settings.go()\nend)",
        )?;
        let planned = rig.plan(&plan_input()?, Proposals::new(), &FakeWorkspace::new());
        assert_eq!(planned.result.err().as_deref(), Some("(recorded)"));
        assert_eq!(
            rig.refusals(),
            ["claude/lib/settings.lua:3: write: path \"/x\" must be relative"]
        );
        Ok(())
    }

    #[test]
    fn a_finalizer_that_proposes_during_an_op_loop_completes() {
        let finished = finishes_within(Duration::from_secs(20), || -> Result<String, String> {
            let calls = [(
                2,
                "collectgarbage('incremental', 1, 1000)\n\
                 local mt = { __gc = function() pcall(agentstacks.require_binary, {name='git'}) end }\n\
                 for i = 1, 5000 do setmetatable({}, mt) pcall(agentstacks.require_binary, {name='git', ['k'..i]=string.rep('y', 200)}) end",
            )];
            let (_, planned) = run(&calls).map_err(|error| error.to_string())?;
            planned.result.map(|_| "done".to_owned())
        });
        assert_eq!(finished, Some(Ok("done".to_owned())));
    }

    #[test]
    fn the_phase_check_comes_before_the_no_lua_frame_check() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('values', agentstacks.write)")?;
        let _ = rig.dispatch(Event::Values, &json!({}));
        assert_eq!(
            rig.refusals(),
            ["claude: write: only allowed inside the plan handler"]
        );
        Ok(())
    }
}
