//! The flow that reports drift between the project disk and `agentstacks.lock`.
//!
//! It exists so a user can learn, without writing anything, whether the files an earlier apply
//! wrote are still as the lock records them and whether the airsl binary is the pinned one. It
//! needs no network, no git and no extension.
//!
//! # Responsibilities
//!
//! - [`doctor`] reads the lock text, parses it, asks the workspace where each locked path leads,
//!   asks the binary port for the airsl version when the project needs airsl, and hands each answer
//!   to the drift rules in `types`. It orders the calls and collects the findings: the lock's own
//!   findings plus the airsl finding when the lock is missing, unreadable or refused, otherwise one
//!   finding at most per locked file in the lock's path order, then the airsl finding.
//!
//! # Non-responsibilities
//!
//! No rule: whether a file counts as changed, and the words of every finding, belong to the drift
//! rules and to [`Finding`]. No network, no extension and no write. Showing the findings and the
//! exit code belong to the command.

use std::path::Path;

use crate::port::{Binary, Workspace};
use crate::types::lock::{FileEntry, Lockfile};
use crate::types::op::OpPath;
use crate::types::plan::{Finding, airsl_finding, file_finding};

/// The lock's name in the project, the file its refusals are located at.
const LOCK_FILE: &str = "agentstacks.lock";

/// Every way the project has drifted from `agentstacks.lock`; an empty list means healthy.
///
/// When the lock is missing, cannot be read or is refused, the result holds the findings about
/// the lock and the airsl binary, because there is nothing else to check against. Otherwise it
/// holds the findings about the locked files in the lock's path order, every file checked, and
/// last the finding about the airsl binary when the project needs it: the lock pins airsl, or a
/// locked file is a hook runtime (`runtime.lua`), whose hooks run on airsl. A project with
/// neither does not need airsl, so its absence is healthy there.
#[must_use]
pub fn doctor<W: Workspace, B: Binary>(workspace: &W, binary: &B) -> Vec<Finding> {
    let text = match workspace.lock_text() {
        Ok(Some(text)) => text,
        Ok(None) => {
            let mut findings = vec![Finding::LockMissing];
            findings.extend(airsl_finding(None, binary.airsl_version()));
            return findings;
        }
        Err(error) => {
            let mut findings = vec![Finding::LockIo(error)];
            findings.extend(airsl_finding(None, binary.airsl_version()));
            return findings;
        }
    };
    let lock = match Lockfile::parse(Path::new(LOCK_FILE), &text) {
        Ok(lock) => lock,
        Err(refusals) => {
            let mut findings: Vec<Finding> = refusals
                .into_sorted()
                .into_iter()
                .map(Finding::Lock)
                .collect();
            findings.extend(airsl_finding(None, binary.airsl_version()));
            return findings;
        }
    };
    let mut findings: Vec<Finding> = lock
        .files()
        .iter()
        .filter_map(|entry| file_check(workspace, entry))
        .collect();
    if lock.airsl().is_some() || has_hook_runtime(&lock) {
        findings.extend(airsl_finding(lock.airsl(), binary.airsl_version()));
    }
    findings
}

/// Whether a locked file is the hook runtime an extension's Lua hooks run on.
fn has_hook_runtime(lock: &Lockfile) -> bool {
    lock.files().iter().any(|entry| {
        let path = entry.path().as_str();
        path == "runtime.lua" || path.ends_with("/runtime.lua")
    })
}

