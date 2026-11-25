//! Runs the shipped hook runtime on the real `airsl` binary, with the argument vector
//! `HookCommand` builds.
//!
//! The project under `tests/fixtures/runtime/proj` holds a small translator for a made-up tool and
//! a set of hook modules. Each test copies it into a temporary directory, writes the shipped runtime
//! beside the translator and runs one command from the project directory, as a tool does.
//! When `airsl` cannot run, each test says so and returns, unless `AGENTSTACKS_REQUIRE_AIRSL` is
//! set (CI sets it): then each test fails, so a missing binary cannot pass the suite unseen.

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, HookModule, RUNTIME_LUA};
use agentstacks::types::{HookAction, PackManifest, PackName, SubPath};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const ROOT: &str = ".tool/agentstacks";
const PRE_TOOL_USE: &str =
    r#"{"hook_event_name":"PreToolUse","session_id":"s1","cwd":"/p","tool_name":"Edit"}"#;
const NOTIFICATION: &str = r#"{"hook_event_name":"Notification","session_id":"s1","cwd":"/p"}"#;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runtime/proj")
}

fn copy_tree(from: &Path, to: &Path) -> TestResult {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The version `airsl` reports, or `None` when it cannot run.
fn airsl_version() -> Option<String> {
    let output = Command::new("airsl").arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// A copy of the fixture project with the shipped runtime written into it.
fn project() -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    copy_tree(&fixture(), dir.path())?;
    fs::write(dir.path().join(ROOT).join("runtime.lua"), RUNTIME_LUA)?;
    Ok(dir)
}

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

/// What one hook run needs: the hook's file name (`hooks/<hook>.lua` in the pack `core`) and the rest.
struct Case<'a> {
    hook: &'a str,
    blocking: Blocking,
    kind: HookKind,
    read: &'a [&'a str],
    args: &'a [&'a str],
    payload: &'a str,
}

impl<'a> Case<'a> {
    const fn new(hook: &'a str) -> Self {
        Self {
            hook,
            blocking: Blocking::MayNot,
            kind: HookKind::Neutral,
            read: &[],
            args: &[],
            payload: PRE_TOOL_USE,
        }
    }
}

