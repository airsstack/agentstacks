//! Runs the shipped musecode extension on the real host.
//!
//! Each plan test copies a fixture project and `extensions/musecode` into a temporary directory and
//! runs the real `app::plan::plan` over it with `StdWorkspace` and `AirslAdapter`. What the
//! extension proposes is compared with the checked-in files under
//! `tests/fixtures/musecode/<case>/expected`, which are written from the rules of the musecode
//! extension, with one exception: the expected `translate.lua` is the translator text itself. The
//! core's own `runtime.lua` is the shipped text and is taken out before the comparison. Only
//! `demo`, `full` and `allskipped` have a whole `expected` tree. `hooksfile` and `removed` have an
//! `expected/.muse/hooks.json` and nothing else, `nullevent` is compared with its own input file,
//! and `nohooks`, `nolua`, `badjson` and `both` have no `expected` tree: their tests check which
//! paths the plan proposes, or the refusal.
//!
//! The plan tests of packs with lua hooks need the `airsl` binary because the plan checks it; they
//! stop early when it cannot run (see `skip_without_airsl`).
//!
//! The translator tests run the `translate.lua` the full fixture expects, with the core runtime, on
//! the real `airsl` binary and the recorded Muse Code payloads under
//! `tests/fixtures/musecode/payloads`, and compare stdout and stderr with the files under
//! `payloads/expected` (stderr is empty except for the `echo` hook).

pub mod common;
pub mod translator;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use agentstacks::app::plan::plan;
use agentstacks::fs::{StdFileTree, StdWorkspace};
use agentstacks::host::AirslAdapter;
use agentstacks::port::{Adapter, FileTree, Workspace};
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::op::{Operation, PlanInput, Proposals, ValuesInput, ValuesResult};
use agentstacks::types::plan::{Action, Plan, Reads};
use agentstacks::types::protocol::RUNTIME_LUA;
use agentstacks::types::report::Diagnostics;
use agentstacks::types::{Extension, SubPath};
use common::tree::copy_tree;
use common::{proposed, run_plan, skip_without_airsl, skips};
use tempfile::TempDir;
use translator::run::Run;

type TestResult = Result<(), Box<dyn Error>>;

/// Where the core writes its runtime.
const RUNTIME: &str = ".muse/agentstacks/runtime.lua";

/// Where the extension writes the translator.
const TRANSLATE: &str = ".muse/agentstacks/translate.lua";

/// The hooks file the user and the extension share.
const HOOKS: &str = ".muse/hooks.json";

/// The tool these tests run.
const TOOL: &str = "musecode";

/// The shipped extension directory at the repository root.
fn extension_dir() -> PathBuf {
    common::extension_dir(TOOL)
}

fn fixture(case: &str) -> PathBuf {
    common::fixture(TOOL, case)
}

/// A copy of the fixture project with the shipped extension at `extensions/musecode`.
fn project(case: &str) -> Result<TempDir, Box<dyn Error>> {
    common::project(TOOL, case)
}

/// The plan of the fixture `case`.
fn planned(case: &str) -> Result<(TempDir, Plan), Box<dyn Error>> {
    common::planned(TOOL, case)
}

/// The checked-in expected files of `case`, keyed by project-relative path. `skips.txt` is not one.
fn expected(case: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    common::expected(TOOL, case)
}

/// The expected text of one file of `case`.
fn expected_file(case: &str, path: &str) -> Result<String, Box<dyn Error>> {
    expected(case)?
        .remove(path)
        .ok_or_else(|| format!("{case}/expected has no {path}").into())
}

/// The bytes the plan installs, without the core's own `runtime.lua`, which is the shipped text.
fn without_runtime(plan: &Plan) -> BTreeMap<String, String> {
    let mut got = proposed(plan);
    assert_eq!(
        got.remove(RUNTIME).as_deref(),
        Some(RUNTIME_LUA),
        "the plan holds the shipped runtime"
    );
    got
}

/// The `.muse/hooks.json` text the plan proposes.
fn hooks_of(plan: &Plan) -> Result<String, Box<dyn Error>> {
    proposed(plan)
        .remove(HOOKS)
        .ok_or_else(|| "no hooks file proposed".into())
}

/// The real adapter, plus a note of the `covers` of the `.muse/hooks.json` write the extension
/// proposed. A plan step does not keep `covers`, so this is where they can be read.
struct Recording {
    inner: AirslAdapter,
    hooks_covers: RefCell<Option<Vec<String>>>,
}

impl Adapter for Recording {
    type Session = <AirslAdapter as Adapter>::Session;

    fn open<T: FileTree>(
        &self,
        extension: &Extension,
        files: &T,
    ) -> Result<Self::Session, Diagnostics> {
        self.inner.open(extension, files)
    }

    fn values(
        &self,
        session: &mut Self::Session,
        input: &ValuesInput,
    ) -> Result<ValuesResult, Diagnostics> {
        self.inner.values(session, input)
    }

