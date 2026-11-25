//! Runs the shipped claude extension on the real host.
//!
//! Each plan test copies a fixture project and `extensions/claude` into a temporary directory and
//! runs the real `app::plan::plan` over it with `StdWorkspace` and `AirslAdapter`. What the
//! extension proposes is compared with the checked-in files under
//! `tests/fixtures/claude/<case>/expected`, which are written from the rules of the claude
//! extension. The one exception is the expected `translate.lua`, which is the translator text
//! itself; the translator tests below are what prove it.
//!
//! The translator tests run the `translate.lua` the full fixture expects, with the core runtime, on
//! the real `airsl` binary and the recorded payloads under `tests/fixtures/claude/payloads`.

pub mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Output;

use agentstacks::fs::StdFileTree;
use agentstacks::types::plan::{Action, Plan};
use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, RUNTIME_LUA};
use agentstacks::types::{Extension, SubPath};
use common::{copy_tree, module, proposed, run_piped, run_plan, skip_without_airsl, skips};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The tool these tests run.
const TOOL: &str = "claude";

/// The shipped extension directory at the repository root.
fn extension_dir() -> PathBuf {
    common::extension_dir(TOOL)
}

fn fixture(case: &str) -> PathBuf {
    common::fixture(TOOL, case)
}

/// A copy of the fixture project with the shipped extension at `extensions/claude`.
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

#[test]
fn claude_manifest_loads() -> TestResult {
    let tree = StdFileTree::new(extension_dir());
    let extension = Extension::load(&tree).map_err(|found| format!("{found:?}"))?;
    assert_eq!(extension.manifest().name().as_str(), "claude");
    assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
    let reads: Vec<String> = extension.reads().iter().map(SubPath::normalized).collect();
    assert_eq!(reads, [".claude/settings.json"]);
    Ok(())
}

#[test]
fn full_pack_renders_every_item_kind() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("full")?;
    let mut got = proposed(&plan);
    // The runtime is the core's own file; it is the shipped text, not the extension's.
    assert_eq!(
        got.remove(".claude/agentstacks/runtime.lua").as_deref(),
        Some(RUNTIME_LUA)
    );
    assert_eq!(got, expected("full")?);

    let want: Vec<String> = fs::read_to_string(fixture("full").join("expected/skips.txt"))?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(skips(&plan), want);
    Ok(())
}

/// The `.claude/settings.json` text the plan proposes.
fn settings_of(plan: &Plan) -> Result<String, Box<dyn Error>> {
    proposed(plan)
        .remove(".claude/settings.json")
        .ok_or_else(|| "no settings.json proposed".into())
}

#[test]
fn existing_settings_keep_user_keys_and_replace_the_marked_hook() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("settings")?;
    assert_eq!(
        settings_of(&plan)?,
        expected_file("settings", ".claude/settings.json")?
    );
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

#[test]
fn removed_tool_strips_only_marked_handlers() -> TestResult {
    let (_dir, plan) = planned("removed")?;
    assert_eq!(
        settings_of(&plan)?,
        expected_file("removed", ".claude/settings.json")?
    );
    // The removed tool proposes nothing else, and its locked file is deleted.
    let deleted: Vec<&str> = plan
        .steps()
        .iter()
        .filter(|step| step.action == Action::Delete)
        .map(|step| step.path.as_str())
        .collect();
    assert_eq!(deleted, [".claude/skills/gone/SKILL.md"]);
    let written: Vec<String> = proposed(&plan).into_keys().collect();
    assert_eq!(written, [".claude/settings.json"]);
    Ok(())
}

#[test]
fn hooks_left_empty_stays_an_empty_object() -> TestResult {
    let (_dir, plan) = planned("emptied")?;
    assert_eq!(
        settings_of(&plan)?,
        expected_file("emptied", ".claude/settings.json")?
    );
    Ok(())
}

#[test]
fn settings_absent_and_no_hooks_proposes_no_settings_write() -> TestResult {
    let (_dir, plan) = planned("nohooks")?;
    let written: Vec<String> = proposed(&plan).into_keys().collect();
    assert_eq!(written, [".claude/skills/core-a/SKILL.md"]);
    Ok(())
}

/// `text` with the line number of a `<file>:<line>:` location replaced by `<line>`.
fn without_line_number(text: &str, file: &str) -> String {
    let Some(at) = text.find(&format!("{file}:")) else {
        return text.to_owned();
    };
    let digits_from = at + file.len() + 1;
    let digits = text[digits_from..]
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    format!(
        "{}<line>{}",
        &text[..digits_from],
        &text[digits_from + digits..]
    )
}

#[test]
fn invalid_settings_json_refuses_the_plan() -> TestResult {
    let dir = project("badjson")?;
    let refused = run_plan(dir.path())?.err().ok_or("a refusal")?;
    let lines: Vec<String> = refused
        .iter()
        .map(|line| without_line_number(line, "adapter.lua"))
        .collect();
    assert_eq!(
        lines,
        ["claude/adapter.lua:<line>: json.edit: top level must be an object"]
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
            "core/hook/0: .claude/settings.json: hooks.PreToolUse holds entries the claude \
             extension cannot rewrite (a null, an empty list, or a value that is not a list of \
             hook groups)"
        ]
    );
    // The event is left as it was, so the file is proposed unchanged.
    let old = fs::read_to_string(fixture("nullevent").join(".claude/settings.json"))?;
    assert_eq!(settings_of(&plan)?, old);
    // The only hook is skipped, so no translator is written.
    assert!(!proposed(&plan).contains_key(".claude/agentstacks/translate.lua"));
    Ok(())
}