/// The finding for one locked file, asking `workspace` where its path leads.
fn file_check<W: Workspace>(workspace: &W, entry: &FileEntry) -> Option<Finding> {
    // The lock reader refuses every locked path shape but one that normalises to nothing, such as
    // `.`: that names the project directory itself, which is not a regular file.
    let Ok(path) = OpPath::new(entry.path().as_str()) else {
        return Some(Finding::NotRegular(entry.path().clone()));
    };
    match workspace.resolve(&path) {
        Ok(resolution) => file_finding(entry, &resolution),
        Err(error) => Some(Finding::Io {
            path: entry.path().clone(),
            error,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{BinaryCall, FakeBinary, FakeWorkspace, WorkspaceCall};
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const LOCKED: &[u8] = b"locked";
    const X: &str = ".claude/x";

    fn path(text: &str) -> Result<OpPath, Box<dyn std::error::Error>> {
        Ok(OpPath::new(text)?)
    }

    /// A lock holding one pack, the `[airsl]` pin when given, and one `[[file]]` per entry, each
    /// recording the hash of the bytes beside its path.
    fn lock_text(airsl: Option<&str>, files: &[(&str, &[u8])]) -> String {
        let airsl_table = airsl.map(|version| format!("[airsl]\nversion = \"{version}\"\n"));
        let pack = format!(
            "[[pack]]\nname = \"core\"\nversion = \"0.3.0\"\npath = \"packs/core\"\n\
             tree = \"sha256:{HEX64}\"\n"
        );
        let file_tables = files.iter().map(|(file, bytes)| {
            format!(
                "[[file]]\npath = \"{file}\"\nhash = \"{}\"\nby = \"core\"\n",
                ContentHash::of_bytes(bytes)
            )
        });
        std::iter::once("api = 1\n".to_owned())
            .chain(airsl_table)
            .chain([pack])
            .chain(file_tables)
            .collect()
    }

    fn shown(findings: &[Finding]) -> Vec<String> {
        findings.iter().map(ToString::to_string).collect()
    }

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    fn run(workspace: &FakeWorkspace, binary: &FakeBinary) -> Vec<String> {
        shown(&doctor(workspace, binary))
    }

    fn locked_x(workspace: FakeWorkspace) -> FakeWorkspace {
        workspace.lock(&lock_text(None, &[(X, LOCKED)]))
    }

    #[test]
    fn doctor_reports_a_missing_lock_alone() {
        let binary = FakeBinary::new().airsl("airsl 0.1.5");
        let found = run(&FakeWorkspace::new(), &binary);
        assert_eq!(found, ["agentstacks.lock: not found (run apply)"]);
        assert_eq!(binary.asked(), [BinaryCall::AirslVersion]);
    }

    #[test]
    fn doctor_reports_airsl_with_missing_lock() {
        let found = run(&FakeWorkspace::new(), &FakeBinary::new());
        assert_eq!(
            found,
            [
                "agentstacks.lock: not found (run apply)",
                "airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            ]
        );
    }

    #[test]
    fn doctor_reports_airsl_with_refused_lock() {
        let workspace = FakeWorkspace::new().lock("api = 1\n[[merge]]\nx = 1\n");
        let found = run(&workspace, &FakeBinary::new());
        assert_eq!(
            found.last().map(String::as_str),
            Some(
                "airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            )
        );
    }

    #[test]
    fn doctor_reports_an_unreadable_lock_alone() {
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::LockText);
        let found = run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5"));
        assert_eq!(found, [format!("agentstacks.lock: {}", denied())]);
    }

    #[test]
    fn doctor_reports_the_lock_refusals() {
        let workspace = FakeWorkspace::new().lock("api = 1\n[[merge]]\nx = 1\n");
        let found = run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5"));
        assert!(
            found.contains(&"agentstacks.lock: merge: unknown key".to_owned()),
            "{found:?}"
        );
        assert_eq!(workspace.calls(), [WorkspaceCall::LockText]);
    }

    #[test]
    fn doctor_reports_a_backslash_lock_path() {
        let workspace = FakeWorkspace::new().lock(&lock_text(None, &[("a\\\\b", LOCKED)]));
        let found = run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5"));
        assert_eq!(
            found,
            ["agentstacks.lock: file[0].path: \"a\\b\" must use / as the separator"]
        );
    }

    #[test]
    fn legacy_lock_reports_like_a_current_one() -> TestResult {
        let legacy = lock_text(None, &[(X, LOCKED)]).replacen("api = 1", "api = 0", 1);
        assert!(legacy.starts_with("api = 0\n"));
        let workspace = FakeWorkspace::new().lock(&legacy).file(&path(X)?, LOCKED);
        assert!(run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")).is_empty());
        let edited = FakeWorkspace::new()
            .lock(&legacy)
            .file(&path(X)?, b"edited");
        assert_eq!(
            run(&edited, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["changed: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_finds_nothing_for_an_empty_healthy_lock() {
        let binary = FakeBinary::new();
        let workspace = FakeWorkspace::new().lock(&lock_text(None, &[]));
        assert!(run(&workspace, &binary).is_empty());
        assert!(binary.asked().is_empty());
    }

    #[test]
    fn doctor_reports_a_missing_file() {
        let workspace = FakeWorkspace::new().lock(&lock_text(
            None,
            &[(".claude/skills/status/SKILL.md", LOCKED)],
        ));
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["missing: .claude/skills/status/SKILL.md"]
        );
    }

    #[test]
    fn doctor_reports_a_directory_as_not_a_regular_file() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new()).not_regular(&path(X)?);
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["not a regular file: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_reports_a_dangling_symlink_as_not_a_regular_file() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new()).symlink(&path(X)?, &path(".claude/gone")?);
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["not a regular file: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_reports_a_changed_file() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new()).file(&path(X)?, b"edited");
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["changed: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_accepts_a_symlink_to_a_matching_file_inside() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new())
            .file(&path("shared/x")?, LOCKED)
            .symlink(&path(X)?, &path("shared/x")?);
        assert!(run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")).is_empty());
        Ok(())
    }

    #[test]
    fn doctor_reports_a_symlink_to_a_changed_file_inside() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new())
            .file(&path("shared/x")?, b"edited")
            .symlink(&path(X)?, &path("shared/x")?);
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["changed: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_reports_a_file_resolving_outside() -> TestResult {
        let workspace = locked_x(FakeWorkspace::new()).outside(&path(X)?);
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["outside the project: .claude/x"]
        );
        Ok(())
    }

    #[test]
    fn doctor_reports_an_unreadable_file() -> TestResult {
        let workspace = FakeWorkspace::new()
            .lock(&lock_text(None, &[(X, LOCKED), (".claude/y", LOCKED)]))
            .fail(WorkspaceCall::Resolve(path(X)?));
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            [
                format!(".claude/x: {}", denied()),
                "missing: .claude/y".to_owned()
            ]
        );
        Ok(())
    }

    #[test]
    fn doctor_reports_every_drifted_file_in_path_order() -> TestResult {
        let workspace = FakeWorkspace::new()
            .lock(&lock_text(
                None,
                &[("c.md", LOCKED), ("a.md", LOCKED), ("b.md", LOCKED)],
            ))
            .file(&path("b.md")?, LOCKED)
            .file(&path("c.md")?, b"edited");
        assert_eq!(
            run(&workspace, &FakeBinary::new().airsl("airsl 0.1.5")),
            ["missing: a.md", "changed: c.md"]
        );
        Ok(())
    }

    fn pinned() -> FakeWorkspace {
        FakeWorkspace::new().lock(&lock_text(Some("0.1.4"), &[]))
    }

    #[test]
    fn doctor_reports_airsl_missing_from_path() {
        assert_eq!(
            run(&pinned(), &FakeBinary::new()),
            [
                "airsl: not found on PATH (lock pins 0.1.4) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            ]
        );
    }

    #[test]
    fn doctor_reports_an_airsl_spawn_failure() {
        let binary = FakeBinary::new().airsl_spawn_failure();
        assert_eq!(
            run(&pinned(), &binary),
            [format!("agentstacks.toml: airsl: {}", FakeBinary::denied())]
        );
    }

    #[test]
    fn doctor_reports_a_failing_airsl_as_unparseable() {
        let binary = FakeBinary::new().airsl_exit_failure("boom");
        assert_eq!(
            run(&pinned(), &binary),
            ["airsl: --version printed \"boom\", expected \"airsl <version>\""]
        );
    }

    #[test]
    fn doctor_reports_unparseable_airsl_version() {
        let binary = FakeBinary::new().airsl("airsl version 0.1.4");
        assert_eq!(
            run(&pinned(), &binary),
            ["airsl: --version printed \"airsl version 0.1.4\", expected \"airsl <version>\""]
        );
    }

    #[test]
    fn doctor_reports_a_different_airsl_version() {
        let binary = FakeBinary::new().airsl("airsl 0.1.5");
        assert_eq!(
            run(&pinned(), &binary),
            ["airsl: 0.1.5 installed, lock pins 0.1.4"]
        );
    }

    #[test]
    fn doctor_accepts_the_pinned_airsl() {
        let binary = FakeBinary::new().airsl("airsl 0.1.4");
        assert!(run(&pinned(), &binary).is_empty());
        assert_eq!(binary.asked(), [BinaryCall::AirslVersion]);
    }

    #[test]
    fn doctor_skips_airsl_without_a_pin() {
        let binary = FakeBinary::new().airsl("airsl 0.1.5");
        let workspace = FakeWorkspace::new().lock(&lock_text(None, &[]));
        assert!(run(&workspace, &binary).is_empty());
        assert!(binary.asked().is_empty());
    }

    #[test]
    fn doctor_reports_missing_airsl_for_a_hook_runtime_without_a_pin() -> TestResult {
        let workspace = FakeWorkspace::new()
            .lock(&lock_text(
                None,
                &[(".claude/agentstacks/runtime.lua", LOCKED)],
            ))
            .file(&path(".claude/agentstacks/runtime.lua")?, LOCKED);
        assert_eq!(
            run(&workspace, &FakeBinary::new()),
            [
                "airsl: not found on PATH (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            ]
        );
        Ok(())
    }

    #[test]
    fn doctor_puts_airsl_after_the_files() -> TestResult {
        let workspace = FakeWorkspace::new()
            .lock(&lock_text(Some("0.1.4"), &[(X, LOCKED)]))
            .file(&path(X)?, b"edited");
        assert_eq!(
            run(&workspace, &FakeBinary::new()),
            [
                "changed: .claude/x",
                "airsl: not found on PATH (lock pins 0.1.4) (install: curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh)"
            ]
        );
        Ok(())
    }
}