    fn plan<W: Workspace>(
        &self,
        session: &mut Self::Session,
        input: &PlanInput,
        workspace: &W,
        proposals: &mut Proposals,
    ) -> Result<Reads, Diagnostics> {
        let reads = self.inner.plan(session, input, workspace, proposals)?;
        for proposal in proposals.iter() {
            if let Operation::Write { path, covers, .. } = &proposal.op
                && path.as_str() == HOOKS
            {
                let ids = covers.iter().map(ToString::to_string).collect();
                *self.hooks_covers.borrow_mut() = Some(ids);
            }
        }
        Ok(reads)
    }
}

/// The plan of the fixture `case` and the `covers` of its hooks file write, in the order the
/// extension gave them.
fn planned_with_hooks_covers(case: &str) -> Result<(TempDir, Plan, Vec<String>), Box<dyn Error>> {
    let dir = project(case)?;
    let recording = Recording {
        inner: AirslAdapter::new(),
        hooks_covers: RefCell::new(None),
    };
    let plan = plan(
        &StdWorkspace::open(dir.path())?,
        &GitCli::from_env(),
        &SystemBinary::from_env(),
        &recording,
    )
    .map_err(|found| format!("{found:?}"))?;
    let covers = recording
        .hooks_covers
        .take()
        .ok_or("the extension proposed no hooks file")?;
    Ok((dir, plan, covers))
}

#[test]
fn musecode_manifest_loads() -> TestResult {
    // `Extension::load` refuses a manifest whose `api` is not the core's (1), so a load that
    // succeeds is what proves the extension declares api 1.
    let tree = StdFileTree::new(extension_dir());
    let extension = Extension::load(&tree).map_err(|found| format!("{found:?}"))?;
    assert_eq!(extension.manifest().name().as_str(), "musecode");
    assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
    assert_eq!(extension.manifest().version().to_string(), "1.1.0");
    let reads: Vec<&str> = extension.reads().iter().map(SubPath::as_str).collect();
    assert_eq!(reads, [HOOKS]);
    Ok(())
}

#[test]
fn demo_pack_plan_matches() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("demo")?;
    assert_eq!(without_runtime(&plan), expected("demo")?);
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

#[test]
fn full_pack_renders_every_item_kind() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan, covers) = planned_with_hooks_covers("full")?;
    assert_eq!(without_runtime(&plan), expected("full")?);

    let want: Vec<String> = fs::read_to_string(fixture("full").join("expected/skips.txt"))?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(skips(&plan), want);
    // Every rendered hook, sorted; the two skipped hooks (1 and 5) are not covered.
    assert_eq!(
        covers,
        [
            "full/hook/0",
            "full/hook/2",
            "full/hook/3",
            "full/hook/4",
            "full/hook/6",
            "full/native-hook/0",
            "full/native-hook/1",
        ]
    );
    Ok(())
}

#[test]
fn existing_hooks_file_keeps_user_group_and_replaces_marked() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("hooksfile")?;
    let got = hooks_of(&plan)?;
    assert_eq!(got, expected_file("hooksfile", HOOKS)?);
    assert!(
        !got.contains("echo old"),
        "the old marked handler stays: {got}"
    );
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

#[test]
fn removed_tool_strips_only_marked_handlers() -> TestResult {
    let (_dir, plan) = planned("removed")?;
    assert_eq!(hooks_of(&plan)?, expected_file("removed", HOOKS)?);
    // The removed tool proposes nothing else, and its locked file is deleted.
    let deleted: Vec<&str> = plan
        .steps()
        .iter()
        .filter(|step| step.action == Action::Delete)
        .map(|step| step.path.as_str())
        .collect();
    assert_eq!(deleted, [".agents/skills/gone/SKILL.md"]);
    let written: Vec<String> = proposed(&plan).into_keys().collect();
    assert_eq!(written, [HOOKS]);
    Ok(())
}

#[test]
fn hooks_file_absent_and_no_hooks_proposes_no_write() -> TestResult {
    let (_dir, plan) = planned("nohooks")?;
    let written: Vec<String> = proposed(&plan).into_keys().collect();
    assert_eq!(written, [".agents/skills/core-a/SKILL.md"]);
    Ok(())
}

#[test]
fn invalid_hooks_json_refuses_the_plan() -> TestResult {
    let dir = project("badjson")?;
    let refused = run_plan(dir.path())?.err().ok_or("a refusal")?;
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].starts_with("musecode/adapter.lua:"),
        "{refused:?}"
    );
    assert!(
        refused[0].contains("json.edit: trailing comma"),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn unrewritable_event_skips_our_hook() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("nullevent")?;
    assert_eq!(
        skips(&plan),
        [
            "core/hook/0: .muse/hooks.json: hooks.PreToolUse holds entries the musecode \
             extension cannot rewrite (a null, an empty list, or a value that is not a list of \
             hook groups)"
        ]
    );
    // The event is left as it was, so the file is proposed unchanged and keeps the user's null.
    let old = fs::read_to_string(fixture("nullevent").join(HOOKS))?;
    assert_eq!(hooks_of(&plan)?, old);
    // The only hook is skipped, so no translator is written.
    assert!(!proposed(&plan).contains_key(TRANSLATE));
    Ok(())
}

