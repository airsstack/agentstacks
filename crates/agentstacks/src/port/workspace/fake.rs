//! An in-memory [`Workspace`] for tests.
//!
//! It lets a flow run against a project without a disk, and makes the cases a disk cannot be made
//! to produce on demand, such as a failing swap or a path that leads outside the project, one
//! builder call.
//!
//! # Responsibilities
//!
//! - [`FakeWorkspace`] holds files, non-regular entries, symlinks and outside paths keyed by their
//!   normalised path, and answers every [`Workspace`] method from them. A staged file is a
//!   temporary file beside its canonical target and a swap moves it onto the target, keeping an
//!   existing file's mode and writing through a symlink.
//! - [`FakeWorkspace::project_written`] and [`FakeWorkspace::lock_written`] return the last text
//!   written to each project file.
//! - [`FakeWorkspace::fail`] makes one exact call fail every time with
//!   [`FakeWorkspace::denied`], and [`FakeWorkspace::set_file`] and [`FakeWorkspace::remove`] edit
//!   the disk between two calls.
//! - [`WorkspaceCall`] is one entry of the call log that [`FakeWorkspace::calls`] returns.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests and under the `testing` feature.
//! How the real disk behaves is proven by the tests of the `fs`
//! module.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

use crate::port::FakeFileTree;
use crate::port::workspace::seam::Workspace;
use crate::types::op::OpPath;
use crate::types::plan::{DiskFile, DiskState, Link, Resolution};
use crate::types::project::LocalPath;

/// What one project path holds in the fake.
#[derive(Debug, Clone)]
enum Entry {
    File { bytes: Vec<u8>, mode: u32 },
    NotRegular,
    Symlink(OpPath),
    Outside,
}

/// One call to a [`FakeWorkspace`], the unit of its call log and of [`FakeWorkspace::fail`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceCall {
    /// [`Workspace::project_text`].
    ProjectText,
    /// [`Workspace::lock_text`].
    LockText,
    /// [`Workspace::resolve`] of a path.
    Resolve(OpPath),
    /// [`Workspace::source_tree`] of a directory.
    SourceTree(LocalPath),
    /// [`Workspace::stage`] of a path.
    Stage(OpPath),
    /// [`Workspace::discard`] of a path.
    Discard(OpPath),
    /// [`Workspace::swap`] of a path.
    Swap(OpPath),
    /// [`Workspace::delete`] of a path.
    Delete(OpPath),
    /// [`Workspace::write_lock`].
    WriteLock,
    /// [`Workspace::write_project`].
    WriteProject,
}

/// The part of the fake that the write methods change.
#[derive(Debug, Default)]
struct Disk {
    entries: BTreeMap<OpPath, Entry>,
    /// Temporary files, keyed by the canonical path of the file they will replace.
    temps: BTreeMap<OpPath, Vec<u8>>,
    project: Option<String>,
    project_written: Option<String>,
    lock: Option<String>,
    lock_written: Option<String>,
}

impl Disk {
    /// The canonical target of `path`: itself, the target of a symlink, or `None` when it leads
    /// outside the project.
    fn canonical(&self, path: &OpPath) -> Option<OpPath> {
        let target = match self.entries.get(path) {
            Some(Entry::Outside) => return None,
            Some(Entry::Symlink(target)) => target,
            _ => return Some(path.clone()),
        };
        match self.entries.get(target) {
            Some(Entry::Outside) => None,
            _ => Some(target.clone()),
        }
    }

    fn resolve(&self, path: &OpPath) -> Resolution {
        let state_of = |entry: Option<&Entry>| match entry {
            None => DiskState::Absent,
            Some(Entry::File { bytes, mode }) => {
                DiskState::File(DiskFile::new(bytes.clone(), *mode))
            }
            Some(_) => DiskState::NotRegular,
        };
        match self.entries.get(path) {
            Some(Entry::Outside) => Resolution::Outside {
                link: Link::Symlink,
            },
            Some(Entry::Symlink(target)) => match self.entries.get(target) {
                Some(Entry::Outside) => Resolution::Outside {
                    link: Link::Symlink,
                },
                Some(file @ Entry::File { .. }) => Resolution::Inside {
                    link: Link::Symlink,
                    state: state_of(Some(file)),
                },
                _ => Resolution::Inside {
                    link: Link::Symlink,
                    state: DiskState::NotRegular,
                },
            },
            other => Resolution::Inside {
                link: Link::Direct,
                state: state_of(other),
            },
        }
    }
}

