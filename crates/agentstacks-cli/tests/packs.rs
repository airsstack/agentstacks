//! The five official packs installed together through the real `agentstacks` binary.
//!
//! Each install test copies the repository's `packs/`, `extensions/claude` and
//! `extensions/opencode` into a temporary git project, runs `init` for all five packs on one tool
//! or on both and checks the result; the unit tests at the end only cover the version parse.
//! Nothing here proves a rule on its own: the rules are proven in the library and the Lua tests,
//! and these runs show that the packs install side by side, that every rendered hook runs under its
//! prefixed module name, and that no rendered file keeps unconverted pack syntax.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The variable that turns a missing or too old `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// The oldest `airsl` the packs run on.
const MIN_AIRSL: (u32, u32, u32) = (0, 1, 5);

/// The matcher of the hook that points a read of a source file at its guideline.
const READ_MATCHER: &str = "Read|Edit|Write";

/// [`MIN_AIRSL`] as `major.minor.patch` text.
fn min_airsl_text() -> String {
    let (major, minor, patch) = MIN_AIRSL;
    format!("{major}.{minor}.{patch}")
}

/// Whether `airsl --version` reports at least [`MIN_AIRSL`].
fn airsl_is_recent_enough() -> bool {
    let Ok(output) = Command::new("airsl").arg("--version").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(text) = String::from_utf8(output.stdout) else {
        return false;
    };
    parse_version(&text).is_some_and(|version| version.at_least(MIN_AIRSL))
}

/// The version an `airsl --version` line reports.
#[derive(Debug, PartialEq, Eq)]
struct Version {
    core: (u32, u32, u32),
    /// Whether the version carries a pre-release suffix such as `-rc1`.
    pre_release: bool,
}

impl Version {
    /// Whether this version is at least `min`, ordered as semver orders it: a pre-release is
    /// below the release it precedes (`0.1.5-rc1` is below `0.1.5`) and above every earlier one.
    fn at_least(&self, min: (u32, u32, u32)) -> bool {
        self.core > min || (self.core == min && !self.pre_release)
    }
}

/// The version in the last word of an `airsl 0.1.5` style line.
fn parse_version(text: &str) -> Option<Version> {
    let word = text.split_whitespace().last()?;
    let (numbers, pre_release) = word
        .split_once('-')
        .map_or((word, false), |(numbers, _)| (numbers, true));
    let mut parts = numbers.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some(Version {
        core: (major, minor, patch),
        pre_release,
    })
}

/// Whether the test must stop: the packs hold lua hooks and need [`MIN_AIRSL`] or later. When it is
/// missing or older the test says so, unless `AGENTSTACKS_REQUIRE_AIRSL` is set: then it fails, so
/// a missing binary cannot pass the suite unseen.
fn skip_without_airsl() -> Result<bool, Box<dyn Error>> {
    if airsl_is_recent_enough() {
        return Ok(false);
    }
    let min = min_airsl_text();
    if std::env::var_os(REQUIRE_AIRSL).is_some() {
        return Err(format!(
            "airsl {min} or later cannot run on this machine and {REQUIRE_AIRSL} is set"
        )
        .into());
    }
    eprintln!("skipped: airsl {min} or later cannot run on this machine");
    Ok(true)
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

/// A temporary git project holding a copy of the official packs and of the claude and opencode
/// extensions, at `ext/claude` and `ext/opencode`.
fn project() -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    copy_tree(&manifest.join("../../packs"), &dir.path().join("packs"))?;
    copy_tree(
        &manifest.join("../../extensions/claude"),
        &dir.path().join("ext/claude"),
    )?;
    copy_tree(
        &manifest.join("../../extensions/opencode"),
        &dir.path().join("ext/opencode"),
    )?;
    Ok(dir)
}

/// Runs the binary with `-C <dir>` and stdin null.
fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_agentstacks"))
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()?)
}

/// `init` for the claude and opencode extensions and all five packs, which must succeed.
fn init_both(dir: &Path) -> TestResult {
    init_tools(dir, &["./ext/claude", "./ext/opencode"])
}

