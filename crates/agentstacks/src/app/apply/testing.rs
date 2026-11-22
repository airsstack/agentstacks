//! Plans and a project to run the apply tests against.
//!
//! # Responsibilities
//!
//! - [`plan`] builds a [`Plan`] from steps with the plan rules, so its lock is the one a full
//!   apply writes.
//! - [`six`] is a plan with one step of every action, and the project it was decided on.
//! - [`step`], [`shown`] and [`written`] make a step, read a refusal as text and read back the
//!   lock a test run wrote.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only and decides nothing.

use std::path::Path;

use crate::port::FakeWorkspace;
use crate::types::lock::{FileEntry, LockContents, LockedEntry, LockedSource, Lockfile, Owner};
use crate::types::manifest::PackVersion;
use crate::types::name::{PackName, ToolName};
use crate::types::op::OpPath;
use crate::types::plan::{Action, DiskFile, DiskState, Landed, Plan, Step, lock_contents};
use crate::types::project::{LocalPath, SubPath};
use crate::types::report::Diagnostics;
use crate::types::tree::ContentHash;

/// What a test body returns.
pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A fallible value in a test.
pub(super) type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

/// A project path.
pub(super) fn path(text: &str) -> Fallible<OpPath> {
    Ok(OpPath::new(text)?)
}

/// The state of a regular file holding `bytes`.
pub(super) fn file(bytes: &[u8]) -> DiskState {
    DiskState::File(DiskFile::new(bytes.to_vec(), FakeWorkspace::NEW_MODE))
}

/// A step. `owner` is the pack or tool that wrote the file, `None` for a shared file.
pub(super) fn step(
    at: &str,
    action: Action,
    owner: Option<&str>,
    new: Option<&[u8]>,
    decided_on: Option<DiskState>,
) -> Fallible<Step> {
    Ok(Step {
        path: path(at)?,
        action,
        owner: owner.map(Owner::new).transpose()?,
        origin: None,
        new: new.map(<[u8]>::to_vec),
        old: None,
        decided_on,
    })
}

/// The entry the old lock held for `at`.
pub(super) fn old(at: &str, bytes: &[u8], by: &str) -> Fallible<FileEntry> {
    Ok(FileEntry::new(
        SubPath::new(at)?,
        ContentHash::of_bytes(bytes),
        Owner::new(by)?,
    ))
}

/// The version the new lock records for the pack `core`.
pub(super) const CORE_VERSION: &str = "0.3.0";

/// A plan of `steps`, made against a lock that held `old_files`. Its lock holds the tool `claude`,
/// the pack `core` at [`CORE_VERSION`] and the entry of every step that gives one.
pub(super) fn plan(steps: Vec<Step>, old_files: Vec<FileEntry>) -> Fallible<Plan> {
    let tree = ContentHash::of_bytes(b"tree");
    let tool = LockedEntry::new(
        ToolName::new("claude")?,
        PackVersion::new("0.1.0")?,
        LockedSource::Path(LocalPath::new("extensions/claude")?),
        tree.clone(),
    );
    let pack = LockedEntry::new(
        PackName::new("core")?,
        PackVersion::new(CORE_VERSION)?,
        LockedSource::Path(LocalPath::new("packs/core")?),
        tree,
    );
    let contents: LockContents = lock_contents(
        vec![tool],
        vec![pack],
        &steps,
        Some(PackVersion::new("0.1.4")?),
        Landed::All,
    )?;
    let lock = Lockfile::new(Path::new("agentstacks.lock"), contents)
        .map_err(|refusals| format!("{:?}", shown(refusals)))?;
    Ok(Plan::new(
        steps,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        lock,
        old_files,
    ))
}

/// A plan with a step of every action, and the project it was decided on.
///
/// ```text
/// .claude/a  create     absent
/// .claude/b  update     "old b"
/// .claude/c  restore    absent
/// .claude/d  unchanged  "new d"
/// .claude/e  delete     "old e"
/// .claude/f  forget     dangling symlink
/// ```
pub(super) fn six() -> Fallible<(Plan, FakeWorkspace)> {
    let steps = vec![
        step(
            ".claude/a",
            Action::Create,
            Some("core"),
            Some(b"new a"),
            Some(DiskState::Absent),
        )?,
        step(
            ".claude/b",
            Action::Update,
            Some("core"),
            Some(b"new b"),
            Some(file(b"old b")),
        )?,
        step(
            ".claude/c",
            Action::Restore,
            Some("core"),
            Some(b"new c"),
            Some(DiskState::Absent),
        )?,
        step(
            ".claude/d",
            Action::Unchanged,
            Some("core"),
            Some(b"new d"),
            Some(file(b"new d")),
        )?,
        step(
            ".claude/e",
            Action::Delete,
            Some("core"),
            None,
            Some(file(b"old e")),
        )?,
        step(".claude/f", Action::Forget, Some("core"), None, None)?,
    ];
    let olds = vec![
        old(".claude/b", b"old b", "core")?,
        old(".claude/c", b"old c", "core")?,
        old(".claude/e", b"old e", "core")?,
        old(".claude/f", b"old f", "core")?,
    ];
    let workspace = FakeWorkspace::new()
        .file(&path(".claude/b")?, b"old b")
        .file(&path(".claude/d")?, b"new d")
        .file(&path(".claude/e")?, b"old e")
        .symlink(&path(".claude/f")?, &path(".claude/gone")?);
    Ok((plan(steps, olds)?, workspace))
}

/// The refusals as the lines a person reads, in output order.
pub(super) fn shown(refusals: Diagnostics) -> Vec<String> {
    refusals.into_iter().map(|d| d.to_string()).collect()
}

/// The lock the run wrote, read back.
pub(super) fn written(workspace: &FakeWorkspace) -> Fallible<Lockfile> {
    let text = workspace.lock_written().ok_or("a lock was written")?;
    Lockfile::parse(Path::new("agentstacks.lock"), &text)
        .map_err(|refusals| format!("{:?}", shown(refusals)).into())
}

/// The hash `lock` records for `at`.
pub(super) fn hash_in(lock: &Lockfile, at: &str) -> Option<ContentHash> {
    lock.files()
        .iter()
        .find(|entry| entry.path().as_str() == at)
        .map(|entry| entry.hash().clone())
}