/// An in-memory project.
#[derive(Debug, Default)]
pub struct FakeWorkspace {
    sources: BTreeMap<LocalPath, Option<FakeFileTree>>,
    failing: Vec<WorkspaceCall>,
    disk: RefCell<Disk>,
    calls: RefCell<Vec<WorkspaceCall>>,
}

impl FakeWorkspace {
    /// The permission mode a file the fake creates gets, the mode a plain write gives on disk.
    pub const NEW_MODE: u32 = 0o644;

    /// An empty project with no `agentstacks.toml` and no lock.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the text of `agentstacks.toml`.
    #[must_use]
    pub fn project(self, text: &str) -> Self {
        self.disk.borrow_mut().project = Some(text.to_owned());
        self
    }

    /// Sets the text of `agentstacks.lock`.
    #[must_use]
    pub fn lock(self, text: &str) -> Self {
        self.disk.borrow_mut().lock = Some(text.to_owned());
        self
    }

    /// Adds a regular file with mode [`FakeWorkspace::NEW_MODE`].
    #[must_use]
    pub fn file(self, path: &OpPath, bytes: &[u8]) -> Self {
        self.file_with_mode(path, bytes, Self::NEW_MODE)
    }

    /// Adds a regular file with the permission `mode`.
    #[must_use]
    pub fn file_with_mode(self, path: &OpPath, bytes: &[u8], mode: u32) -> Self {
        self.set_entry(
            path,
            Entry::File {
                bytes: bytes.to_vec(),
                mode,
            },
        );
        self
    }

    /// Adds a path that is a directory or a special file.
    #[must_use]
    pub fn not_regular(self, path: &OpPath) -> Self {
        self.set_entry(path, Entry::NotRegular);
        self
    }

    /// Adds a symlink inside the project that leads to `target`. A target with no entry makes it
    /// dangling.
    #[must_use]
    pub fn symlink(self, path: &OpPath, target: &OpPath) -> Self {
        self.set_entry(path, Entry::Symlink(target.clone()));
        self
    }

    /// Adds a path that leads outside the project, such as a link to a system file.
    #[must_use]
    pub fn outside(self, path: &OpPath) -> Self {
        self.set_entry(path, Entry::Outside);
        self
    }

    /// Serves `tree` for the local source directory `dir`.
    #[must_use]
    pub fn source(mut self, dir: &LocalPath, tree: FakeFileTree) -> Self {
        self.sources.insert(dir.clone(), Some(tree));
        self
    }

    /// Makes the local source directory `dir` lead outside the project.
    #[must_use]
    pub fn source_outside(mut self, dir: &LocalPath) -> Self {
        self.sources.insert(dir.clone(), None);
        self
    }

    /// Makes `call` fail every time with [`FakeWorkspace::denied`] and change nothing.
    #[must_use]
    pub fn fail(mut self, call: WorkspaceCall) -> Self {
        self.failing.push(call);
        self
    }

    /// The error every failing call returns.
    #[must_use]
    pub fn denied() -> io::Error {
        FakeFileTree::denied()
    }

    /// Replaces the bytes of `path` between two calls, keeping the mode of a file that is there.
    pub fn set_file(&self, path: &OpPath, bytes: &[u8]) {
        let mut disk = self.disk.borrow_mut();
        let mode = match disk.entries.get(path) {
            Some(Entry::File { mode, .. }) => *mode,
            _ => Self::NEW_MODE,
        };
        disk.entries.insert(
            path.clone(),
            Entry::File {
                bytes: bytes.to_vec(),
                mode,
            },
        );
    }