#[test]
fn no_lua_hook_writes_no_translate() -> TestResult {
    let (_dir, plan) = planned("nolua")?;
    let written = proposed(&plan);
    assert!(written.contains_key(HOOKS));
    assert!(!written.contains_key(TRANSLATE));
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

#[test]
fn every_lua_hook_skipped_writes_runtime_but_no_translate() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("allskipped")?;
    // The core still writes the runtime; the extension writes no translator.
    assert!(!proposed(&plan).contains_key(TRANSLATE));
    assert_eq!(without_runtime(&plan), expected("allskipped")?);
    let want: Vec<String> = fs::read_to_string(fixture("allskipped").join("expected/skips.txt"))?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(skips(&plan), want);
    Ok(())
}

#[test]
fn both_tools_plan_without_clash() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let dir = project("both")?;
    copy_tree(
        &extension_dir().with_file_name("claude"),
        &dir.path().join("extensions/claude"),
        None,
    )?;
    let plan = run_plan(dir.path())?.map_err(|lines| lines.join("\n"))?;
    let got = proposed(&plan);
    // The claude side is checked for presence only; its own tests compare its bytes.
    for path in [
        ".claude/agentstacks/runtime.lua",
        ".claude/agentstacks/translate.lua",
        ".claude/settings.json",
        ".claude/skills/core-a/SKILL.md",
        RUNTIME,
        TRANSLATE,
        HOOKS,
        ".agents/skills/core-a/SKILL.md",
    ] {
        assert!(
            got.contains_key(path),
            "{path} not proposed: {:?}",
            got.keys()
        );
    }
    Ok(())
}

/// Where the extension root sits in the project the translator tests build.
const ROOT: &str = ".muse/agentstacks";

/// Runs `case` and compares stdout with `<out>.out` and stderr with `<err>.err`, or with nothing
/// when `err` is `None`.
fn check(case: &Run<'_>, out: &str, err: Option<&str>) -> TestResult {
    translator::run::check(TOOL, ROOT, case, out, err)
}

#[test]
fn echo_maps_each_event() -> TestResult {
    for name in [
        "start", "end", "prompt", "read", "edit", "write", "postread", "postedit", "stop",
    ] {
        check(
            &Run::new("echo", name),
            "empty",
            Some(&format!("echo/{name}")),
        )?;
    }
    // A bash call gives no tool event, so the hook does not run and prints nothing.
    check(&Run::new("echo", "bash"), "empty", None)?;
    Ok(())
}

#[test]
fn pre_tool_use_block_prints_deny_with_absolute_path() -> TestResult {
    check(&Run::new("blk", "write").may_block(), "pre-deny", None)
}

#[test]
fn pre_tool_use_context_prints_additional_context() -> TestResult {
    check(&Run::new("ctx", "read"), "pre-context", None)
}

#[test]
fn pre_tool_use_context_and_deny_prints_both() -> TestResult {
    check(&Run::new("both", "write").may_block(), "pre-both", None)
}

#[test]
fn post_tool_use_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "postread").may_block(), "post-block", None)
}

#[test]
fn post_tool_use_context_prints_additional_context() -> TestResult {
    check(&Run::new("ctx", "postedit"), "post-context", None)
}

#[test]
fn user_prompt_submit_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "prompt").may_block(), "prompt-block", None)
}

#[test]
fn session_start_context_prints_additional_context() -> TestResult {
    check(&Run::new("ctx", "start"), "start-context", None)
}

#[test]
fn session_start_with_context_and_block_joins_them() -> TestResult {
    check(&Run::new("both", "start").may_block(), "start-joined", None)
}

#[test]
fn subagent_stop_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "stop").may_block(), "stop-block", None)
}

#[test]
fn subagent_stop_context_only_prints_nothing() -> TestResult {
    check(&Run::new("ctx", "stop"), "empty", None)
}

#[test]
fn session_end_prints_nothing() -> TestResult {
    check(&Run::new("ctx", "end"), "empty", None)
}

#[test]
fn bash_tool_is_not_run() -> TestResult {
    // The block hook would print a deny if it ran, so an empty stdout shows it did not.
    check(&Run::new("blk", "bash").may_block(), "empty", None)
}

#[test]
fn fork_source_maps_to_nil() -> TestResult {
    check(&Run::new("echo", "fork"), "empty", Some("echo/fork"))
}

#[test]
fn empty_path_passes_through() -> TestResult {
    check(
        &Run::new("echo", "emptypath"),
        "empty",
        Some("echo/emptypath"),
    )
}

#[test]
fn empty_cwd_passes_path_through() -> TestResult {
    check(
        &Run::new("echo", "emptycwd"),
        "empty",
        Some("echo/emptycwd"),
    )
}

#[test]
fn nil_result_prints_nothing() -> TestResult {
    check(&Run::new("none", "read"), "empty", None)
}