/// `init` for the claude extension and all five packs, which must succeed.
fn init_all(dir: &Path) -> TestResult {
    init_tools(dir, &["./ext/claude"])
}

/// `init` for the given tool extensions and all five packs, which must succeed.
fn init_tools(dir: &Path, tools: &[&str]) -> TestResult {
    let mut args = vec!["init"];
    for tool in tools {
        args.extend(["--tool", tool]);
    }
    for pack in [
        "./packs/core",
        "./packs/sdlc",
        "./packs/journal",
        "./packs/guideline-rust",
        "./packs/guideline-python",
    ] {
        args.extend(["--pack", pack]);
    }
    args.push("--yes");
    let output = run(dir, &args)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// The names of the entries of a directory, sorted.
fn entry_names(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = fs::read_dir(dir)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    names.sort();
    Ok(names)
}

/// The groups of one hook event in `.claude/settings.json`.
fn hook_groups(dir: &Path, event: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let settings: Value =
        serde_json::from_str(&fs::read_to_string(dir.join(".claude/settings.json"))?)?;
    let groups = settings
        .pointer(&format!("/hooks/{event}"))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("settings.json has no hooks.{event} array"))?;
    Ok(groups.clone())
}

/// The command `.claude/settings.json` runs for the hook of `module` in `event`, from the group
/// whose matcher is `matcher` (`None`: a group without a matcher).
fn hook_command(
    dir: &Path,
    event: &str,
    matcher: Option<&str>,
    module: &str,
) -> Result<String, Box<dyn Error>> {
    let quoted = format!("\"{module}\"");
    hook_groups(dir, event)?
        .iter()
        .filter(|group| group.get("matcher").and_then(Value::as_str) == matcher)
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .find(|command| command.contains(&quoted))
        .map(str::to_owned)
        .ok_or_else(|| format!("no {event} hook for {module} with matcher {matcher:?}").into())
}

/// Pipes `event` into a rendered hook `command` the way Claude Code runs it: from a shell, with
/// `CLAUDE_PROJECT_DIR` set and the event on stdin. `home` is the `AIRSSTACK_HOME` the hook sees.
///
/// Returns the parsed reply, or `None` when the hook printed nothing. The runtime is fail-open, so
/// a hook that cannot load its modules also exits 0 and prints nothing: a caller that expects
/// output is what notices.
fn run_hook(
    root: &Path,
    home: &Path,
    command: &str,
    event: &Value,
) -> Result<Option<Value>, Box<dyn Error>> {
    let tmpdir = root.join("tmp-run");
    fs::create_dir_all(&tmpdir)?;
    fs::create_dir_all(home)?;
    let mut child = Command::new("sh")
        .args(["-c", command])
        .current_dir(root)
        .env("CLAUDE_PROJECT_DIR", root)
        .env("TMPDIR", &tmpdir)
        .env("AIRSSTACK_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    std::io::Write::write_all(
        &mut child.stdin.take().ok_or("hook stdin was not piped")?,
        event.to_string().as_bytes(),
    )?;
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "hook failed: {stderr}");
    if stdout.trim().is_empty() {
        return Ok(None);
    }
    let reply = serde_json::from_str(&stdout).map_err(|error| {
        format!("hook stdout is not JSON ({error}): {stdout:?}; stderr: {stderr}")
    })?;
    Ok(Some(reply))
}

/// The `additionalContext` of a hook reply.
fn additional_context(reply: Option<&Value>) -> Result<&str, Box<dyn Error>> {
    reply
        .and_then(|reply| reply.pointer("/hookSpecificOutput/additionalContext"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no additionalContext in {reply:?}").into())
}

/// The `reason` of a hook reply that blocks.
fn block_reason(reply: Option<&Value>) -> Result<&str, Box<dyn Error>> {
    let reply = reply.ok_or("the hook printed nothing, expected a block")?;
    assert_eq!(
        reply.get("decision").and_then(Value::as_str),
        Some("block"),
        "{reply}"
    );
    reply
        .get("reason")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no reason in {reply}").into())
}

/// The `SubagentStop` event of `agent` whose final message names the report at `report`.
fn subagent_stop(root: &Path, agent: &str, report: &str) -> Value {
    json!({
        "hook_event_name": "SubagentStop",
        "session_id": "s1",
        "cwd": root.display().to_string(),
        "agent_type": agent,
        "last_assistant_message": format!("Report at {report}"),
    })
}

/// The `PreToolUse` event of a `Read` of the project's `src/main.rs`.
fn read_event(root: &Path) -> Value {
    json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": { "file_path": format!("{}/src/main.rs", root.display()) },
        "session_id": "s1",
        "cwd": root.display().to_string(),
    })
}