    /// Removes whatever is at `path` between two calls.
    pub fn remove(&self, path: &OpPath) {
        self.disk.borrow_mut().entries.remove(path);
    }

    /// Every call so far, in order, the failed ones included.
    #[must_use]
    pub fn calls(&self) -> Vec<WorkspaceCall> {
        self.calls.borrow().clone()
    }

    /// The bytes of the regular file at `path`, without following a symlink.
    #[must_use]
    pub fn file_bytes(&self, path: &OpPath) -> Option<Vec<u8>> {
        match self.disk.borrow().entries.get(path) {
            Some(Entry::File { bytes, .. }) => Some(bytes.clone()),
            _ => None,
        }
    }

    /// The permission mode of the regular file at `path`, without following a symlink.
    #[must_use]
    pub fn mode(&self, path: &OpPath) -> Option<u32> {
        match self.disk.borrow().entries.get(path) {
            Some(Entry::File { mode, .. }) => Some(*mode),
            _ => None,
        }
    }

    /// The canonical paths that still have a temporary file.
    #[must_use]
    pub fn temps(&self) -> Vec<OpPath> {
        self.disk.borrow().temps.keys().cloned().collect()
    }

    /// The last text given to [`Workspace::write_project`].
    #[must_use]
    pub fn project_written(&self) -> Option<String> {
        self.disk.borrow().project_written.clone()
    }

    /// The last text given to [`Workspace::write_lock`].
    #[must_use]
    pub fn lock_written(&self) -> Option<String> {
        self.disk.borrow().lock_written.clone()
    }

    fn set_entry(&self, path: &OpPath, entry: Entry) {
        self.disk.borrow_mut().entries.insert(path.clone(), entry);
    }

    /// Logs `call`, then fails it when it is set to fail.
    fn enter(&self, call: WorkspaceCall) -> io::Result<()> {
        let failing = self.failing.contains(&call);
        self.calls.borrow_mut().push(call);
        if failing { Err(Self::denied()) } else { Ok(()) }
    }

    /// The refusal for a write to a path that leads outside the project.
    fn outside_error() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, "resolves outside the project")
    }
}

impl Workspace for FakeWorkspace {
    type Tree = FakeFileTree;

    fn project_text(&self) -> io::Result<String> {
        self.enter(WorkspaceCall::ProjectText)?;
        self.disk
            .borrow()
            .project
            .clone()
            .ok_or_else(FakeFileTree::not_found)
    }

    fn lock_text(&self) -> io::Result<Option<String>> {
        self.enter(WorkspaceCall::LockText)?;
        Ok(self.disk.borrow().lock.clone())
    }

    fn resolve(&self, path: &OpPath) -> io::Result<Resolution> {
        self.enter(WorkspaceCall::Resolve(path.clone()))?;
        Ok(self.disk.borrow().resolve(path))
    }

    fn source_tree(&self, dir: &LocalPath) -> io::Result<Option<Self::Tree>> {
        self.enter(WorkspaceCall::SourceTree(dir.clone()))?;
        Ok(match self.sources.get(dir) {
            Some(Some(tree)) => Some(tree.clone()),
            Some(None) => None,
            None => Some(FakeFileTree::missing(dir.as_str())),
        })
    }

    fn stage(&self, path: &OpPath, bytes: &[u8]) -> io::Result<()> {
        self.enter(WorkspaceCall::Stage(path.clone()))?;
        let mut disk = self.disk.borrow_mut();
        let target = disk.canonical(path).ok_or_else(Self::outside_error)?;
        disk.temps.insert(target, bytes.to_vec());
        Ok(())
    }

    fn discard(&self, path: &OpPath) -> io::Result<()> {
        self.enter(WorkspaceCall::Discard(path.clone()))?;
        let mut disk = self.disk.borrow_mut();
        let target = disk.canonical(path).unwrap_or_else(|| path.clone());
        disk.temps.remove(&target);
        Ok(())
    }

