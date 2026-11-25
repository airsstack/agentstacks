//! Runs the shipped opencode extension on the real host.
//!
//! Each plan test copies a fixture project and `extensions/opencode` into a temporary directory and
//! runs the real `app::plan::plan` over it with `StdWorkspace` and `AirslAdapter`. What the
//! extension proposes is compared with the checked-in files under
//! `tests/fixtures/opencode/<case>/expected`, which are written from the rules of the opencode
//! extension, with three exceptions: the expected `translate.lua` is the translator text itself, the
//! header and body of the expected `agentstacks.js` are the fixed text of `lib/plugin.lua` (only its
//! hook table is written by hand), and `both_tools_plan_without_clash` has no `expected` tree and
//! checks that the paths of both tools are proposed.
//!
//! The translator tests run the `translate.lua` the full fixture expects, with the core runtime, on
//! the real `airsl` binary and the payloads under `tests/fixtures/opencode/payloads`, and compare
//! stdout with the hand-written files under `payloads/expected`.
//!
//! The plan tests of packs with lua hooks need the `airsl` binary because the plan checks it, and the
//! translator tests need it because they run it; both stop early when it cannot run (see
//! `skip_without_airsl`).

pub mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use agentstacks::app::apply::apply;
use agentstacks::fs::{StdFileTree, StdWorkspace};
use agentstacks::types::plan::{Action, Plan};
use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, RUNTIME_LUA};
use agentstacks::types::{Extension, SubPath};
use common::{copy_tree, module, proposed, run_piped, run_plan, skip_without_airsl, skips};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// Where the opencode extension's runtime, translator and shim are written.
const RUNTIME: &str = ".opencode/agentstacks/runtime.lua";
const TRANSLATE: &str = ".opencode/agentstacks/translate.lua";
const SHIM: &str = ".opencode/plugins/agentstacks.js";

/// The tool these tests run.
const TOOL: &str = "opencode";

/// The shipped extension directory at the repository root.
fn extension_dir() -> PathBuf {
    common::extension_dir(TOOL)
}

fn fixture(case: &str) -> PathBuf {
    common::fixture(TOOL, case)
}

/// A copy of the fixture project with the shipped extension at `extensions/opencode`.
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

#[test]
fn opencode_manifest_loads() -> TestResult {
    // `Extension::load` refuses a manifest whose `api` is not the core's (0), so a load that
    // succeeds is what proves the extension declares api 0.
    let tree = StdFileTree::new(extension_dir());
    let extension = Extension::load(&tree).map_err(|found| format!("{found:?}"))?;
    assert_eq!(extension.manifest().name().as_str(), "opencode");
    assert_eq!(extension.manifest().entry().as_str(), "adapter.lua");
    assert_eq!(extension.manifest().version().to_string(), "0.1.0");
    assert!(extension.reads().is_empty());
    Ok(())
}

#[test]
fn full_pack_writes_skills_commands_agents_and_native_files() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("full")?;
    assert_eq!(without_runtime(&plan), expected("full")?);

    let want: Vec<String> = fs::read_to_string(fixture("full").join("expected/skips.txt"))?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(skips(&plan), want);
    Ok(())
}

#[test]
fn skill_with_bad_native_key_is_skipped_without_command() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("full")?;
    assert!(
        skips(&plan).contains(
            &"full/skill/odd: native.opencode key \"bad key\" cannot be written as frontmatter"
                .to_owned()
        ),
        "{:?}",
        skips(&plan)
    );
    let odd: Vec<String> = proposed(&plan)
        .into_keys()
        .filter(|path| path.contains("full-odd"))
        .collect();
    assert!(odd.is_empty(), "{odd:?}");
    Ok(())
}

#[test]
fn agent_with_bad_native_key_is_skipped_without_file() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("full")?;
    assert!(
        skips(&plan).contains(
            &"full/agent/odd: native.opencode key \"bad key\" cannot be written as frontmatter"
                .to_owned()
        ),
        "{:?}",
        skips(&plan)
    );
    assert!(!proposed(&plan).contains_key(".opencode/agents/full-odd.md"));
    Ok(())
}

#[test]
fn full_pack_renders_hooks_into_the_shim() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("full")?;
    // The whole-tree comparison in `full_pack_writes_skills_commands_agents_and_native_files`
    // covers the bytes; this test names the shim, the translator and the hook skips.
    let got = proposed(&plan);
    assert!(got.contains_key(SHIM), "no shim proposed: {:?}", got.keys());
    assert!(
        got.contains_key(TRANSLATE),
        "no translator proposed: {:?}",
        got.keys()
    );

    let reasons: Vec<String> = fs::read_to_string(fixture("full").join("expected/skips.txt"))?
        .lines()
        .filter(|line| line.contains("/hook/") || line.contains("/native-hook/"))
        .map(str::to_owned)
        .collect();
    assert_eq!(reasons.len(), 5);
    let all = skips(&plan);
    for line in reasons {
        assert!(all.contains(&line), "{line} not in {all:?}");
    }
    Ok(())
}