/// A `SessionStart` (source `startup`) or `UserPromptSubmit` event from the session's directory.
fn session_event(root: &Path, kind: &str) -> Value {
    let mut event = json!({
        "hook_event_name": kind,
        "session_id": "s1",
        "cwd": root.display().to_string(),
    });
    if kind == "SessionStart" {
        event["source"] = json!("startup");
    } else {
        event["prompt"] = json!("hi");
    }
    event
}

/// Every file under `dir`, recursively.
fn files_under(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            files.extend(files_under(&entry.path())?);
        } else {
            files.push(entry.path());
        }
    }
    Ok(files)
}

/// The `name` of a Markdown file's frontmatter, with its quotes removed.
fn frontmatter_name(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    lines
        .take_while(|line| line.trim_end() != "---")
        .find_map(|line| line.strip_prefix("name:"))
        .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
}

/// The rules of the `permissions:` line of an agent file, or `None` when it has no such line.
fn permission_rules(text: &str) -> Option<Result<Vec<Value>, serde_json::Error>> {
    text.lines()
        .find_map(|line| line.strip_prefix("permissions:"))
        .map(|rules| serde_json::from_str(rules.trim()))
}

#[test]
fn five_packs_install() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    init_all(project.path())?;

    let plan = run(project.path(), &["plan"])?;
    assert_eq!(
        plan.status.code(),
        Some(0),
        "plan failed: {}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let stdout = String::from_utf8(plan.stdout)?;
    let skips: Vec<&str> = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("skip "))
        .collect();
    assert_eq!(skips.len(), 1, "{stdout}");
    assert!(
        skips[0].contains("core/hook/3: claude shows no hook output to the model at session end"),
        "{stdout}"
    );

    let skills = entry_names(&project.path().join(".claude/skills"))?;
    assert_eq!(skills.len(), 28, "{skills:?}");
    let with_prefix = |prefix: &str| skills.iter().filter(|n| n.starts_with(prefix)).count();
    assert_eq!(with_prefix("core-"), 8, "{skills:?}");
    assert_eq!(with_prefix("sdlc-"), 11, "{skills:?}");
    assert_eq!(with_prefix("journal-"), 7, "{skills:?}");
    for name in [
        "core-statusline",
        "guideline-rust-guidelines",
        "guideline-python-guidelines",
    ] {
        assert!(
            skills.iter().any(|n| n == name),
            "{name} missing: {skills:?}"
        );
    }

    let agents = entry_names(&project.path().join(".claude/agents"))?;
    assert_eq!(agents.len(), 10, "{agents:?}");
    Ok(())
}

#[test]
fn five_packs_install_on_both_tools() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    init_both(project.path())?;

    let plan = run(project.path(), &["plan"])?;
    assert_eq!(
        plan.status.code(),
        Some(0),
        "plan failed: {}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let stdout = String::from_utf8(plan.stdout)?;
    let mut skips: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("skip "))
        .map(str::trim_start)
        .collect();
    let mut expected = [
        "core/hook/3: claude shows no hook output to the model at session end",
        "core/hook/1: opencode runs session.start hooks only for a new session (source \"startup\"), which this hook does not match",
        "core/hook/3: opencode has no hook when a session ends",
        "core/hook/8: opencode has no hook when a subagent stops",
        "core/hook/9: opencode has no hook when a subagent stops",
        "journal/hook/1: opencode has no hook when a subagent stops",
        "sdlc/hook/1: opencode has no hook when a subagent stops",
    ];
    skips.sort_unstable();
    expected.sort_unstable();
    assert_eq!(skips, expected, "{stdout}");
    Ok(())
}