    fn swap(&self, path: &OpPath) -> io::Result<()> {
        self.enter(WorkspaceCall::Swap(path.clone()))?;
        let mut disk = self.disk.borrow_mut();
        let target = disk.canonical(path).ok_or_else(Self::outside_error)?;
        let bytes = disk
            .temps
            .remove(&target)
            .ok_or_else(FakeFileTree::not_found)?;
        let mode = match disk.entries.get(&target) {
            Some(Entry::File { mode, .. }) => *mode,
            _ => Self::NEW_MODE,
        };
        disk.entries.insert(target, Entry::File { bytes, mode });
        Ok(())
    }

    fn delete(&self, path: &OpPath) -> io::Result<()> {
        self.enter(WorkspaceCall::Delete(path.clone()))?;
        let mut disk = self.disk.borrow_mut();
        if disk.canonical(path).is_none() {
            return Err(Self::outside_error());
        }
        disk.entries
            .remove(path)
            .map(|_| ())
            .ok_or_else(FakeFileTree::not_found)
    }

    fn write_project(&self, text: &str) -> io::Result<()> {
        self.enter(WorkspaceCall::WriteProject)?;
        let mut disk = self.disk.borrow_mut();
        disk.project = Some(text.to_owned());
        disk.project_written = Some(text.to_owned());
        Ok(())
    }