#[test]
fn no_hooks_writes_an_empty_shim_and_no_translator() -> TestResult {
    let (_dir, plan) = planned("nohooks")?;
    let got = proposed(&plan);
    assert!(
        got.get(SHIM)
            .is_some_and(|text| text.contains("const HOOKS = [];\n")),
        "{:?}",
        got.get(SHIM)
    );
    assert!(!got.contains_key(TRANSLATE));
    assert!(!got.contains_key(RUNTIME));
    assert_eq!(got, expected("nohooks")?);
    assert!(skips(&plan).is_empty(), "{:?}", skips(&plan));
    Ok(())
}

#[test]
fn every_lua_hook_skipped_writes_no_translator() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (_dir, plan) = planned("endonly")?;
    // The core still writes the runtime; the extension writes no translator.
    assert!(!proposed(&plan).contains_key(TRANSLATE));
    assert_eq!(without_runtime(&plan), expected("endonly")?);
    let want: Vec<String> = fs::read_to_string(fixture("endonly").join("expected/skips.txt"))?
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(skips(&plan), want);
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
        SHIM,
        ".opencode/skills/core-a/SKILL.md",
        ".opencode/commands/core-a.md",
    ] {
        assert!(
            got.contains_key(path),
            "{path} not proposed: {:?}",
            got.keys()
        );
    }
    Ok(())
}

#[test]
fn plan_is_clean_after_apply() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let (dir, first) = planned("full")?;
    apply(&first, &StdWorkspace::open(dir.path())?).map_err(|found| format!("{found:?}"))?;
    let second = run_plan(dir.path())?.map_err(|lines| lines.join("\n"))?;
    let changed: Vec<_> = second
        .steps()
        .iter()
        .filter(|step| step.action != Action::Unchanged)
        .map(|step| (step.path.as_str().to_owned(), step.action))
        .collect();
    assert!(changed.is_empty(), "{changed:?}");
    assert!(!second.steps().is_empty());
    Ok(())
}

/// Where the extension root sits in the project the translator tests build.
const ROOT: &str = ".opencode/agentstacks";

/// A project directory holding the expected translator, the shipped runtime and the hook modules.
fn translator_project() -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join(ROOT);
    fs::create_dir_all(root.join("core/hooks"))?;
    fs::write(root.join("runtime.lua"), RUNTIME_LUA)?;
    fs::copy(
        fixture("full").join("expected").join(TRANSLATE),
        root.join("translate.lua"),
    )?;
    copy_tree(
        &fixture("payloads").join("hooks"),
        &root.join("core/hooks"),
        None,
    )?;
    Ok(dir)
}

/// Runs the hook module `hook` on `payloads/<payload>.json` with the argument vector `HookCommand`
/// builds, from the project directory, and compares stdout with `payloads/expected/<out>.out` byte
/// for byte. Stderr must be empty and the exit status zero. Does nothing when `airsl` cannot run and
/// is not required.
fn check_to(hook: &str, blocking: Blocking, payload: &str, out: &str) -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let dir = translator_project()?;
    let argv = HookCommand::new(
        SubPath::new(ROOT)?,
        module(hook)?,
        blocking,
        HookKind::Neutral,
    )
    .argv();
    let input = fs::read(fixture("payloads").join(format!("{payload}.json")))?;
    let output = run_piped(&argv, dir.path(), &input)?;
    let want = fs::read_to_string(fixture("payloads/expected").join(format!("{out}.out")))?;
    assert_eq!(String::from_utf8_lossy(&output.stdout), want);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert!(output.status.success());
    Ok(())
}

/// [`check_to`] with the expected stdout named after the payload.
fn check(hook: &str, blocking: Blocking, payload: &str) -> TestResult {
    check_to(hook, blocking, payload, payload)
}

#[test]
fn translator_read_gives_context() -> TestResult {
    check("ctx", Blocking::MayNot, "read")
}

#[test]
fn translator_write_block_passes() -> TestResult {
    check("blk", Blocking::May, "write")
}

#[test]
fn translator_patch_splits_per_file() -> TestResult {
    check("ctx", Blocking::MayNot, "patch-add-update-write")?;
    check("ctx", Blocking::MayNot, "patch-add-update")
}

#[test]
fn translator_patch_move_uses_new_path() -> TestResult {
    check("ctx", Blocking::MayNot, "patch-move")
}

#[test]
fn translator_patch_delete_only_runs_nothing() -> TestResult {
    check("ctx", Blocking::MayNot, "patch-delete")
}

#[test]
fn translator_subagent_names_agent() -> TestResult {
    check("ctx", Blocking::MayNot, "subagent-read")
}

#[test]
fn translator_session_start_joins_block() -> TestResult {
    check("both", Blocking::May, "session-start")
}

#[test]
fn translator_prompt_gives_context() -> TestResult {
    check("ctx", Blocking::MayNot, "prompt")
}

#[test]
fn translator_other_tool_is_silent() -> TestResult {
    check("ctx", Blocking::MayNot, "bash")
}

#[test]
fn translator_hook_returning_nil_is_silent() -> TestResult {
    check_to("none", Blocking::MayNot, "read", "nil-result")
}