#[test]
fn opencode_agents_reach_outside_the_project() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    init_both(project.path())?;

    let dir = project.path().join(".opencode/agents");
    let agents = entry_names(&dir)?;
    assert!(!agents.is_empty(), "no agent under {}", dir.display());
    for file in agents {
        let path = dir.join(&file);
        let text = fs::read_to_string(&path)?;
        let rules = permission_rules(&text)
            .ok_or_else(|| format!("{} has no permissions line", path.display()))??;
        assert_eq!(
            rules.first(),
            Some(&json!({"action": "*", "effect": "deny", "resource": "*"})),
            "{file}: {rules:?}"
        );
        assert_eq!(
            rules.get(1),
            Some(&json!({"action": "external_directory", "effect": "allow", "resource": "*"})),
            "{file}: {rules:?}"
        );
    }
    Ok(())
}

#[test]
fn agent_stop_matchers_are_pack_prefixed() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    init_all(project.path())?;

    let matchers: Vec<String> = hook_groups(project.path(), "SubagentStop")?
        .iter()
        .filter_map(|group| group.get("matcher").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    for expected in [
        "core-coder|core-explorer|core-reviewer",
        "sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer",
        "journal-curator",
    ] {
        assert!(
            matchers.iter().any(|m| m == expected),
            "{expected} missing: {matchers:?}"
        );
    }
    Ok(())
}

#[test]
fn read_hook_points_at_the_rust_guideline() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\n",
    )?;
    init_all(root)?;

    let command = hook_command(root, "PreToolUse", Some(READ_MATCHER), "core.hooks.enforce")?;
    let event = read_event(root);
    let reply = run_hook(root, &root.join("home"), &command, &event)?;
    let context = additional_context(reply.as_ref())?;
    assert!(context.contains("/guideline-rust-guidelines"), "{context}");
    Ok(())
}

#[test]
fn compaction_rearms_the_read_hook() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\n",
    )?;
    init_all(root)?;
    let home = root.join("home");

    let read = hook_command(root, "PreToolUse", Some(READ_MATCHER), "core.hooks.enforce")?;
    let first = run_hook(root, &home, &read, &read_event(root))?;
    additional_context(first.as_ref())?;
    let second = run_hook(root, &home, &read, &read_event(root))?;
    assert_eq!(second, None, "the pointer is sent once per context");

    let rearm = hook_command(root, "SessionStart", Some("compact"), "core.hooks.rearm")?;
    let mut compact = session_event(root, "SessionStart");
    compact["source"] = json!("compact");
    assert_eq!(run_hook(root, &home, &rearm, &compact)?, None);

    let third = run_hook(root, &home, &read, &read_event(root))?;
    let context = additional_context(third.as_ref())?;
    assert!(context.contains("/guideline-rust-guidelines"), "{context}");
    Ok(())
}

/// Writes one handoff report that breaks the protocol, one that conforms and one that is a
/// communication report breaking its protocol; returns the directory they sit in, relative to
/// `root`.
fn write_reports(root: &Path) -> Result<&'static str, Box<dyn Error>> {
    let rel = ".airsstack/handoff/s";
    fs::create_dir_all(root.join(rel))?;
    fs::write(
        root.join(rel).join("01-coder-bad.md"),
        "<summary>\nfine\n</summary>\n",
    )?;
    fs::write(
        root.join(rel).join("02-coder-good.md"),
        "---\nagent: coder\ntask: x\nsession: s\nseq: 1\n---\n<summary>\nDone.\n</summary>\n",
    )?;
    fs::write(
        root.join(rel).join("03-reviewer-badcomm.md"),
        "---\nagent: reviewer\ntask: x\nsession: s\nseq: 3\nprotocol: discuss/1\n---\n<summary>\nx\n</summary>\n<detail>\nd\n</detail>\n",
    )?;
    Ok(rel)
}