#[test]
fn no_lua_hook_writes_no_translate() -> TestResult {
    let (_dir, plan) = planned("nolua")?;
    let written = proposed(&plan);
    assert!(written.contains_key(".claude/settings.json"));
    assert!(!written.contains_key(".claude/agentstacks/translate.lua"));
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

/// Where the extension root sits in the project the translator tests build.
const ROOT: &str = ".claude/agentstacks";

/// One run of a hook module under the shipped runtime and the expected translator.
struct Run<'a> {
    /// The hook module under `payloads/hooks`.
    hook: &'a str,
    /// The recorded payload under `payloads/input`.
    payload: &'a str,
    blocking: Blocking,
    kind: HookKind,
}

impl<'a> Run<'a> {
    /// A neutral hook that may not block.
    const fn new(hook: &'a str, payload: &'a str) -> Self {
        Self {
            hook,
            payload,
            blocking: Blocking::MayNot,
            kind: HookKind::Neutral,
        }
    }

    /// The same run for a hook declared `may_block`.
    const fn may_block(mut self) -> Self {
        self.blocking = Blocking::May;
        self
    }

    /// The same run for a native hook.
    const fn native(mut self) -> Self {
        self.kind = HookKind::Native;
        self
    }
}

/// A project directory holding the expected translator, the shipped runtime and the hook modules.
fn translator_project() -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join(ROOT);
    fs::create_dir_all(root.join("core/hooks"))?;
    fs::write(root.join("runtime.lua"), RUNTIME_LUA)?;
    fs::copy(
        fixture("full")
            .join("expected")
            .join(ROOT)
            .join("translate.lua"),
        root.join("translate.lua"),
    )?;
    copy_tree(
        &fixture("payloads").join("hooks"),
        &root.join("core/hooks"),
        None,
    )?;
    Ok(dir)
}

/// Runs `case` with the argument vector `HookCommand` builds and the payload on stdin, from the
/// project directory, or returns `None` when `airsl` cannot run and is not required.
fn run_hook(case: &Run<'_>) -> Result<Option<Output>, Box<dyn Error>> {
    if skip_without_airsl()? {
        return Ok(None);
    }
    let dir = translator_project()?;
    let argv = HookCommand::new(
        SubPath::new(ROOT)?,
        module(case.hook)?,
        case.blocking,
        case.kind,
    )
    .argv();
    let payload = fs::read(fixture("payloads/input").join(format!("{}.json", case.payload)))?;
    Ok(Some(run_piped(&argv, dir.path(), &payload)?))
}

/// The expected stdout under `payloads/expected/<name>.out` or stderr under `<name>.err`.
fn recorded(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(fixture("payloads/expected").join(name))?)
}

/// Runs `case` and compares stdout with `<out>.out` and stderr with `<err>.err`, or with nothing
/// when `err` is `None`.
fn check(case: &Run<'_>, out: &str, err: Option<&str>) -> TestResult {
    let Some(output) = run_hook(case)? else {
        return Ok(());
    };
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        recorded(&format!("{out}.out"))?
    );
    let want_err = match err {
        Some(name) => recorded(&format!("{name}.err"))?,
        None => String::new(),
    };
    assert_eq!(String::from_utf8_lossy(&output.stderr), want_err);
    assert!(output.status.success());
    Ok(())
}

#[test]
fn pre_tool_use_block_prints_deny() -> TestResult {
    check(&Run::new("blk", "read").may_block(), "deny", None)
}

#[test]
fn post_tool_use_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "write").may_block(), "block", None)
}

#[test]
fn session_start_context_prints_additional_context() -> TestResult {
    check(&Run::new("ctx", "start"), "context", None)
}

#[test]
fn subagent_stop_context_only_prints_nothing() -> TestResult {
    check(&Run::new("ctx", "subagent"), "empty", None)
}

#[test]
fn subagent_stop_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "subagent").may_block(), "block", None)
}

#[test]
fn user_prompt_submit_block_prints_decision_block() -> TestResult {
    check(&Run::new("blk", "prompt").may_block(), "block", None)
}

#[test]
fn session_end_prints_nothing() -> TestResult {
    check(&Run::new("both", "end").may_block(), "empty", None)
}

#[test]
fn bash_tool_is_not_run() -> TestResult {
    // The echo hook would write to stderr if it ran, so an empty stderr shows it did not.
    check(&Run::new("echo", "bash"), "empty", None)
}

#[test]
fn native_hook_on_stop_prints_nothing() -> TestResult {
    check(
        &Run::new("both", "stop").may_block().native(),
        "empty",
        None,
    )
}

#[test]
fn echo_maps_each_event() -> TestResult {
    for name in ["start", "end", "prompt", "read", "write", "subagent"] {
        check(
            &Run::new("echo", name),
            "empty",
            Some(&format!("echo/{name}")),
        )?;
    }
    Ok(())
}

#[test]
fn fork_source_maps_to_nil() -> TestResult {
    check(&Run::new("echo", "fork"), "empty", Some("echo/fork"))
}

#[test]
fn empty_agent_type_and_id_map_to_nil() -> TestResult {
    check(&Run::new("echo", "noagent"), "empty", Some("echo/noagent"))
}

#[test]
fn session_start_with_context_and_block_joins_them() -> TestResult {
    check(&Run::new("both", "start").may_block(), "joined", None)
}

#[test]
fn nil_result_prints_nothing() -> TestResult {
    check(&Run::new("none", "start"), "empty", None)
}
