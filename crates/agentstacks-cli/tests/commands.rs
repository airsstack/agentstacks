//! End-to-end runs of the `agentstacks` binary on a temporary project.
//!
//! Each test copies `tests/fixtures/project` and the shipped `extensions/claude` into a temporary
//! directory (the legacy tests lay the `0.x` project files of `tests/fixtures/legacy` over it) and
//! runs the real binary there with `-C`, stdin null. Nothing here proves a rule on
//! its own: the rules are proven in the library, and these runs show that the commands, the claude
//! extension and the airsl adapter hold together.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The variable that turns a missing `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// The skill the demo pack writes into the project.
const SKILL: &str = ".claude/skills/demo-greet/SKILL.md";

/// What every run that applies nothing without a terminal prints on stderr.
const NO_TERMINAL: &str = "stdin is not a terminal; pass --yes to apply";

/// Whether `airsl` can run on this machine.
fn airsl_runs() -> bool {
    Command::new("airsl")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Whether the test must stop: the demo pack holds a lua hook, so the plan checks `airsl`. When it
/// cannot run the test says so, unless `AGENTSTACKS_REQUIRE_AIRSL` is set: then it fails, so a
/// missing binary cannot pass the suite unseen.
fn skip_without_airsl() -> Result<bool, Box<dyn Error>> {
    if airsl_runs() {
        return Ok(false);
    }
    if std::env::var_os(REQUIRE_AIRSL).is_some() {
        return Err(format!("airsl cannot run on this machine and {REQUIRE_AIRSL} is set").into());
    }
    eprintln!("skipped: airsl cannot run on this machine");
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

/// A temporary project holding the fixture packs and a copy of the claude extension.
struct Project {
    dir: TempDir,
}

impl Project {
    fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        copy_tree(&manifest.join("tests/fixtures/project"), dir.path())?;
        copy_tree(
            &manifest.join("../../extensions/claude"),
            &dir.path().join("ext/claude"),
        )?;
        Ok(Self { dir })
    }

    /// [`Project::new`] with the `agentstacks.toml` and `agentstacks.lock` of a project written for
    /// the `0.x` releases, both at `api = 0`, laid over it.
    fn legacy() -> Result<Self, Box<dyn Error>> {
        let project = Self::new()?;
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        copy_tree(&manifest.join("tests/fixtures/legacy"), project.dir.path())?;
        Ok(project)
    }

    /// A temporary directory with no fixture: nothing but what a test adds.
    fn empty() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            dir: tempfile::tempdir()?,
        })
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Runs the binary with `-C <project>` and stdin null.
    fn run(&self, args: &[&str]) -> Result<Run, Box<dyn Error>> {
        let output = Command::new(env!("CARGO_BIN_EXE_agentstacks"))
            .arg("-C")
            .arg(self.dir.path())
            .args(args)
            .stdin(Stdio::null())
            .output()?;
        Run::new(&output)
    }

    /// `init --tool ./ext/claude --pack ./packs/demo --yes`, which must succeed.
    fn init(&self) -> TestResult {
        let run = self.run(&[
            "init",
            "--tool",
            "./ext/claude",
            "--pack",
            "./packs/demo",
            "--yes",
        ])?;
        assert_eq!(run.code, 0, "init failed: {}", run.stderr);
        Ok(())
    }
}

/// One finished run: the exit code and both streams as text.
struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    fn new(output: &Output) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            code: output
                .status
                .code()
                .ok_or("the binary was killed by a signal")?,
            stdout: String::from_utf8(output.stdout.clone())?,
            stderr: String::from_utf8(output.stderr.clone())?,
        })
    }
}

#[test]
fn init_with_yes_writes_toml_and_applies() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    let run = project.run(&[
        "init",
        "--tool",
        "./ext/claude",
        "--pack",
        "./packs/demo",
        "--yes",
    ])?;

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let toml = fs::read_to_string(project.path("agentstacks.toml"))?;
    assert!(toml.contains("path = \"./ext/claude\""), "{toml}");
    assert!(toml.contains("path = \"./packs/demo\""), "{toml}");
    for written in [SKILL, ".claude/settings.json", "agentstacks.lock"] {
        assert!(project.path(written).is_file(), "{written} was not written");
    }
    let last = run.stdout.lines().last().ok_or("stdout is empty")?;
    assert!(last.starts_with("applied: "), "{last}");
    Ok(())
}