const HANDOFF_BLOCK: &str =
    "handoff report does not conform to the protocol:\nfrontmatter-missing:";
const COMM_BLOCK: &str = "communication protocol report does not conform:\ntopics-missing:";

#[test]
fn core_report_checks_run_on_a_write_and_on_a_handback() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let rel = write_reports(root)?;
    let home = root.join("home");

    for (module, report, want) in [
        (
            "core.scripts.handoff_check_hook",
            "01-coder-bad.md",
            HANDOFF_BLOCK,
        ),
        (
            "core.scripts.comm_check_hook",
            "03-reviewer-badcomm.md",
            COMM_BLOCK,
        ),
    ] {
        let write = hook_command(root, "PostToolUse", Some("Write"), module)?;
        let event = json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s1",
            "cwd": root.display().to_string(),
            "tool_name": "Write",
            "tool_input": { "file_path": root.join(rel).join(report).display().to_string() },
        });
        let reply = run_hook(root, &home, &write, &event)?;
        let reason = block_reason(reply.as_ref())?;
        assert!(reason.starts_with(want), "{module} on a write: {reason}");

        let handback = hook_command(root, "PreToolUse", Some("^SubagentHandback$"), module)?;
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "s1",
            "cwd": root.display().to_string(),
            "tool_name": "SubagentHandback",
            "tool_input": { "message": format!("Report at {rel}/{report}") },
        });
        let reply = run_hook(root, &home, &handback, &event)?;
        let reply = reply.ok_or("the hook printed nothing, expected a deny")?;
        let output = reply
            .get("hookSpecificOutput")
            .ok_or_else(|| format!("no hookSpecificOutput in {reply}"))?;
        assert_eq!(
            output.get("permissionDecision").and_then(Value::as_str),
            Some("deny"),
            "{module} on a handback: {reply}"
        );
        let reason = output
            .get("permissionDecisionReason")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("no permissionDecisionReason in {reply}"))?;
        assert!(reason.starts_with(want), "{module} on a handback: {reason}");
    }
    Ok(())
}

#[test]
fn core_report_checks_run_at_agent_stop() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let rel = write_reports(root)?;
    let home = root.join("home");

    let handoff = hook_command(
        root,
        "SubagentStop",
        Some("core-coder|core-explorer|core-reviewer"),
        "core.scripts.handoff_check_hook",
    )?;
    let bad = run_hook(
        root,
        &home,
        &handoff,
        &subagent_stop(root, "core-coder", &format!("{rel}/01-coder-bad.md")),
    )?;
    let reason = block_reason(bad.as_ref())?;
    assert!(reason.starts_with(HANDOFF_BLOCK), "{reason}");
    let good = run_hook(
        root,
        &home,
        &handoff,
        &subagent_stop(root, "core-coder", &format!("{rel}/02-coder-good.md")),
    )?;
    assert_eq!(good, None);

    let comm = hook_command(root, "SubagentStop", None, "core.scripts.comm_check_hook")?;
    let bad = run_hook(
        root,
        &home,
        &comm,
        &subagent_stop(
            root,
            "core-reviewer",
            &format!("{rel}/03-reviewer-badcomm.md"),
        ),
    )?;
    let reason = block_reason(bad.as_ref())?;
    assert!(reason.starts_with(COMM_BLOCK), "{reason}");
    Ok(())
}

#[test]
fn core_style_hook_adds_context_at_session_start_and_on_a_prompt() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let home = root.join("home");

    for event in ["SessionStart", "UserPromptSubmit"] {
        let command = hook_command(root, event, None, "core.scripts.style")?;
        let reply = run_hook(root, &home, &command, &session_event(root, event))?;
        let context = additional_context(reply.as_ref())?;
        assert!(!context.trim().is_empty(), "{event}: empty context");
    }
    Ok(())
}

