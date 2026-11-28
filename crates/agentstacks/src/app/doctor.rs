//! The flow that reports drift between the project disk and `agentstacks.lock`, and notices
//! newer releases upstream.
//!
//! It exists so a user can learn, without writing anything, whether the files an earlier apply
//! wrote are still as the lock records them, whether the airsl binary is the pinned one, and
//! whether a locked source has a newer tag upstream. The drift half needs no network and no git;
//! the freshness half lists each locked git source under its own stream prefix and degrades to a
//! note when the network, git, or the listing is unavailable.
//!
//! # Responsibilities
//!
//! - [`doctor`] reads the lock text, parses it, asks the workspace where each locked path leads,
//!   asks the binary port for the airsl version when the project needs airsl, and hands each answer
//!   to the drift rules in `types`. It orders the calls and collects the findings: the lock's own
//!   findings when the lock is missing, unreadable or refused, otherwise one finding at most per
//!   locked file in the lock's path order, then the airsl finding.
//! - [`doctor`] also hands each locked git source to the update rules in `types` and collects an
//!   advisory per source that has a newer tag, is pinned to a frozen bundle, or could not be
//!   checked. Advisories never fail the run; only findings do.
//!
//! # Non-responsibilities
//!
//! No rule: whether a file counts as changed, which update note a source earns, and the words of
//! every finding, belong to the drift and update rules and to [`Finding`]. No extension and no
//! write. Showing the report and choosing the exit code belong to the command.
//!
//! [Finding]: crate::types::plan::Finding

use std::fmt;
use std::path::Path;

use crate::port::{Binary, Git, Workspace};
use crate::types::lock::{FileEntry, LockedSource, Lockfile};
use crate::types::op::OpPath;
use crate::types::plan::{Finding, airsl_finding, file_finding};
use crate::types::project::{GitTag, GitUrl, Pin, Source};
use crate::types::resolve::{SourceKey, UpdateNote, UpdateOutcome, UpdateQuery};

/// The lock's name in the project, the file its refusals are located at.
const LOCK_FILE: &str = "agentstacks.lock";

/// What `doctor` noticed about a locked git source, without failing the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advisory {
    /// A newer tag of the source's stream exists; the lock still pins the current one.
    Newer {
        /// The locked source.
        key: SourceKey,
        /// The tag the lock pins.
        current: GitTag,
        /// The newest listed tag.
        newest: GitTag,
    },
    /// The update note for the source: frozen, pinned, versionless, or missing upstream.
    Note(UpdateNote),
    /// The listed stream holds no tag at all.
    NoStream {
        /// The locked source.
        key: SourceKey,
        /// The listed stream prefix.
        prefix: String,
        /// The listed repository.
        url: GitUrl,
    },
    /// The pinned tag is not among the listed tags.
    PinnedMissing {
        /// The locked source.
        key: SourceKey,
        /// The tag the lock pins.
        tag: GitTag,
    },
    /// The stream could not be listed: git missing, unreachable, or the list failed.
    Unchecked {
        /// The locked source.
        key: SourceKey,
    },
}

impl fmt::Display for Advisory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Newer {
                key,
                current,
                newest,
            } => write!(
                f,
                "{}: {newest} is available (pinned {current})",
                key.location()
            ),
            Self::Note(note) => write!(f, "{note}"),
            Self::NoStream { key, prefix, url } => write!(
                f,
                "{}: stream {prefix} lists no release tag in {url}",
                key.location()
            ),
            Self::PinnedMissing { key, tag } => {
                write!(f, "{}: pinned tag {tag} not found upstream", key.location())
            }
            Self::Unchecked { key } => {
                write!(f, "{}: could not check for updates", key.location())
            }
        }
    }
}

/// What `doctor` found: drift findings, which fail the run, and advisories, which do not.
#[derive(Debug, Default)]
pub struct DoctorReport {
    /// Drift between the project disk and `agentstacks.lock`; non-empty fails the run.
    pub findings: Vec<Finding>,
    /// Newer tags, frozen pins, and uncheckable streams; never fails the run.
    pub advisories: Vec<Advisory>,
}

impl DoctorReport {
    /// Whether there is nothing to report at all.
    #[must_use]
    pub const fn healthy(&self) -> bool {
        self.findings.is_empty() && self.advisories.is_empty()
    }
}

