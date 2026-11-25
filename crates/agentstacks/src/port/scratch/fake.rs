//! An in-memory [`Scratch`] for tests.
//!
//! It lets a flow open directories and create a temporary project without a disk, and records what
//! it was asked.
//!
//! # Responsibilities
//!
//! - [`FakeScratch`] serves a [`FakeFileTree`] for each directory registered with
//!   [`FakeScratch::dir`], labelled with the label the caller passes, and answers `NotFound` for
//!   any other directory. [`Scratch::create`] returns a [`FakeWorkspace`] holding the files, with
//!   `agentstacks.toml` as its project text and the other files as project files, so the workspace
//!   serves any directory among them as a source tree.
//! - [`ScratchCall`] is one entry of the call log that [`FakeScratch::calls`] returns.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests and under the `testing` feature.
//! How the real disk and the temporary directory behave is proven by the tests of the `fs`
//! module.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::port::FakeFileTree;
use crate::port::scratch::seam::Scratch;
use crate::port::workspace::FakeWorkspace;
use crate::types::check::StagedFile;
use crate::types::project::PROJECT_FILE;

/// One call made on a [`FakeScratch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScratchCall {
    /// [`Scratch::open`].
    Open {
        /// The directory to read.
        dir: PathBuf,
        /// The path as the user wrote it.
        label: PathBuf,
    },
    /// [`Scratch::create`] with the files it was given.
    Create(Vec<StagedFile>),
}

/// An in-memory scratch space.
#[derive(Debug, Default)]
pub struct FakeScratch {
    dirs: BTreeMap<PathBuf, FakeFileTree>,
    calls: RefCell<Vec<ScratchCall>>,
}

impl FakeScratch {
    /// A scratch space with no directory registered.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Serves `tree` when `dir` is opened.
    #[must_use]
    pub fn dir(mut self, dir: &Path, tree: FakeFileTree) -> Self {
        self.dirs.insert(dir.to_path_buf(), tree);
        self
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<ScratchCall> {
        self.calls.borrow().clone()
    }
}

impl Scratch for FakeScratch {
    type Tree = FakeFileTree;
    type Project = FakeWorkspace;

    fn open(&self, dir: &Path, label: &Path) -> io::Result<Self::Tree> {
        self.calls.borrow_mut().push(ScratchCall::Open {
            dir: dir.to_path_buf(),
            label: label.to_path_buf(),
        });
        self.dirs
            .get(dir)
            .map(|tree| tree.clone().labelled(label))
            .ok_or_else(FakeFileTree::not_found)
    }

    fn create(&self, files: &[StagedFile]) -> io::Result<Self::Project> {
        self.calls
            .borrow_mut()
            .push(ScratchCall::Create(files.to_vec()));
        let mut project = FakeWorkspace::new();
        for file in files {
            if file.path.as_str() == PROJECT_FILE {
                project = project.project(&String::from_utf8_lossy(&file.bytes));
            } else {
                project = project.file(&file.path, &file.bytes);
            }
        }
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    use crate::port::{FileTree, Workspace};
    use crate::types::op::OpPath;
    use crate::types::project::LocalPath;

    type TestResult = Result<(), Box<dyn Error>>;

    fn staged(path: &str, bytes: &[u8]) -> Result<StagedFile, Box<dyn Error>> {
        Ok(StagedFile {
            path: OpPath::new(path)?,
            bytes: bytes.to_vec(),
        })
    }

    #[test]
    fn fake_scratch_serves_registered_dirs() -> TestResult {
        let scratch = FakeScratch::new().dir(
            Path::new("/work/ext"),
            FakeFileTree::new("elsewhere").file("extension.toml", b"x"),
        );
        let tree = scratch.open(Path::new("/work/ext"), Path::new("ext"))?;
        assert_eq!(tree.root(), Path::new("ext"));
        assert_eq!(tree.read_file(Path::new("extension.toml"))?, b"x");
        let Err(error) = scratch.open(Path::new("/work/other"), Path::new("other")) else {
            return Err("an unregistered dir must not open".into());
        };
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(
            scratch.calls(),
            [
                ScratchCall::Open {
                    dir: PathBuf::from("/work/ext"),
                    label: PathBuf::from("ext"),
                },
                ScratchCall::Open {
                    dir: PathBuf::from("/work/other"),
                    label: PathBuf::from("other"),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn fake_scratch_create_preloads_workspace() -> TestResult {
        let scratch = FakeScratch::new();
        let files = [
            staged("ext/a.lua", b"return {}\n")?,
            staged("agentstacks.toml", b"tools = []\n")?,
        ];
        let project = scratch.create(&files)?;
        assert_eq!(
            project.file_bytes(&OpPath::new("ext/a.lua")?),
            Some(b"return {}\n".to_vec())
        );
        assert_eq!(project.project_text()?, "tools = []\n");
        assert_eq!(scratch.calls(), [ScratchCall::Create(files.to_vec())]);
        Ok(())
    }

    #[test]
    fn fake_scratch_create_serves_ext_and_pack_dirs_as_sources() -> TestResult {
        let scratch = FakeScratch::new();
        let files = [
            staged("ext/mytool/adapter.lua", b"return {}\n")?,
            staged("packs/team/skills/a/SKILL.md", b"x")?,
            staged("agentstacks.toml", b"api = 1\n")?,
        ];
        let project = scratch.create(&files)?;
        let ext = project
            .source_tree(&LocalPath::new("ext/mytool")?)?
            .ok_or("an extension tree")?;
        assert_eq!(ext.root(), Path::new("ext/mytool"));
        assert_eq!(ext.read_file(Path::new("adapter.lua"))?, b"return {}\n");
        let team = project
            .source_tree(&LocalPath::new("packs/team")?)?
            .ok_or("a pack tree")?;
        assert_eq!(team.read_file(Path::new("skills/a/SKILL.md"))?, b"x");
        Ok(())
    }
}