#[test]
fn init_without_terminal_keeps_the_toml_and_is_refused() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    let run = project.run(&["init", "--tool", "./ext/claude", "--pack", "./packs/demo"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert_eq!(
        run.stderr,
        format!("{NO_TERMINAL}\nagentstacks.toml written; nothing applied (run apply)\n")
    );
    assert!(project.path("agentstacks.toml").is_file());
    assert!(!project.path(SKILL).exists());
    Ok(())
}

#[test]
fn init_refuses_an_existing_toml() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    let run = project.run(&["init", "--tool", "./ext/claude", "--pack", "./packs/demo"])?;

    assert_eq!(run.code, 1);
    assert_eq!(
        run.stderr,
        "agentstacks.toml: already exists (edit it and run apply)\n"
    );
    Ok(())
}

#[test]
fn plan_after_apply_reports_nothing_to_write() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    let run = project.run(&["plan"])?;

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let summary = run.stdout.lines().last().ok_or("stdout is empty")?;
    assert!(summary.starts_with("0 to write"), "{}", run.stdout);
    Ok(())
}

#[test]
fn ext_add_a_path_extension() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    copy_tree(
        &manifest.join("../../extensions/opencode"),
        &project.path("ext/opencode"),
    )?;
    let run = project.run(&["ext", "add", "./ext/opencode", "--yes"])?;

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    assert!(
        run.stdout
            .lines()
            .any(|line| line == "agentstacks.toml: added tools.opencode"),
        "{}",
        run.stdout
    );
    let toml = fs::read_to_string(project.path("agentstacks.toml"))?;
    assert!(toml.contains("[tools.opencode]"), "{toml}");
    let planned = project.run(&["plan"])?;
    assert_eq!(planned.code, 0, "stderr: {}", planned.stderr);
    let summary = planned.stdout.lines().last().ok_or("stdout is empty")?;
    assert!(summary.starts_with("0 to write"), "{}", planned.stdout);
    Ok(())
}

/// The shipped extension `name` of this repository.
fn shipped_extension(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../extensions")
        .join(name)
}

/// Every file under `dir` with its bytes, keyed by its path relative to `dir`.
fn snapshot(dir: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                out.insert(path.strip_prefix(root)?.to_path_buf(), fs::read(&path)?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out)?;
    Ok(out)
}

#[test]
fn ext_test_claude_passes() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::empty()?;
    let extension = shipped_extension("claude");
    let run = project.run(&["ext", "test", &extension.display().to_string()])?;

    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout.lines().last(), Some("passed"), "{}", run.stdout);
    assert!(run.stdout.starts_with("ext test claude "), "{}", run.stdout);
    assert!(run.stdout.contains("\nlua tests: "), "{}", run.stdout);
    assert!(
        run.stdout.contains("\nsecond plan: nothing to change\n"),
        "{}",
        run.stdout
    );
    assert_eq!(run.stderr, "");
    Ok(())
}

#[test]
fn ext_test_opencode_passes() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::empty()?;
    let extension = shipped_extension("opencode");
    let run = project.run(&["ext", "test", &extension.display().to_string()])?;

    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout.lines().last(), Some("passed"), "{}", run.stdout);
    let skips: Vec<&str> = run
        .stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("skip "))
        .collect();
    assert_eq!(skips.len(), 2, "{}", run.stdout);
    assert!(
        skips
            .iter()
            .any(|line| line
                .contains("conformance/hook/2: opencode has no hook when a session ends")),
        "{skips:?}"
    );
    assert!(
        skips
            .iter()
            .any(|line| line
                .contains("conformance/hook/6: opencode has no hook when a subagent stops")),
        "{skips:?}"
    );
    Ok(())
}

#[test]
fn ext_test_writes_nothing_outside_scratch() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::empty()?;
    let extension = project.path("ext/claude");
    copy_tree(&shipped_extension("claude"), &extension)?;
    let before = snapshot(&extension)?;
    let run = project.run(&["ext", "test", "ext/claude"])?;

    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert_eq!(snapshot(&extension)?, before);
    let top: Vec<_> = fs::read_dir(project.path("."))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<_, _>>()?;
    assert_eq!(top, ["ext"]);
    Ok(())
}

/// The text of the operating system's not-found error is the one Unix writes.
#[cfg(unix)]
#[test]
fn ext_test_labels_paths_as_written() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::empty()?;
    let run = project.run(&["ext", "test", "ext/missing"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "ext/missing: No such file or directory (os error 2)\n"
    );
    Ok(())
}

