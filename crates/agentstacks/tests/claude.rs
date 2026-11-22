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

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use agentstacks::app::plan::plan;
use agentstacks::fs::{StdFileTree, StdWorkspace};
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::op::SkipSubject;
use agentstacks::types::plan::{Action, Plan};
use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, HookModule, RUNTIME_LUA};
use agentstacks::types::{Extension, HookAction, PackManifest, PackName, SubPath};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The variable that turns a missing `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// The version `airsl` reports, or `None` when it cannot run.
fn airsl_version() -> Option<String> {
    let output = Command::new("airsl").arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Whether the test must stop: a pack with lua hooks needs the `airsl` binary, because the plan
/// checks it, and a translator run needs it to run the hook. When it cannot run, the test says so,
/// unless `AGENTSTACKS_REQUIRE_AIRSL` is set: then the test fails, so a missing binary cannot pass
/// the suite unseen.
fn skip_without_airsl() -> Result<bool, Box<dyn Error>> {
    if let Some(version) = airsl_version() {
        eprintln!("ran on {version}");
        return Ok(false);
    }
    if std::env::var_os(REQUIRE_AIRSL).is_some() {
        return Err(format!("airsl cannot run on this machine and {REQUIRE_AIRSL} is set").into());
    }
    eprintln!("skipped: airsl cannot run on this machine");
    Ok(true)
}

/// The shipped extension directory at the repository root.
fn extension_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extensions/claude")
}

fn fixture(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/claude")
        .join(case)
}

/// Copies `from` into `to`, leaving out a top-level entry named `skip`.
fn copy_tree(from: &Path, to: &Path, skip: Option<&str>) -> TestResult {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if skip.is_some_and(|name| entry.file_name() == name) {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, None)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A copy of the fixture project with the shipped extension at `extensions/claude`.
fn project(case: &str) -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    copy_tree(&fixture(case), dir.path(), Some("expected"))?;
    copy_tree(
        &extension_dir(),
        &dir.path().join("extensions/claude"),
        None,
    )?;
    Ok(dir)
}

/// Runs the real plan flow over `dir`; a refusal comes back as its lines.
fn run_plan(dir: &Path) -> Result<Result<Plan, Vec<String>>, Box<dyn Error>> {
    let workspace = StdWorkspace::open(dir)?;
    Ok(plan(
        &workspace,
        &GitCli::from_env(),
        &SystemBinary::from_env(),
        &AirslAdapter::new(),
    )
    .map_err(|found| {
        found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }))
}

/// The plan of the fixture `case`.
fn planned(case: &str) -> Result<(TempDir, Plan), Box<dyn Error>> {
    let dir = project(case)?;
    let plan = run_plan(dir.path())?.map_err(|lines| lines.join("\n"))?;
    Ok((dir, plan))
}

/// What a byte string looks like in a failed comparison.
fn show(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes).map_or_else(|_| format!("{bytes:?}"), str::to_owned)
}

/// The bytes the plan installs at each path.
fn proposed(plan: &Plan) -> BTreeMap<String, String> {
    plan.steps()
        .iter()
        .filter_map(|step| {
            step.new
                .as_deref()
                .map(|bytes| (step.path.as_str().to_owned(), show(bytes)))
        })
        .collect()
}

/// The checked-in expected files of `case`, keyed by project-relative path. `skips.txt` is not one.
fn expected(case: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> TestResult {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                walk(root, &entry.path(), out)?;
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)?
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if rel != "skips.txt" {
                out.insert(rel, show(&fs::read(entry.path())?));
            }
        }
        Ok(())
    }
    let root = fixture(case).join("expected");
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out)?;
    Ok(out)
}

/// The expected text of one file of `case`.
fn expected_file(case: &str, path: &str) -> Result<String, Box<dyn Error>> {
    expected(case)?
        .remove(path)
        .ok_or_else(|| format!("{case}/expected has no {path}").into())
}

/// The skips of the plan as `<item>: <reason>`, sorted.
fn skips(plan: &Plan) -> Vec<String> {
    let mut lines: Vec<String> = plan
        .skips()
        .iter()
        .map(|skip| {
            let subject = match skip.subject() {
                SkipSubject::Item(item) => item.to_string(),
                SkipSubject::RootFile { pack, rel } => format!("{pack}/{rel}"),
            };
            format!("{subject}: {}", skip.reason())
        })
        .collect();
    lines.sort();
    lines
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
    assert_eq!(written, [".claude/skills/a/SKILL.md"]);
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

/// The module of `hooks/<name>.lua` in the pack `core`.
fn module(name: &str) -> Result<HookModule, Box<dyn Error>> {
    let text = format!(
        "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n\n\
         [[hooks]]\nkind = \"lua\"\nevent = \"session.start\"\nrun = \"hooks/{name}.lua\"\n"
    );
    let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), &text)
        .map_err(|found| format!("{found:?}"))?;
    let hook = manifest.hooks().first().ok_or("one hook")?;
    let HookAction::Lua(lua) = hook.action() else {
        return Err("a lua hook".into());
    };
    Ok(HookModule::new(&PackName::new("core")?, lua.run()))
}

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
    let (program, rest) = argv.split_first().ok_or("a program")?;
    let payload = fs::read(fixture("payloads/input").join(format!("{}.json", case.payload)))?;
    let mut child = Command::new(program)
        .args(rest)
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("a stdin pipe")?
        .write_all(&payload)?;
    Ok(Some(child.wait_with_output()?))
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
fn empty_agent_type_maps_to_nil_agent() -> TestResult {
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
