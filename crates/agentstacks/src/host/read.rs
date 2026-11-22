//! What `agentstacks.read` can answer, taken from the project before the `plan` handler runs.
//!
//! # Responsibilities
//!
//! - [`ReadSnapshot::take`] asks the workspace once about every path the extension declared in
//!   `reads`, and keeps each answer or the failure of asking.
//! - [`function`] is `agentstacks.read`. It works only while the `plan` handler runs and takes one
//!   string, a path the extension declared in `reads`. It refuses a path that leads outside the
//!   project and a path whose answer was a failure, and a refusal is recorded, so the plan is
//!   refused even when the extension catches the error. Otherwise it returns the file's bytes as
//!   they are, or `nil` where no regular file is, and records what it returned, the hash or
//!   absence, for the plan. A path is recorded only when it was read.
//!
//! # Non-responsibilities
//!
//! An installed Lua function is `'static` and cannot borrow the workspace, which is why the
//! answers are taken up front.

use std::collections::BTreeMap;
use std::io;

use airsl::mlua::{self, Function, Lua, Value};

use crate::host::caller::locate;
use crate::host::convert::lua_type;
use crate::host::failure::Raised;
use crate::host::state::{RunState, SharedState};
use crate::port::Workspace;
use crate::types::op::{ApiFunction, LuaType, OpError, OpPath};
use crate::types::plan::{DiskState, ReadOutcome, Resolution};
use crate::types::report::{Diagnostic, Location, Quoted};

/// What the workspace said about one declared path.
#[derive(Debug)]
pub(super) enum Answer {
    /// Where the path leads and what is there.
    Resolved(Resolution),
    /// Asking failed; the error is kept as its kind and its text.
    Failed(io::ErrorKind, String),
}

/// The answer for every path an extension declared in `reads`.
#[derive(Debug, Default)]
pub(super) struct ReadSnapshot {
    answers: BTreeMap<OpPath, Answer>,
}

impl ReadSnapshot {
    /// Asks `workspace` about each of `reads`.
    pub(super) fn take<W: Workspace>(workspace: &W, reads: &[OpPath]) -> Self {
        let answers = reads
            .iter()
            .map(|path| {
                let answer = match workspace.resolve(path) {
                    Ok(resolution) => Answer::Resolved(resolution),
                    Err(error) => Answer::Failed(error.kind(), error.to_string()),
                };
                (path.clone(), answer)
            })
            .collect();
        Self { answers }
    }

    /// The answer for `path`, or `None` when the extension did not declare it.
    pub(super) fn get(&self, path: &OpPath) -> Option<&Answer> {
        self.answers.get(path)
    }
}

/// The Lua function `agentstacks.read`.
///
/// The state is locked only to decide the answer and to record it, never while a Lua value is
/// built: building one can run the garbage collector, and a finalizer that calls back into the
/// API would wait on the lock for itself.
pub(super) fn function(lua: &Lua, state: &SharedState) -> mlua::Result<Function> {
    let state = state.clone();
    lua.create_function(move |lua, argument: Value| {
        let tool = state.with(|run| run.tool().clone());
        let location = locate(lua, &tool);
        let written = written(&argument);
        match state.with(|run| decide(run, written, &location)) {
            Ok(Decision::Absent) => Ok(Value::Nil),
            Ok(Decision::Bytes(bytes)) => {
                let string = lua.create_string(&bytes)?;
                Ok(Value::String(string))
            }
            Err(refusal) => {
                let text = refusal.to_string();
                state.with(|run| run.record(refusal));
                Err(Raised::recorded(text).into_lua())
            }
        }
    })
}

/// What a call of `read` returns.
#[derive(Debug)]
enum Decision {
    /// `nil`: no regular file is there.
    Absent,
    /// The file's bytes, which the caller turns into a Lua string.
    Bytes(Vec<u8>),
}

/// The argument of a call of `read`, taken out of Lua before the state is locked.
enum Written {
    /// The path, as written.
    Path(String),
    /// Not a string; the type it has.
    Other(LuaType),
}