#[test]
fn sdlc_attention_hook_reports_the_board() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let chain = root.join(".claudestacks/sdlc/2026-01-01-x");
    fs::create_dir_all(&chain)?;
    fs::write(
        chain.join("intent.md"),
        "---\nstatus: approved\ncreated: 2026-01-01\n---\n# Intent: x\n",
    )?;

    let command = hook_command(root, "SessionStart", None, "sdlc.scripts.attention")?;
    let reply = run_hook(
        root,
        &root.join("home"),
        &command,
        &session_event(root, "SessionStart"),
    )?;
    let context = additional_context(reply.as_ref())?;
    assert!(context.contains("1 open, 0 need attention"), "{context}");
    Ok(())
}

#[test]
fn journal_session_start_hook_renders_the_card() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("the project directory has no UTF-8 name")?;
    let command = hook_command(
        root,
        "SessionStart",
        Some("startup|resume|clear"),
        "journal.scripts.session-start",
    )?;
    let event = session_event(root, "SessionStart");

    let empty = run_hook(root, &root.join("home-empty"), &command, &event)?;
    assert_eq!(empty, None, "an empty vault renders no card");

    let home = root.join("home-notes");
    fs::create_dir_all(home.join("journal/notes"))?;
    fs::write(
        home.join("journal/notes/sample.md"),
        format!(
            "---\ntype: insight\ntitle: Sample\nsummary: a sample note for the card\nproject: {name}\nupdated: 2026-11-24\n---\nBody.\n"
        ),
    )?;
    let reply = run_hook(root, &home, &command, &event)?;
    let context = additional_context(reply.as_ref())?;
    assert!(
        context.starts_with(&format!("## Journal \u{2014} recent activity ({name})")),
        "{context}"
    );
    Ok(())
}

/// Each tool's `journal-capture/session.md` lands in that tool's skill directory, with `{{root}}`
/// and `{{var tmpdir}}` rendered and nothing else changed, and the rendered capture and note
/// skills read it from that same path.
#[test]
fn journal_session_lookup_is_per_tool() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_both(root)?;
    let source = |tool: &str| {
        fs::read_to_string(root.join(format!(
            "packs/journal/native/{tool}/skills/journal-capture/session.md"
        )))
    };
    let claude = source("claude")?.replace("{{root}}", ".claude/agentstacks/journal");
    let opencode = source("opencode")?.replace("{{var tmpdir}}", "${TMPDIR:-/tmp}");
    assert_eq!(
        fs::read_to_string(root.join(".claude/skills/journal-capture/session.md"))?,
        claude
    );
    assert_eq!(
        fs::read_to_string(root.join(".opencode/skills/journal-capture/session.md"))?,
        opencode
    );
    for tool in ["claude", "opencode"] {
        let session = format!(".{tool}/skills/journal-capture/session.md");
        for skill in ["capture", "note"] {
            let path = root.join(format!(".{tool}/skills/journal-{skill}/SKILL.md"));
            let text = fs::read_to_string(&path)?;
            assert!(
                text.contains(&session),
                "{} does not read {session}",
                path.display()
            );
        }
    }
    Ok(())
}

/// The names pack text gate 4 keeps out of neutral pack text, read from its pattern in the
/// `[tasks.lua]` table of `Makefile.toml`, so the rendered output is checked against the same list.
fn gate4_names() -> Result<Vec<String>, Box<dyn Error>> {
    let makefile =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Makefile.toml"))?;
    let lua = make_task(&makefile, "lua")?;
    let gate = &lua[lua
        .find("# Gate 4")
        .ok_or("[tasks.lua] has no `# Gate 4` block")?..];
    let pattern = gate
        .split_once("grep -rnE '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(pattern, _)| pattern)
        .ok_or("gate 4 has no `grep -rnE '<pattern>'`")?;
    Ok(literal_alternatives(pattern))
}

/// Every literal string an extended regex of plain alternatives matches: `a|b` gives `a` and `b`,
/// one group `x(a|b)y` gives `xay` and `xby`, and `\.` stands for `.`.
fn literal_alternatives(pattern: &str) -> Vec<String> {
    let mut alternatives = Vec::new();
    let mut depth = 0_usize;
    let mut from = 0;
    for (at, character) in pattern.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                alternatives.push(&pattern[from..at]);
                from = at + 1;
            }
            _ => {}
        }
    }
    alternatives.push(&pattern[from..]);

    let mut names = Vec::new();
    for alternative in alternatives {
        let alternative = alternative.replace("\\.", ".");
        match (alternative.find('('), alternative.rfind(')')) {
            (Some(open), Some(close)) if open < close => {
                let (head, tail) = (&alternative[..open], &alternative[close + 1..]);
                for inner in alternative[open + 1..close].split('|') {
                    names.push(format!("{head}{inner}{tail}"));
                }
            }
            _ => names.push(alternative),
        }
    }
    names
}