/// Drift against `agentstacks.lock`, plus an advisory per locked git source with a newer tag, a
/// frozen bundle pin, or a stream that could not be checked.
///
/// When the lock is missing, cannot be read or is refused, the report holds the findings about
/// the lock and the airsl binary, because there is nothing else to check against. Otherwise it
/// holds the findings about the locked files in the lock's path order, every file checked, then
/// the airsl finding when the project needs it, and one advisory at most per locked git source
/// in pack then tool order. Advisories never fail the run; only findings do.
#[must_use]
pub fn doctor<W: Workspace, B: Binary, G: Git>(workspace: &W, binary: &B, git: &G) -> DoctorReport {
    let text = match workspace.lock_text() {
        Ok(Some(text)) => text,
        Ok(None) => {
            let mut findings = vec![Finding::LockMissing];
            findings.extend(airsl_finding(None, binary.airsl_version()));
            return DoctorReport {
                findings,
                advisories: Vec::new(),
            };
        }
        Err(error) => {
            let mut findings = vec![Finding::LockIo(error)];
            findings.extend(airsl_finding(None, binary.airsl_version()));
            return DoctorReport {
                findings,
                advisories: Vec::new(),
            };
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
            return DoctorReport {
                findings,
                advisories: Vec::new(),
            };
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
    let advisories = freshness(&lock, git);
    DoctorReport {
        findings,
        advisories,
    }
}

/// One advisory at most per locked git source, in pack then tool order.
fn freshness<G: Git>(lock: &Lockfile, git: &G) -> Vec<Advisory> {
    let mut advisories = Vec::new();
    for entry in lock.packs() {
        check_entry(
            SourceKey::Pack(entry.name().clone()),
            entry.source(),
            git,
            &mut advisories,
        );
    }
    for entry in lock.tools() {
        check_entry(
            SourceKey::Tool(entry.name().clone()),
            entry.source(),
            git,
            &mut advisories,
        );
    }
    advisories
}

/// The advisory for one locked source: nothing for a path source, a note for a rev pin or a tag
/// with no version, the frozen or newer-tag answer for a versioned tag, and an unchecked note
/// when the stream cannot be listed at all.
fn check_entry<G: Git>(
    key: SourceKey,
    source: &LockedSource,
    git: &G,
    advisories: &mut Vec<Advisory>,
) {
    let LockedSource::Git {
        url,
        tag,
        subdir,
        rev,
    } = source
    else {
        return;
    };
    let Some(current) = tag else {
        advisories.push(Advisory::Note(UpdateNote::PinnedRev {
            key,
            rev: rev.clone(),
        }));
        return;
    };
    let source = Source::Git {
        url: url.clone(),
        pin: Pin::Tag(current.clone()),
        subdir: subdir.clone(),
    };
    let query = match UpdateQuery::for_source(key.clone(), &source) {
        None => return,
        Some(Err(entry)) => {
            push_note(&entry, advisories);
            return;
        }
        Some(Ok(query)) => query,
    };
    let Ok(tags) = git.list_tags(&query.url, &query.prefix) else {
        advisories.push(Advisory::Unchecked { key });
        return;
    };
    if query.frozen.is_none() {
        if tags.is_empty() {
            advisories.push(Advisory::NoStream {
                key,
                prefix: query.prefix,
                url: query.url,
            });
            return;
        }
        if !tags.iter().any(|remote| remote.tag == *current) {
            advisories.push(Advisory::PinnedMissing {
                key,
                tag: current.clone(),
            });
            return;
        }
    }
    match query.finish(&tags).outcome {
        UpdateOutcome::NewTag(newest) => advisories.push(Advisory::Newer {
            key,
            current: current.clone(),
            newest,
        }),
        UpdateOutcome::Note(UpdateNote::NoNewer { .. }) => {}
        UpdateOutcome::Note(note) => advisories.push(Advisory::Note(note)),
    }
}

/// The note of an update entry that is known to carry one.
fn push_note(entry: &crate::types::resolve::UpdateEntry, advisories: &mut Vec<Advisory>) {
    if let UpdateOutcome::Note(note) = &entry.outcome {
        advisories.push(Advisory::Note(note.clone()));
    }
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
    use crate::port::{BinaryCall, FakeBinary, FakeGit, FakeWorkspace, WorkspaceCall};
    use crate::types::project::CommitId;
    use crate::types::resolve::RemoteTag;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";
    const LOCKED: &[u8] = b"locked";
    const X: &str = ".claude/x";

    const OFFICIAL: &str = "https://github.com/airsstack/agentstacks";
    const THIRD: &str = "https://example.com/org/packs";

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

    /// A lock holding one pack from a git source at `tag` (or at a rev when `tag` is `None`),
    /// with no files, so drift stays quiet.
    fn git_lock(url: &str, tag: Option<&str>) -> String {
        let tag_line = tag.map_or_else(String::new, |tag| format!("tag = \"{tag}\"\n"));
        format!(
            "api = 1\n\n[[pack]]\nname = \"core\"\nversion = \"0.3.0\"\n\
             git = \"{url}\"\n{tag_line}subdir = \"packs/core\"\nrev = \"{HEX40}\"\n\
             tree = \"sha256:{HEX64}\"\n"
        )
    }

    fn remote(url: &str, tag: &str) -> Result<(GitUrl, RemoteTag), Box<dyn std::error::Error>> {
        Ok((
            GitUrl::new(url)?,
            RemoteTag {
                rev: CommitId::new(HEX40)?,
                tag: GitTag::new(tag)?,
            },
        ))
    }

    fn listing(
        git: FakeGit,
        url: &str,
        tags: &[&str],
    ) -> Result<FakeGit, Box<dyn std::error::Error>> {
        let mut remotes = Vec::new();
        for tag in tags {
            let (_, remote) = remote(url, tag)?;
            remotes.push(remote);
        }
        Ok(git.tags(&GitUrl::new(url)?, remotes))
    }

    fn shown(findings: &[Finding]) -> Vec<String> {
        findings.iter().map(ToString::to_string).collect()
    }

    fn advised(report: &DoctorReport) -> Vec<String> {
        report.advisories.iter().map(ToString::to_string).collect()
    }

    fn denied() -> String {
        FakeWorkspace::denied().to_string()
    }

    fn run(workspace: &FakeWorkspace, binary: &FakeBinary) -> Vec<String> {
        shown(&doctor(workspace, binary, &FakeGit::new()).findings)
    }

    fn run_all(workspace: &FakeWorkspace, binary: &FakeBinary, git: &FakeGit) -> DoctorReport {
        doctor(workspace, binary, git)
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

    #[test]
    fn stale_stream_reports_advisory_at_exit_zero() -> TestResult {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, Some("packs-core-v1.0.1")));
        let git = listing(
            FakeGit::new(),
            THIRD,
            &["packs-core-v1.0.1", "packs-core-v1.0.2"],
        )?;
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            [
                "agentstacks.toml: packs.core: packs-core-v1.0.2 is available (pinned packs-core-v1.0.1)"
            ]
        );
        Ok(())
    }

    #[test]
    fn frozen_bundle_pin_reports_stream() -> TestResult {
        let workspace = FakeWorkspace::new().lock(&git_lock(OFFICIAL, Some("packs-v1.2.0")));
        let git = listing(
            FakeGit::new(),
            OFFICIAL,
            &["packs-core-v1.0.1", "packs-core-v1.0.2"],
        )?;
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            [
                "agentstacks.toml: packs.core: pinned to bundle tag packs-v1.2.0 (frozen stream); packs-core-v1.0.2 is available — retag to migrate"
            ]
        );
        Ok(())
    }

    #[test]
    fn offline_doctor_behaves_as_today() -> TestResult {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, Some("packs-core-v1.0.1")));
        let git = FakeGit::new().git_missing_for(&GitUrl::new(THIRD)?);
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            ["agentstacks.toml: packs.core: could not check for updates"]
        );
        Ok(())
    }

    #[test]
    fn drift_still_fails() -> TestResult {
        let changed = format!(
            "[[file]]\npath = \"{}\"\nhash = \"{}\"\nby = \"core\"\n",
            X,
            ContentHash::of_bytes(b"changed")
        );
        let workspace = FakeWorkspace::new()
            .lock(&format!(
                "{}{}",
                git_lock(THIRD, Some("packs-core-v1.0.1")),
                changed
            ))
            .file(&path(X)?, LOCKED);
        let git = listing(FakeGit::new(), THIRD, &["packs-core-v1.0.1"])?;
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert_eq!(shown(&report.findings), ["changed: .claude/x"]);
        Ok(())
    }

    #[test]
    fn path_source_reports_nothing() {
        let workspace = FakeWorkspace::new().lock(&lock_text(None, &[]));
        let report = run_all(&workspace, &FakeBinary::new(), &FakeGit::new());
        assert!(report.healthy(), "{report:?}");
    }

    #[test]
    fn rev_pin_reports_pinned_note() {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, None));
        let report = run_all(&workspace, &FakeBinary::new(), &FakeGit::new());
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            [format!(
                "agentstacks.toml: packs.core: pinned to rev {HEX40}, not updated"
            )]
        );
    }

    #[test]
    fn versionless_tag_reports_no_version_note() {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, Some("main-latest")));
        let report = run_all(&workspace, &FakeBinary::new(), &FakeGit::new());
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            ["agentstacks.toml: packs.core: tag \"main-latest\" has no version to update from"]
        );
    }

    #[test]
    fn empty_stream_reports_no_tag_advisory() -> TestResult {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, Some("packs-core-v1.0.1")));
        let git = listing(FakeGit::new(), THIRD, &[])?;
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            [
                "agentstacks.toml: packs.core: stream packs-core-v lists no release tag in https://example.com/org/packs"
            ]
        );
        Ok(())
    }

    #[test]
    fn missing_pinned_tag_reports_not_found() -> TestResult {
        let workspace = FakeWorkspace::new().lock(&git_lock(THIRD, Some("packs-core-v9.9.9")));
        let git = listing(FakeGit::new(), THIRD, &["packs-core-v1.0.1"])?;
        let report = run_all(&workspace, &FakeBinary::new(), &git);
        assert!(report.findings.is_empty(), "{report:?}");
        assert_eq!(
            advised(&report),
            ["agentstacks.toml: packs.core: pinned tag packs-core-v9.9.9 not found upstream"]
        );
        Ok(())
    }
}