fn owned(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// The variable that turns a missing `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// Runs the case on `airsl` from a fresh project directory, or `None` when `airsl` is absent and
/// not required.
fn run(case: &Case<'_>) -> Result<Option<Output>, Box<dyn Error>> {
    let Some(version) = airsl_version() else {
        if std::env::var_os(REQUIRE_AIRSL).is_some() {
            return Err(
                format!("airsl cannot run on this machine and {REQUIRE_AIRSL} is set").into(),
            );
        }
        eprintln!("skipped: airsl cannot run on this machine");
        return Ok(None);
    };
    eprintln!("ran on {version}");
    let dir = project()?;
    let argv = HookCommand::new(
        SubPath::new(ROOT)?,
        module(case.hook)?,
        case.blocking,
        case.kind,
    )
    .with_read(owned(case.read))
    .with_args(owned(case.args))
    .argv();
    let (program, rest) = argv.split_first().ok_or("a program")?;
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
        .write_all(case.payload.as_bytes())?;
    Ok(Some(child.wait_with_output()?))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn runtime_context_with_read_grant() -> TestResult {
    let case = Case {
        read: &[".tool/agentstacks/core"],
        args: &["--manifest", "x.json"],
        ..Case::new("ctx")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        concat!(
            r#"{"context":"event=tool.before session=s1 args=--manifest,x.json read=true:hello\n fs="#,
            "append atomic_write canonicalize copy create_exclusive exists is_dir is_file list ",
            "mkdir read read_lines remove remove_dir rename same_content stat tempdir tempfile ",
            "walk write\",\"event\":\"PreToolUse\"}\n"
        )
    );
    assert!(output.status.success());
    Ok(())
}

#[test]
fn runtime_read_without_grant_is_denied() -> TestResult {
    let Some(output) = run(&Case::new("ctx"))? else {
        return Ok(());
    };
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("read=false:fs.read denied:"),
        "stdout was {stdout}"
    );
    assert!(output.status.success());
    Ok(())
}

#[test]
fn runtime_drops_block_when_not_may_block() -> TestResult {
    let Some(output) = run(&Case::new("blk"))? else {
        return Ok(());
    };
    assert_eq!(text(&output.stdout), "");
    assert_eq!(
        text(&output.stderr),
        "agentstacks: hook core.hooks.blk tried to block but is not declared may_block; reason: edits to tool.before are not allowed\n"
    );
    Ok(())
}

#[test]
fn runtime_passes_block_when_may_block() -> TestResult {
    let case = Case {
        blocking: Blocking::May,
        ..Case::new("blk")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"decision\":\"block\",\"event\":\"PreToolUse\",\"reason\":\"edits to tool.before are not allowed\"}\n"
    );
    Ok(())
}

#[test]
fn runtime_native_gets_the_raw_payload() -> TestResult {
    let case = Case {
        kind: HookKind::Native,
        ..Case::new("raw")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"native payload tool_name=Edit event=nil\",\"event\":\"PreToolUse\"}\n"
    );
    Ok(())
}

#[test]
fn runtime_native_result_sees_the_payload_event() -> TestResult {
    let case = Case {
        kind: HookKind::Native,
        ..Case::new("native_ctx")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"ctx\",\"event\":\"PreToolUse\"}\n"
    );
    assert!(output.status.success());
    Ok(())
}

#[test]
fn runtime_native_gets_args() -> TestResult {
    let case = Case {
        kind: HookKind::Native,
        args: &["--flag", "v"],
        ..Case::new("native_args")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"native args=--flag,v\",\"event\":\"PreToolUse\"}\n"
    );
    Ok(())
}

#[test]
fn runtime_native_drops_block_when_not_may_block() -> TestResult {
    let case = Case {
        kind: HookKind::Native,
        ..Case::new("blk")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(text(&output.stdout), "");
    assert_eq!(
        text(&output.stderr),
        "agentstacks: hook core.hooks.blk tried to block but is not declared may_block; reason: edits to nil are not allowed\n"
    );
    Ok(())
}

#[test]
fn runtime_native_passes_block_when_may_block() -> TestResult {
    let case = Case {
        kind: HookKind::Native,
        blocking: Blocking::May,
        ..Case::new("blk")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"decision\":\"block\",\"event\":\"PreToolUse\",\"reason\":\"edits to nil are not allowed\"}\n"
    );
    assert_eq!(text(&output.stderr), "");
    Ok(())
}

#[test]
fn runtime_keeps_context_and_drops_block() -> TestResult {
    let Some(output) = run(&Case::new("both"))? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"keep this context\",\"event\":\"PreToolUse\"}\n"
    );
    assert_eq!(
        text(&output.stderr),
        "agentstacks: hook core.hooks.both tried to block but is not declared may_block; reason: and drop this block\n"
    );
    Ok(())
}

#[test]
fn runtime_untranslated_event_is_silent() -> TestResult {
    let case = Case {
        payload: NOTIFICATION,
        ..Case::new("ctx")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(text(&output.stdout), "");
    assert!(output.status.success());
    Ok(())
}

#[test]
fn runtime_missing_module_fails_open() -> TestResult {
    let Some(output) = run(&Case::new("missing"))? else {
        return Ok(());
    };
    assert_eq!(text(&output.stdout), "");
    assert!(output.status.success());
    Ok(())
}

const PATHS_WITH_STOP: &str = r#"{"paths":["a","stop","c"]}"#;

#[test]
fn runtime_list_joins_contexts() -> TestResult {
    let case = Case {
        payload: r#"{"paths":["a","b"]}"#,
        ..Case::new("each")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(text(&output.stdout), "{\"context\":\"each a\\neach b\"}\n");
    assert!(output.status.success());
    Ok(())
}

#[test]
fn runtime_list_stops_at_the_first_block() -> TestResult {
    let case = Case {
        blocking: Blocking::May,
        payload: PATHS_WITH_STOP,
        ..Case::new("each")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"block\":\"stop stop\",\"context\":\"each a\\neach stop\"}\n"
    );
    assert_eq!(text(&output.stderr), "");
    Ok(())
}

#[test]
fn runtime_list_drops_a_block_when_not_may_block() -> TestResult {
    let case = Case {
        payload: PATHS_WITH_STOP,
        ..Case::new("each")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stderr),
        "agentstacks: hook core.hooks.each tried to block but is not declared may_block; reason: stop stop\n"
    );
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"each a\\neach stop\\neach c\"}\n"
    );
    Ok(())
}

#[test]
fn runtime_list_gives_every_event_the_args() -> TestResult {
    let case = Case {
        args: &["--flag", "v"],
        payload: r#"{"paths":["a","b"]}"#,
        ..Case::new("each_args")
    };
    let Some(output) = run(&case)? else {
        return Ok(());
    };
    assert_eq!(
        text(&output.stdout),
        "{\"context\":\"a args=--flag,v\\nb args=--flag,v\"}\n"
    );
    Ok(())
}

#[test]
fn runtime_empty_list_gives_nil() -> TestResult {
    for payload in [r#"{"paths":[]}"#, r#"{"paths":["quiet","quiet"]}"#] {
        let case = Case {
            payload,
            ..Case::new("each")
        };
        let Some(output) = run(&case)? else {
            return Ok(());
        };
        assert_eq!(text(&output.stdout), "nil\n", "payload {payload}");
        assert!(output.status.success());
    }
    Ok(())
}