    fn write_lock(&self, text: &str) -> io::Result<()> {
        self.enter(WorkspaceCall::WriteLock)?;
        let mut disk = self.disk.borrow_mut();
        disk.lock = Some(text.to_owned());
        disk.lock_written = Some(text.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::tree::FileTree;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn p(text: &str) -> Result<OpPath, Box<dyn std::error::Error>> {
        Ok(OpPath::new(text)?)
    }

    fn read_project<W: Workspace>(workspace: &W) -> std::io::Result<String> {
        workspace.project_text()
    }

    #[test]
    fn resolve_reports_each_disk_state() -> TestResult {
        let ws = FakeWorkspace::new()
            .file_with_mode(&p("f")?, b"x", 0o755)
            .not_regular(&p("d")?)
            .outside(&p("o")?);
        let Resolution::Inside {
            link: Link::Direct,
            state: DiskState::File(file),
        } = ws.resolve(&p("f")?)?
        else {
            return Err("f is an inside regular file".into());
        };
        assert_eq!(file.hash(), &ContentHash::of_bytes(b"x"));
        assert_eq!(file.bytes(), b"x");
        assert_eq!(file.mode(), 0o755);
        assert_eq!(
            ws.resolve(&p("none")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::Absent
            }
        );
        assert_eq!(
            ws.resolve(&p("d")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::NotRegular
            }
        );
        assert!(matches!(ws.resolve(&p("o")?)?, Resolution::Outside { .. }));
        Ok(())
    }

    #[test]
    fn resolve_follows_an_inside_symlink() -> TestResult {
        let ws = FakeWorkspace::new()
            .file(&p("real")?, b"x")
            .symlink(&p("link")?, &p("real")?)
            .symlink(&p("dangling")?, &p("gone")?);
        let Resolution::Inside {
            link: Link::Symlink,
            state: DiskState::File(file),
        } = ws.resolve(&p("link")?)?
        else {
            return Err("link leads to a file".into());
        };
        assert_eq!(file.bytes(), b"x");
        assert_eq!(
            ws.resolve(&p("dangling")?)?,
            Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::NotRegular
            }
        );
        Ok(())
    }

    #[test]
    fn project_and_lock_text() -> TestResult {
        let ws = FakeWorkspace::new().project("tools = []\n");
        assert_eq!(ws.project_text()?, "tools = []\n");
        assert_eq!(ws.lock_text()?, None);
        let ws = ws.lock("# lock\n");
        assert_eq!(ws.lock_text()?, Some("# lock\n".to_owned()));
        let kind = FakeWorkspace::new()
            .project_text()
            .err()
            .map(|error| error.kind());
        assert_eq!(kind, Some(std::io::ErrorKind::NotFound));
        Ok(())
    }

    #[test]
    fn source_tree_answers_configured_outside_and_missing() -> TestResult {
        let dir = LocalPath::new("./packs/core")?;
        let away = LocalPath::new("./away")?;
        let gone = LocalPath::new("./gone")?;
        let ws = FakeWorkspace::new()
            .source(
                &dir,
                FakeFileTree::new("packs/core").file("pack.toml", b"x"),
            )
            .source_outside(&away);
        let tree = ws.source_tree(&dir)?.ok_or("a configured tree")?;
        assert_eq!(tree.read_file(std::path::Path::new("pack.toml"))?, b"x");
        assert!(ws.source_tree(&away)?.is_none());
        let missing = ws.source_tree(&gone)?.ok_or("a missing tree")?;
        let kind = missing
            .read_dir(std::path::Path::new(""))
            .err()
            .map(|error| error.kind());
        assert_eq!(kind, Some(std::io::ErrorKind::NotFound));
        Ok(())
    }

    #[test]
    fn stage_then_swap_installs_bytes_and_keeps_mode() -> TestResult {
        let ws = FakeWorkspace::new().file_with_mode(&p(".claude/old")?, b"old", 0o755);
        ws.stage(&p(".claude/old")?, b"new")?;
        ws.stage(&p(".claude/fresh")?, b"fresh")?;
        assert_eq!(ws.file_bytes(&p(".claude/old")?), Some(b"old".to_vec()));
        ws.swap(&p(".claude/old")?)?;
        ws.swap(&p(".claude/fresh")?)?;
        assert_eq!(ws.file_bytes(&p(".claude/old")?), Some(b"new".to_vec()));
        assert_eq!(ws.mode(&p(".claude/old")?), Some(0o755));
        assert_eq!(ws.file_bytes(&p(".claude/fresh")?), Some(b"fresh".to_vec()));
        assert_eq!(ws.mode(&p(".claude/fresh")?), Some(FakeWorkspace::NEW_MODE));
        assert!(ws.temps().is_empty());
        Ok(())
    }

    #[test]
    fn swap_through_a_symlink_updates_the_target() -> TestResult {
        let ws = FakeWorkspace::new()
            .file(&p("real")?, b"old")
            .symlink(&p("link")?, &p("real")?);
        ws.stage(&p("link")?, b"new")?;
        assert_eq!(ws.temps(), [p("real")?]);
        ws.swap(&p("link")?)?;
        assert_eq!(ws.file_bytes(&p("real")?), Some(b"new".to_vec()));
        assert_eq!(ws.resolve(&p("link")?)?.link(), Link::Symlink);
        Ok(())
    }

    #[test]
    fn discard_removes_a_temp_and_tolerates_none() -> TestResult {
        let ws = FakeWorkspace::new();
        ws.stage(&p("a")?, b"x")?;
        assert_eq!(ws.temps(), [p("a")?]);
        ws.discard(&p("a")?)?;
        assert!(ws.temps().is_empty());
        ws.discard(&p("a")?)?;
        Ok(())
    }

    #[test]
    fn delete_removes_the_file() -> TestResult {
        let ws = FakeWorkspace::new().file(&p("a")?, b"x");
        ws.delete(&p("a")?)?;
        assert_eq!(
            ws.resolve(&p("a")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::Absent
            }
        );
        let kind = ws.delete(&p("a")?).err().map(|error| error.kind());
        assert_eq!(kind, Some(std::io::ErrorKind::NotFound));
        Ok(())
    }

    #[test]
    fn write_lock_records_the_text() -> TestResult {
        let ws = FakeWorkspace::new();
        assert_eq!(ws.lock_written(), None);
        ws.write_lock("# new\n")?;
        assert_eq!(ws.lock_written(), Some("# new\n".to_owned()));
        assert_eq!(ws.lock_text()?, Some("# new\n".to_owned()));
        Ok(())
    }

    #[test]
    fn fake_records_the_written_project_text() -> TestResult {
        let ws = FakeWorkspace::new();
        assert_eq!(ws.project_written(), None);
        ws.write_project("tools = []\n")?;
        assert_eq!(ws.project_written(), Some("tools = []\n".to_owned()));
        assert_eq!(ws.project_text()?, "tools = []\n");
        let failing = FakeWorkspace::new().fail(WorkspaceCall::WriteProject);
        let error = failing.write_project("x").err().ok_or("a failure")?;
        assert_eq!(error.to_string(), FakeWorkspace::denied().to_string());
        assert_eq!(failing.project_written(), None);
        Ok(())
    }

    #[test]
    fn writing_an_outside_path_is_refused() -> TestResult {
        let ws = FakeWorkspace::new().outside(&p("o")?);
        let refused = [
            ws.stage(&p("o")?, b"x"),
            ws.swap(&p("o")?),
            ws.delete(&p("o")?),
        ];
        for result in refused {
            let error = result.err().ok_or("a refusal")?;
            assert_eq!(error.to_string(), "resolves outside the project");
        }
        assert!(ws.temps().is_empty());
        Ok(())
    }

    #[test]
    fn a_failing_call_returns_denied_and_changes_nothing() -> TestResult {
        let ws = FakeWorkspace::new()
            .file(&p(".claude/a")?, b"old a")
            .file(&p(".claude/b")?, b"old b")
            .fail(WorkspaceCall::Swap(p(".claude/b")?));
        ws.stage(&p(".claude/a")?, b"new a")?;
        ws.stage(&p(".claude/b")?, b"new b")?;
        ws.swap(&p(".claude/a")?)?;
        let error = ws.swap(&p(".claude/b")?).err().ok_or("a failure")?;
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), FakeWorkspace::denied().to_string());
        assert_eq!(ws.temps(), [p(".claude/b")?]);
        assert_eq!(ws.file_bytes(&p(".claude/b")?), Some(b"old b".to_vec()));
        assert_eq!(ws.file_bytes(&p(".claude/a")?), Some(b"new a".to_vec()));
        Ok(())
    }

    #[test]
    fn calls_are_logged_in_order() -> TestResult {
        let ws = FakeWorkspace::new()
            .project("p")
            .fail(WorkspaceCall::LockText);
        let _ = ws.project_text();
        let _ = ws.lock_text();
        ws.stage(&p("a")?, b"x")?;
        ws.resolve(&p("a")?)?;
        ws.swap(&p("a")?)?;
        ws.write_lock("l")?;
        assert_eq!(
            ws.calls(),
            [
                WorkspaceCall::ProjectText,
                WorkspaceCall::LockText,
                WorkspaceCall::Stage(p("a")?),
                WorkspaceCall::Resolve(p("a")?),
                WorkspaceCall::Swap(p("a")?),
                WorkspaceCall::WriteLock,
            ]
        );
        Ok(())
    }

    #[test]
    fn set_file_changes_the_disk_between_calls() -> TestResult {
        let ws = FakeWorkspace::new().file(&p("a")?, b"before");
        let hash_of = |ws: &FakeWorkspace| -> Result<ContentHash, Box<dyn std::error::Error>> {
            match ws.resolve(&p("a")?)? {
                Resolution::Inside {
                    state: DiskState::File(file),
                    ..
                } => Ok(file.hash().clone()),
                other => Err(format!("{other:?}").into()),
            }
        };
        assert_eq!(hash_of(&ws)?, ContentHash::of_bytes(b"before"));
        ws.set_file(&p("a")?, b"after");
        assert_eq!(hash_of(&ws)?, ContentHash::of_bytes(b"after"));
        ws.remove(&p("a")?);
        assert_eq!(ws.file_bytes(&p("a")?), None);
        Ok(())
    }

    #[test]
    fn fake_is_usable_through_a_generic_bound() -> TestResult {
        let ws = FakeWorkspace::new().project("tools = []\n");
        assert_eq!(read_project(&ws)?, "tools = []\n");
        Ok(())
    }
}