/// `argument` as the path it names, or the type that makes it not one.
fn written(argument: &Value) -> Written {
    match argument {
        Value::String(text) => Written::Path(text.to_string_lossy()),
        other => Written::Other(lua_type(other)),
    }
}

/// What a call of `read` with `written` returns, or the refusal it is met with, and records what
/// it returned. `location` is where a refusal of the call is reported. No Lua value is touched.
fn decide(
    run: &mut RunState,
    written: Written,
    location: &Location,
) -> Result<Decision, Diagnostic> {
    let at = |error: OpError| Diagnostic::new(location.clone(), error);
    let function = ApiFunction::Read;
    let loan = run.plan_loan(function).map_err(at)?;
    let written = match written {
        Written::Path(written) => written,
        Written::Other(got) => {
            return Err(at(OpError::ArgumentType {
                function,
                expected: LuaType::String,
                got,
            }));
        }
    };
    let declared = OpPath::new(&written)
        .ok()
        .and_then(|path| loan.snapshot.get(&path).map(|answer| (path, answer)));
    let Some((path, answer)) = declared else {
        return Err(at(OpError::NotInReads {
            path: Quoted::new(written),
        }));
    };
    match answer {
        Answer::Failed(kind, text) => Err(Diagnostic::new(
            Location::file(path.as_str()),
            io::Error::new(*kind, text.clone()),
        )),
        Answer::Resolved(Resolution::Outside { .. }) => Err(at(OpError::ReadOutside {
            path: Quoted::new(written),
        })),
        Answer::Resolved(Resolution::Inside {
            state: DiskState::File(file),
            ..
        }) => {
            let bytes = file.bytes().to_vec();
            loan.reads
                .record(path, ReadOutcome::Bytes(file.hash().clone()));
            Ok(Decision::Bytes(bytes))
        }
        Answer::Resolved(Resolution::Inside { .. }) => {
            loan.reads.record(path, ReadOutcome::Absent);
            Ok(Decision::Absent)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::host::events::Dispatched;
    use crate::host::testing::{Planned, Rig, finishes_within, plan_input};
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::op::Proposals;
    use crate::types::plan::{DiskState, Link, ReadOutcome, Reads};
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn Error>>;

    fn path(text: &str) -> Result<OpPath, Box<dyn Error>> {
        Ok(OpPath::new(text)?)
    }

    #[test]
    fn every_declared_path_is_asked_once() -> TestResult {
        let settings = path(".claude/settings.json")?;
        let other = path("opencode.json")?;
        let workspace = FakeWorkspace::new().file(&settings, b"{}");
        let snapshot = ReadSnapshot::take(&workspace, &[settings.clone(), other.clone()]);
        assert!(matches!(
            snapshot.get(&settings),
            Some(Answer::Resolved(Resolution::Inside {
                link: Link::Direct,
                state: DiskState::File(_)
            }))
        ));
        assert!(matches!(
            snapshot.get(&other),
            Some(Answer::Resolved(Resolution::Inside {
                state: DiskState::Absent,
                ..
            }))
        ));
        assert_eq!(
            workspace.calls(),
            [
                WorkspaceCall::Resolve(settings),
                WorkspaceCall::Resolve(other)
            ]
        );
        Ok(())
    }

    #[test]
    fn an_undeclared_path_has_no_answer() -> TestResult {
        let snapshot = ReadSnapshot::take(&FakeWorkspace::new(), &[path("a")?]);
        assert!(snapshot.get(&path("b")?).is_none());
        Ok(())
    }

    #[test]
    fn a_failure_is_kept_with_its_text() -> TestResult {
        let settings = path(".claude/settings.json")?;
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::Resolve(settings.clone()));
        let snapshot = ReadSnapshot::take(&workspace, std::slice::from_ref(&settings));
        let denied = FakeWorkspace::denied();
        match snapshot.get(&settings) {
            Some(Answer::Failed(kind, text)) => {
                assert_eq!(*kind, denied.kind());
                assert_eq!(text, &denied.to_string());
            }
            other => return Err(format!("expected a failure, got {other:?}").into()),
        }
        Ok(())
    }

    /// Runs `source` as the entry file of a tool that declares `reads`, then its `plan` handler
    /// over `workspace`.
    fn run_plan(
        reads: &[&str],
        workspace: &FakeWorkspace,
        source: &str,
    ) -> Result<(Rig, Planned), Box<dyn Error>> {
        let rig = Rig::with(reads, &[])?;
        rig.run(source)?;
        let planned = rig.plan(&plan_input()?, Proposals::new(), workspace);
        Ok((rig, planned))
    }

    /// A `plan` handler of `body`, which starts on line 2.
    fn handler(body: &str) -> String {
        format!("agentstacks.on('plan', function()\n{body}\nend)")
    }

    const SETTINGS: &str = ".claude/settings.json";

    #[test]
    fn read_returns_raw_bytes() -> TestResult {
        let workspace = FakeWorkspace::new().file(&path(SETTINGS)?, &[0, 255, 65]);
        let (_, planned) = run_plan(
            &[SETTINGS],
            &workspace,
            &handler(&format!(
                "  local s = agentstacks.read('{SETTINGS}')\n  return {{ n = #s, b = {{ s:byte(1, -1) }} }}"
            )),
        )?;
        assert_eq!(
            planned.result?,
            Dispatched::Value(json!({"n": 3, "b": [0, 255, 65]}))
        );
        Ok(())
    }

    #[test]
    fn read_returns_nil_for_an_absent_file() -> TestResult {
        let (_, planned) = run_plan(
            &[SETTINGS],
            &FakeWorkspace::new(),
            &handler(&format!(
                "  return {{ gone = agentstacks.read('{SETTINGS}') == nil }}"
            )),
        )?;
        assert_eq!(planned.result?, Dispatched::Value(json!({"gone": true})));
        let declared = [path(SETTINGS)?];
        assert_eq!(
            planned.reads.shared(&declared[0], &declared),
            Some(ReadOutcome::Absent)
        );
        Ok(())
    }

    #[test]
    fn read_returns_nil_for_a_non_regular_file() -> TestResult {
        let workspace = FakeWorkspace::new().not_regular(&path(SETTINGS)?);
        let (_, planned) = run_plan(
            &[SETTINGS],
            &workspace,
            &handler(&format!(
                "  return {{ gone = agentstacks.read('{SETTINGS}') == nil }}"
            )),
        )?;
        assert_eq!(planned.result?, Dispatched::Value(json!({"gone": true})));
        Ok(())
    }

    #[test]
    fn read_records_the_hash_of_what_it_returned() -> TestResult {
        let settings = path(SETTINGS)?;
        let other = path("opencode.json")?;
        let workspace = FakeWorkspace::new()
            .file(&settings, b"{}")
            .file(&other, b"{}");
        let (_, planned) = run_plan(
            &[SETTINGS, "opencode.json"],
            &workspace,
            &handler(&format!("  agentstacks.read('{SETTINGS}')")),
        )?;
        planned.result?;
        let declared = [settings.clone(), other.clone()];
        assert_eq!(
            planned.reads.shared(&settings, &declared),
            Some(ReadOutcome::Bytes(ContentHash::of_bytes(b"{}")))
        );
        assert_eq!(planned.reads.shared(&other, &declared), None);
        Ok(())
    }

    #[test]
    fn unread_reads_entries_are_not_recorded() -> TestResult {
        let settings = path(SETTINGS)?;
        let workspace = FakeWorkspace::new().file(&settings, b"{}");
        let (_, planned) = run_plan(&[SETTINGS], &workspace, &handler("  return {}"))?;
        planned.result?;
        assert_eq!(planned.reads, Reads::default());
        Ok(())
    }

    #[test]
    fn read_outside_reads_is_refused() -> TestResult {
        let (rig, _) = run_plan(
            &[SETTINGS],
            &FakeWorkspace::new(),
            &handler("\n\n  agentstacks.read('.env')"),
        )?;
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:4: read: \".env\" is not in extension.reads"]
        );
        Ok(())
    }

    #[test]
    fn read_resolving_outside_is_refused() -> TestResult {
        let workspace = FakeWorkspace::new().outside(&path(SETTINGS)?);
        let body = format!("{}agentstacks.read('{SETTINGS}')", "\n".repeat(7));
        let (rig, planned) = run_plan(&[SETTINGS], &workspace, &handler(&body))?;
        assert_eq!(planned.reads, Reads::default());
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:9: read: \".claude/settings.json\" resolves outside the project"]
        );
        Ok(())
    }

    #[test]
    fn read_io_failure_names_the_path() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::Resolve(path(SETTINGS)?));
        let (rig, _) = run_plan(
            &[SETTINGS],
            &workspace,
            &handler(&format!("  agentstacks.read('{SETTINGS}')")),
        )?;
        assert_eq!(
            rig.refusals(),
            [format!("{SETTINGS}: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn io_failure_on_an_unread_path_is_silent() -> TestResult {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::Resolve(path(SETTINGS)?));
        let (rig, planned) = run_plan(&[SETTINGS], &workspace, &handler("  return {}"))?;
        planned.result?;
        assert!(rig.refusals().is_empty());
        Ok(())
    }

    #[test]
    fn read_takes_one_string() -> TestResult {
        let (rig, _) = run_plan(
            &[SETTINGS],
            &FakeWorkspace::new(),
            &handler("  agentstacks.read(5)"),
        )?;
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:2: read: expected string, got number"]
        );
        Ok(())
    }

    #[test]
    fn read_outside_plan_is_refused() -> TestResult {
        let rig = Rig::with(&[SETTINGS], &[])?;
        let refused = rig.run(&format!("agentstacks.read('{SETTINGS}')")).err();
        assert_eq!(refused.as_deref(), Some("(recorded)"));
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:1: read: only allowed inside the plan handler"]
        );
        Ok(())
    }

    #[test]
    fn a_refused_read_is_refused_under_pcall_too() -> TestResult {
        let (rig, planned) = run_plan(
            &[SETTINGS],
            &FakeWorkspace::new(),
            &handler("  pcall(agentstacks.read, '.env')"),
        )?;
        planned.result?;
        assert_eq!(
            rig.refusals(),
            ["claude/adapter.lua:2: read: \".env\" is not in extension.reads"]
        );
        Ok(())
    }

    #[test]
    fn read_path_is_normalised() -> TestResult {
        let workspace = FakeWorkspace::new().file(&path(SETTINGS)?, b"x");
        let (rig, planned) = run_plan(
            &[SETTINGS],
            &workspace,
            &handler("  return { n = #agentstacks.read('./.claude//settings.json') }"),
        )?;
        assert_eq!(planned.result?, Dispatched::Value(json!({"n": 1})));
        assert!(rig.refusals().is_empty());
        Ok(())
    }

    #[test]
    fn eight_mib_read_fits_the_memory_limit() -> TestResult {
        const EIGHT_MIB: usize = 8 * 1024 * 1024;
        let workspace = FakeWorkspace::new().file(&path(SETTINGS)?, &vec![b'x'; EIGHT_MIB]);
        let (_, planned) = run_plan(
            &[SETTINGS],
            &workspace,
            &handler(&format!(
                "  return {{ n = #agentstacks.read('{SETTINGS}') }}"
            )),
        )?;
        assert_eq!(planned.result?, Dispatched::Value(json!({"n": EIGHT_MIB})));
        Ok(())
    }

    #[test]
    fn a_finalizer_that_reads_during_a_read_loop_completes() {
        let finished = finishes_within(Duration::from_secs(20), || -> Result<String, String> {
            let settings = path(SETTINGS).map_err(|error| error.to_string())?;
            let workspace = FakeWorkspace::new().file(&settings, &[b'x'; 4096]);
            let (_, planned) = run_plan(
                &[SETTINGS],
                &workspace,
                &handler(&format!(
                    "  collectgarbage('incremental', 1, 1000)\n\
                     local mt = {{ __gc = function() pcall(agentstacks.read, '{SETTINGS}') end }}\n\
                     for i = 1, 20000 do setmetatable({{}}, mt) local s = agentstacks.read('{SETTINGS}') end"
                )),
            )
            .map_err(|error| error.to_string())?;
            planned.result.map(|_| "done".to_owned())
        });
        assert_eq!(finished, Some(Ok("done".to_owned())));
    }
}
