//! The five official packs installed together through the real `agentstacks` binary.
//!
//! Each test copies the repository's `packs/` and `extensions/claude` into a temporary git
//! project, runs `init` for all five packs and checks the result. Nothing here proves a rule on its
//! own: the rules are proven in the library and the Lua tests, and these runs show that the packs
//! install side by side and that a rendered hook runs.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The variable that turns a missing or too old `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// The oldest `airsl` the packs run on.
const MIN_AIRSL: (u32, u32, u32) = (0, 1, 5);

/// The matcher of the hook that points a read of a source file at its guideline.
const READ_MATCHER: &str = "Read|Edit|Write";

/// Whether `airsl --version` reports at least [`MIN_AIRSL`].
fn airsl_at_least_0_1_5() -> bool {
    let Ok(output) = Command::new("airsl").arg("--version").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(text) = String::from_utf8(output.stdout) else {
        return false;
    };
    parse_version(&text).is_some_and(|version| version >= MIN_AIRSL)
}

/// The `major.minor.patch` in the last word of a `airsl 0.1.5` style line.
fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split_whitespace().last()?.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Whether the test must stop: the packs hold lua hooks and need `airsl` 0.1.5 or later. When it is
/// missing or older the test says so, unless `AGENTSTACKS_REQUIRE_AIRSL` is set: then it fails, so
/// a missing binary cannot pass the suite unseen.
fn skip_without_airsl() -> Result<bool, Box<dyn Error>> {
    if airsl_at_least_0_1_5() {
        return Ok(false);
    }
    if std::env::var_os(REQUIRE_AIRSL).is_some() {
        return Err(format!(
            "airsl 0.1.5 or later cannot run on this machine and {REQUIRE_AIRSL} is set"
        )
        .into());
    }
    eprintln!("skipped: airsl 0.1.5 or later cannot run on this machine");
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

/// A temporary git project holding a copy of the official packs and the claude extension.
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

/// `init` for the claude extension and all five packs, which must succeed.
fn init_all(dir: &Path) -> TestResult {
    let output = run(
        dir,
        &[
            "init",
            "--tool",
            "./ext/claude",
            "--pack",
            "./packs/core",
            "--pack",
            "./packs/sdlc",
            "--pack",
            "./packs/journal",
            "--pack",
            "./packs/guideline-rust",
            "--pack",
            "./packs/guideline-python",
            "--yes",
        ],
    )?;
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

#[test]
fn five_packs_install() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = project()?;
    init_all(project.path())?;

    let plan = run(project.path(), &["plan"])?;
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

    let command = hook_groups(root, "PreToolUse")?
        .iter()
        .find(|group| group.get("matcher").and_then(Value::as_str) == Some(READ_MATCHER))
        .and_then(|group| group.pointer("/hooks/0/command"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no PreToolUse group with matcher {READ_MATCHER}"))?
        .to_owned();

    let tmpdir = root.join("tmp-run");
    fs::create_dir_all(&tmpdir)?;
    let project_dir = root.to_string_lossy();
    let event = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{{"file_path":"{project_dir}/src/main.rs"}},"session_id":"s1","cwd":"{project_dir}"}}"#
    );
    let mut child = Command::new("sh")
        .args(["-c", &command])
        .current_dir(root)
        .env("CLAUDE_PROJECT_DIR", root)
        .env("TMPDIR", &tmpdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    std::io::Write::write_all(
        &mut child.stdin.take().ok_or("hook stdin was not piped")?,
        event.as_bytes(),
    )?;
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        output.status.success(),
        "hook failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let reply: Value = serde_json::from_str(&stdout)
        .map_err(|error| format!("hook stdout is not JSON ({error}): {stdout:?}"))?;
    let context = reply
        .pointer("/hookSpecificOutput/additionalContext")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("no additionalContext in {stdout}"))?;
    assert!(context.contains("/guideline-rust-guidelines"), "{context}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn parses_the_version_line() {
        assert_eq!(parse_version("airsl 0.1.5\n"), Some((0, 1, 5)));
        assert_eq!(parse_version("airsl 0.2.0"), Some((0, 2, 0)));
        assert!(parse_version("airsl 0.1.4").is_some_and(|v| v < (0, 1, 5)));
        assert_eq!(parse_version("airsl dev"), None);
        assert_eq!(parse_version(""), None);
    }
}
