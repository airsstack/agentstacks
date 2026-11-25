//! The [`Scratch`] port on the real disk.
//!
//! # Responsibilities
//!
//! - [`DiskScratch`] opens a directory as a [`StdFileTree`] that reports the label the caller gave,
//!   and creates a temporary project: a `tempfile` directory holding the files it is given.
//! - [`ScratchProject`] is that project. It answers the `Workspace` port through a
//!   [`StdWorkspace`] over the directory and owns the directory, so dropping it removes the
//!   directory and everything in it.
//!
//! # Non-responsibilities
//!
//! No decisions: which files the project holds and what a refusal says belong to `types`.

use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::fs::disk::StdFileTree;
use crate::fs::workspace::StdWorkspace;
use crate::port::{Scratch, Workspace};
use crate::types::check::StagedFile;
use crate::types::op::OpPath;
use crate::types::plan::Resolution;
use crate::types::project::LocalPath;

/// Directories outside the project and temporary projects, on the real disk.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::DiskScratch;
/// use agentstacks::port::{FileTree, Scratch};
///
/// let dir = tempfile::tempdir()?;
/// std::fs::write(dir.path().join("extension.toml"), "x")?;
///
/// let tree = DiskScratch.open(dir.path(), std::path::Path::new("ext"))?;
/// assert_eq!(tree.root(), std::path::Path::new("ext"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskScratch;

/// A temporary project directory, removed when the value is dropped.
#[derive(Debug)]
pub struct ScratchProject {
    workspace: StdWorkspace,
    dir: TempDir,
}

impl ScratchProject {
    /// The temporary project directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

impl Scratch for DiskScratch {
    type Tree = StdFileTree;
    type Project = ScratchProject;

    fn open(&self, dir: &Path, label: &Path) -> io::Result<Self::Tree> {
        std::fs::metadata(dir)?;
        Ok(StdFileTree::labelled(dir, label))
    }

    fn create(&self, files: &[StagedFile]) -> io::Result<Self::Project> {
        let dir = tempfile::tempdir()?;
        for file in files {
            let target: PathBuf = dir.path().join(file.path.as_str());
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &file.bytes)?;
        }
        let workspace = StdWorkspace::open(dir.path())?;
        Ok(ScratchProject { workspace, dir })
    }
}

impl Workspace for ScratchProject {
    type Tree = <StdWorkspace as Workspace>::Tree;

    fn project_text(&self) -> io::Result<String> {
        self.workspace.project_text()
    }

    fn lock_text(&self) -> io::Result<Option<String>> {
        self.workspace.lock_text()
    }

    fn resolve(&self, path: &OpPath) -> io::Result<Resolution> {
        self.workspace.resolve(path)
    }

    fn source_tree(&self, dir: &LocalPath) -> io::Result<Option<Self::Tree>> {
        self.workspace.source_tree(dir)
    }

    fn stage(&self, path: &OpPath, bytes: &[u8]) -> io::Result<()> {
        self.workspace.stage(path, bytes)
    }

    fn discard(&self, path: &OpPath) -> io::Result<()> {
        self.workspace.discard(path)
    }

    fn swap(&self, path: &OpPath) -> io::Result<()> {
        self.workspace.swap(path)
    }

    fn delete(&self, path: &OpPath) -> io::Result<()> {
        self.workspace.delete(path)
    }

    fn write_project(&self, text: &str) -> io::Result<()> {
        self.workspace.write_project(text)
    }

    fn write_lock(&self, text: &str) -> io::Result<()> {
        self.workspace.write_lock(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    use crate::port::{FakeFileTree, FakeScratch, FileTree};

    type TestResult = Result<(), Box<dyn Error>>;

    fn staged(path: &str, bytes: &[u8]) -> Result<StagedFile, Box<dyn Error>> {
        Ok(StagedFile {
            path: OpPath::new(path)?,
            bytes: bytes.to_vec(),
        })
    }

    #[test]
    fn disk_scratch_contract() -> TestResult {
        // `open` reads a directory like the fake does.
        let outside = tempfile::tempdir()?;
        std::fs::create_dir(outside.path().join("hooks"))?;
        std::fs::write(outside.path().join("extension.toml"), b"x")?;
        std::fs::write(outside.path().join("hooks/a.lua"), b"y")?;
        let fake = FakeScratch::new().dir(
            outside.path(),
            FakeFileTree::new("elsewhere")
                .file("extension.toml", b"x")
                .file("hooks/a.lua", b"y"),
        );
        let real = DiskScratch.open(outside.path(), Path::new("ext"))?;
        let fake = fake.open(outside.path(), Path::new("ext"))?;
        assert_eq!(real.root(), fake.root());
        for rel in ["", "hooks"] {
            let (mut real_dir, mut fake_dir) = (
                real.read_dir(Path::new(rel))?,
                fake.read_dir(Path::new(rel))?,
            );
            real_dir.sort();
            fake_dir.sort();
            assert_eq!(real_dir, fake_dir, "{rel}");
        }
        assert_eq!(
            real.read_file(Path::new("hooks/a.lua"))?,
            fake.read_file(Path::new("hooks/a.lua"))?
        );
        assert_eq!(
            DiskScratch
                .open(&outside.path().join("missing"), Path::new("missing"))
                .err()
                .map(|error| error.kind()),
            Some(io::ErrorKind::NotFound)
        );

        // `create` writes the files, and the project reads them back.
        let project = DiskScratch.create(&[
            staged("agentstacks.toml", b"tools = []\n")?,
            staged("ext/mytool/adapter.lua", b"return {}\n")?,
        ])?;
        let root = project.path().to_path_buf();
        assert_eq!(project.project_text()?, "tools = []\n");
        assert_eq!(project.lock_text()?, None);
        assert_eq!(
            std::fs::read(root.join("ext/mytool/adapter.lua"))?,
            b"return {}\n"
        );
        assert!(
            project
                .resolve(&OpPath::new("ext/mytool/adapter.lua")?)
                .is_ok()
        );
        project.write_lock("lock\n")?;
        assert_eq!(project.lock_text()?, Some("lock\n".to_owned()));

        // Dropping the project removes the directory.
        drop(project);
        assert!(!root.exists());
        Ok(())
    }
}