/// No skill, agent or command rendered under `.opencode/` names a Claude Code tool, hook event or
/// store path from pack text gate 4. `doc-comment-discipline.md` is skipped, as the gate skips it:
/// it lists such names as words to keep out of source comments.
#[test]
fn opencode_instructions_name_no_claude_tools() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_both(root)?;
    let names = gate4_names()?;
    for known in ["subagent_type", "`Agent`", ".claude/"] {
        assert!(
            names.iter().any(|name| name == known),
            "gate 4's names lack {known}: {names:?}"
        );
    }

    let mut hits = Vec::new();
    for kind in ["skills", "agents", "commands"] {
        for file in files_under(&root.join(".opencode").join(kind))? {
            if file
                .file_name()
                .is_some_and(|name| name == "doc-comment-discipline.md")
            {
                continue;
            }
            let text = String::from_utf8_lossy(&fs::read(&file)?).into_owned();
            for (number, line) in text.lines().enumerate() {
                for name in &names {
                    if line.contains(name.as_str()) {
                        hits.push(format!("{}:{}: {name}", file.display(), number + 1));
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "{}", hits.join("\n"));
    Ok(())
}

#[test]
fn rendered_files_keep_no_pack_syntax_and_names_match_their_paths() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    let root = project.path();
    init_all(root)?;
    let claude = root.join(".claude");

    for file in files_under(&claude)? {
        let text = String::from_utf8_lossy(&fs::read(&file)?).into_owned();
        for leftover in ["{{", "CLAUDE_PLUGIN_ROOT"] {
            assert!(
                !text.contains(leftover),
                "{} holds {leftover}",
                file.display()
            );
        }
    }

    for dir in entry_names(&claude.join("skills"))? {
        let skill = claude.join("skills").join(&dir).join("SKILL.md");
        let name = frontmatter_name(&fs::read_to_string(&skill)?);
        assert_eq!(name.as_deref(), Some(dir.as_str()), "{}", skill.display());
    }
    for file in entry_names(&claude.join("agents"))? {
        let agent = claude.join("agents").join(&file);
        let stem = file
            .strip_suffix(".md")
            .ok_or("an agent file is not Markdown")?;
        let name = frontmatter_name(&fs::read_to_string(&agent)?);
        assert_eq!(name.as_deref(), Some(stem), "{}", agent.display());
    }
    Ok(())
}

/// The text of the `[tasks.<task>]` table of `Makefile.toml`, up to the next task table.
fn make_task<'a>(makefile: &'a str, task: &str) -> Result<&'a str, Box<dyn Error>> {
    let header = format!("[tasks.{task}]");
    let start = makefile
        .find(&header)
        .ok_or_else(|| format!("Makefile.toml has no {header}"))?;
    let rest = &makefile[start + header.len()..];
    let end = rest.find("\n[tasks.").unwrap_or(rest.len());
    Ok(&rest[..end])
}