/// The text of the operating system's not-found error is the one Unix writes.
#[cfg(unix)]
#[test]
fn ext_add_without_project() -> TestResult {
    let project = Project::empty()?;
    let run = project.run(&["ext", "add", "./x", "--yes"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert_eq!(
        run.stderr,
        "agentstacks.toml: No such file or directory (os error 2)\n"
    );
    Ok(())
}

#[test]
fn apply_without_terminal_or_yes_refuses() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    fs::remove_file(project.path(SKILL))?;
    let lock = fs::read(project.path("agentstacks.lock"))?;
    let run = project.run(&["apply"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert_eq!(run.stderr, format!("{NO_TERMINAL}\n"));
    assert!(!project.path(SKILL).exists(), "apply wrote without consent");
    assert_eq!(
        fs::read(project.path("agentstacks.lock"))?,
        lock,
        "apply rewrote the lock"
    );
    Ok(())
}

#[test]
fn apply_yes_with_nothing_pending_prints_nothing_to_change() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    let run = project.run(&["apply", "--yes"])?;

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    assert!(run.stdout.contains("nothing to change"), "{}", run.stdout);
    Ok(())
}

#[test]
fn doctor_is_healthy_then_reports_a_change() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    let healthy = project.run(&["doctor"])?;
    assert_eq!(healthy.code, 0, "stdout: {}", healthy.stdout);
    assert_eq!(healthy.stdout.trim_end(), "healthy");

    fs::write(project.path(SKILL), "edited by hand\n")?;
    let changed = project.run(&["doctor"])?;
    assert_eq!(changed.code, 1);
    assert!(
        changed
            .stdout
            .lines()
            .any(|line| line == format!("changed: {SKILL}")),
        "{}",
        changed.stdout
    );
    Ok(())
}

#[test]
fn init_with_unknown_name_is_refused() -> TestResult {
    let project = Project::empty()?;
    let run = project.run(&["init", "--tool", "./ext/claude", "--pack", "nope"])?;

    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("--pack \"nope\": not an official pack; pass a path, or add a git source to agentstacks.toml\n"),
        "{}",
        run.stderr
    );
    assert!(!project.path("agentstacks.toml").exists());
    Ok(())
}

#[test]
fn usage_error_exits_two() -> TestResult {
    let project = Project::empty()?;
    let run = project.run(&["init", "--tool", "claude"])?;

    assert_eq!(run.code, 2, "stderr: {}", run.stderr);
    Ok(())
}

#[test]
fn update_moves_a_legacy_project() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::legacy()?;
    let run = project.run(&["update", "--yes"])?;

    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    assert_eq!(
        run.stdout.lines().next(),
        Some("  api: 0 \u{2192} 1"),
        "{}",
        run.stdout
    );
    let toml = fs::read_to_string(project.path("agentstacks.toml"))?;
    assert!(toml.starts_with("api = 1\n"), "{toml}");
    let lock = fs::read_to_string(project.path("agentstacks.lock"))?;
    assert!(lock.starts_with("api = 1\n"), "{lock}");
    assert!(project.path(SKILL).is_file());
    Ok(())
}

#[test]
fn plan_refuses_the_legacy_fixture_before_update() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::legacy()?;
    let run = project.run(&["plan"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert!(
        run.stderr
            .contains("agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)"),
        "{}",
        run.stderr
    );
    Ok(())
}

#[test]
fn path_pack_refusal_names_the_path_as_written() -> TestResult {
    if skip_without_airsl()? {
        return Ok(());
    }
    let project = Project::new()?;
    project.init()?;
    fs::create_dir_all(project.path("packs/bad"))?;
    fs::write(project.path("packs/bad/pack.toml"), "this is not a pack\n")?;
    let toml_path = project.path("agentstacks.toml");
    let mut toml = fs::read_to_string(&toml_path)?;
    toml.push_str("\n[packs.bad]\npath = \"packs/bad\"\n");
    fs::write(&toml_path, toml)?;
    let run = project.run(&["plan"])?;

    assert_eq!(run.code, 1, "stdout: {}", run.stdout);
    assert!(
        run.stderr.starts_with("packs/bad/pack.toml:"),
        "{}",
        run.stderr
    );
    let root = project.dir.path().to_string_lossy();
    assert!(!run.stderr.contains(root.as_ref()), "{}", run.stderr);
    Ok(())
}