#[test]
fn the_lua_gate_tests_the_packs_and_is_part_of_dod() -> TestResult {
    let makefile =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Makefile.toml"))?;

    let lua = make_task(&makefile, "lua")?;
    let runs_over_packs = |command: &str| {
        lua.lines()
            .map(str::trim)
            .any(|line| line.starts_with(command) && line.ends_with(" packs"))
    };
    assert!(
        runs_over_packs("airsl check"),
        "[tasks.lua] does not run `airsl check` over packs"
    );
    assert!(
        runs_over_packs("airsl test"),
        "[tasks.lua] does not run `airsl test` over packs"
    );
    for gate in 1..=4 {
        let message = format!("pack text gate {gate}");
        assert!(lua.contains(&message), "[tasks.lua] has no `{message}`");
    }

    let dod = make_task(&makefile, "dod")?;
    let dependencies = dod
        .lines()
        .find(|line| line.trim_start().starts_with("dependencies"))
        .ok_or("[tasks.dod] has no dependencies")?;
    assert!(dependencies.contains("\"lua\""), "{dependencies}");
    Ok(())
}

/// The major version of the `version` the `table` table of the manifest at `path` declares.
fn declared_major(path: &Path, table: &str) -> Result<u64, Box<dyn Error>> {
    let manifest: toml::Table = fs::read_to_string(path)?.parse()?;
    let version = manifest
        .get(table)
        .and_then(toml::Value::as_table)
        .and_then(|found| found.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{}: no [{table}] version", path.display()))?;
    let major = version.split('.').next().unwrap_or_default();
    major.parse().map_err(|_| {
        format!(
            "{}: version \"{version}\" has no major number",
            path.display()
        )
        .into()
    })
}

/// The directories directly under `dir` of the repository root, sorted.
fn repo_dirs(dir: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(dir);
    let mut found = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    found.retain(|path| path.is_dir());
    found.sort();
    Ok(found)
}

#[test]
fn every_official_pack_and_extension_is_at_version_one_or_later() -> TestResult {
    let manifests = [
        ("packs", "pack.toml", "pack"),
        ("extensions", "extension.toml", "extension"),
    ];
    let mut checked = 0;
    for (dir, file, table) in manifests {
        for path in repo_dirs(dir)? {
            let manifest = path.join(file);
            let major = declared_major(&manifest, table)?;
            assert!(major >= 1, "{} is below 1.0.0", manifest.display());
            checked += 1;
        }
    }
    assert_eq!(checked, 8, "five packs and three extensions");
    Ok(())
}

mod tests {
    use super::{Version, frontmatter_name, literal_alternatives, parse_version};

    const fn version(core: (u32, u32, u32), pre_release: bool) -> Version {
        Version { core, pre_release }
    }

    #[test]
    fn parses_the_version_line() {
        assert_eq!(
            parse_version("airsl 0.1.5\n"),
            Some(version((0, 1, 5), false))
        );
        assert_eq!(
            parse_version("airsl 0.2.0"),
            Some(version((0, 2, 0), false))
        );
        assert_eq!(
            parse_version("airsl 0.1.5-rc1"),
            Some(version((0, 1, 5), true))
        );
        assert_eq!(parse_version("airsl dev"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn orders_versions_as_semver_does() {
        let min = (0, 1, 5);
        let at_least = |text: &str| parse_version(text).is_some_and(|v| v.at_least(min));
        assert!(at_least("airsl 0.1.5"));
        assert!(at_least("airsl 0.2.0"));
        assert!(at_least("airsl 0.1.6-rc1"));
        assert!(!at_least("airsl 0.1.5-rc1"));
        assert!(!at_least("airsl 0.1.4"));
    }

    #[test]
    fn reads_the_frontmatter_name() {
        assert_eq!(
            frontmatter_name("---\nname: core-x\n---\nbody").as_deref(),
            Some("core-x")
        );
        assert_eq!(
            frontmatter_name("---\ndescription: d\nname: \"core-y\"\n---\n").as_deref(),
            Some("core-y")
        );
        assert_eq!(frontmatter_name("---\ndescription: d\n---\nname: z"), None);
        assert_eq!(frontmatter_name("no frontmatter"), None);
    }

    #[test]
    fn expands_the_gate_pattern_into_literal_names() {
        assert_eq!(
            literal_alternatives("a b|`(Read|Task)`|\\.claude/|x(1|2)y"),
            ["a b", "`Read`", "`Task`", ".claude/", "x1y", "x2y"]
        );
        assert_eq!(literal_alternatives("single"), ["single"]);
    }
}
