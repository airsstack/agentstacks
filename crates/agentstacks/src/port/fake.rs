//! An in-memory [`FileTree`] for tests.
//!
//! It is separate from the real-disk adapter so a rule over the port is tested without touching the
//! disk, and cases a disk cannot be made to produce on demand, such as an unreadable file, are one
//! builder call.
//!
//! # Responsibilities
//!
//! - [`FakeFileTree`] builds a tree of files, directories, symlinks, special entries and entries
//!   that fail with an I/O error, so a rule over the port is tested without a temp directory.
//!   Every builder takes a path relative to the root and creates the directories above it.
//!   Names may be any [`OsStr`](std::ffi::OsStr), so a non-UTF-8 name is built from bytes on Unix.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests and under the `testing` feature.
//! How the real disk reports the same cases is proven by the tests
//! of the `fs` module.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::port::kind::EntryKind;
use crate::port::tree::FileTree;

/// What one path holds in the fake.
#[derive(Debug, Clone)]
enum Node {
    File(Vec<u8>),
    Dir,
    Symlink,
    Special,
    /// A file whose bytes cannot be read.
    Unreadable,
    /// A directory that cannot be listed.
    UnlistableDir,
    /// An entry whose type cannot be inspected.
    Uninspectable,
}

/// An in-memory directory tree rooted at a display path.
#[derive(Debug, Clone)]
pub struct FakeFileTree {
    root: PathBuf,
    /// Keyed by the path relative to the root; the empty path is the root directory.
    nodes: BTreeMap<PathBuf, Node>,
}

impl FakeFileTree {
    /// An empty tree whose root directory exists and shows as `root` in messages.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(PathBuf::new(), Node::Dir);
        Self {
            root: root.into(),
            nodes,
        }
    }

    /// A tree whose root directory does not exist, so listing it fails with "not found".
    pub fn missing(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            nodes: BTreeMap::new(),
        }
    }

    /// Adds a regular file.
    #[must_use]
    pub fn file(self, rel: impl AsRef<Path>, bytes: &[u8]) -> Self {
        self.with(rel.as_ref(), Node::File(bytes.to_vec()))
    }

    /// Adds a directory.
    #[must_use]
    pub fn dir(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::Dir)
    }

    /// Adds a symlink. Its target is not modelled: it is only ever reported as a symlink.
    #[must_use]
    pub fn symlink(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::Symlink)
    }

    /// Adds an entry that is neither a file, a directory nor a symlink.
    #[must_use]
    pub fn special(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::Special)
    }

    /// Adds a regular file whose bytes cannot be read.
    #[must_use]
    pub fn unreadable(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::Unreadable)
    }

    /// Adds a directory that cannot be listed.
    #[must_use]
    pub fn unlistable_dir(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::UnlistableDir)
    }

    /// Adds an entry that shows in its directory but whose type cannot be inspected.
    #[must_use]
    pub fn uninspectable(self, rel: impl AsRef<Path>) -> Self {
        self.with(rel.as_ref(), Node::Uninspectable)
    }

    /// The error every failing read of the fake returns.
    #[must_use]
    pub fn denied() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "permission denied")
    }

    /// The error a read of a path that is not in the tree returns.
    #[must_use]
    pub fn not_found() -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, "no such file or directory")
    }

    fn with(mut self, rel: &Path, node: Node) -> Self {
        let mut parent = rel.parent();
        while let Some(dir) = parent {
            if dir.as_os_str().is_empty() {
                break;
            }
            self.nodes.entry(dir.to_path_buf()).or_insert(Node::Dir);
            parent = dir.parent();
        }
        self.nodes.insert(rel.to_path_buf(), node);
        self
    }

    fn node(&self, rel: &Path) -> io::Result<&Node> {
        self.nodes.get(rel).ok_or_else(Self::not_found)
    }
}

impl FileTree for FakeFileTree {
    fn root(&self) -> &Path {
        &self.root
    }

    fn kind(&self, rel: &Path) -> io::Result<EntryKind> {
        match self.node(rel)? {
            Node::File(_) | Node::Unreadable => Ok(EntryKind::File),
            Node::Dir | Node::UnlistableDir => Ok(EntryKind::Dir),
            Node::Symlink => Ok(EntryKind::Symlink),
            Node::Special => Ok(EntryKind::Other),
            Node::Uninspectable => Err(Self::denied()),
        }
    }

    fn read_dir(&self, rel: &Path) -> io::Result<Vec<OsString>> {
        match self.node(rel)? {
            Node::Dir => Ok(self
                .nodes
                .keys()
                .filter(|path| path.as_os_str() != "" && path.parent() == Some(rel))
                .filter_map(|path| path.file_name().map(OsString::from))
                .collect()),
            Node::UnlistableDir => Err(Self::denied()),
            _ => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "not a directory",
            )),
        }
    }

    fn read_file(&self, rel: &Path) -> io::Result<Vec<u8>> {
        match self.node(rel)? {
            Node::File(bytes) => Ok(bytes.clone()),
            Node::Unreadable => Err(Self::denied()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn names(tree: &FakeFileTree, rel: &str) -> io::Result<Vec<String>> {
        let mut names: Vec<String> = tree
            .read_dir(Path::new(rel))?
            .iter()
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    #[test]
    fn the_root_is_the_display_path() {
        assert_eq!(
            FakeFileTree::new("packs/core").root(),
            Path::new("packs/core")
        );
    }

    #[test]
    fn a_file_creates_the_directories_above_it() -> TestResult {
        let tree = FakeFileTree::new("r").file("a/b/c.md", b"x");
        assert_eq!(tree.kind(Path::new("a"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new("a/b"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new("a/b/c.md"))?, EntryKind::File);
        assert_eq!(tree.read_file(Path::new("a/b/c.md"))?, b"x");
        Ok(())
    }

    #[test]
    fn read_dir_lists_direct_children_only() -> TestResult {
        let tree = FakeFileTree::new("r")
            .file("a.md", b"")
            .file("d/e.md", b"")
            .file("d/f/g.md", b"");
        assert_eq!(names(&tree, "")?, ["a.md", "d"]);
        assert_eq!(names(&tree, "d")?, ["e.md", "f"]);
        Ok(())
    }

    #[test]
    fn each_builder_reports_its_kind() -> TestResult {
        let tree = FakeFileTree::new("r")
            .dir("d")
            .symlink("l")
            .special("s")
            .unreadable("u")
            .unlistable_dir("x");
        assert_eq!(tree.kind(Path::new("d"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new("l"))?, EntryKind::Symlink);
        assert_eq!(tree.kind(Path::new("s"))?, EntryKind::Other);
        assert_eq!(tree.kind(Path::new("u"))?, EntryKind::File);
        assert_eq!(tree.kind(Path::new("x"))?, EntryKind::Dir);
        Ok(())
    }

    #[test]
    fn the_failing_entries_return_an_io_error() {
        let tree = FakeFileTree::new("r")
            .unreadable("u")
            .unlistable_dir("x")
            .uninspectable("i");
        let kinds = [
            tree.read_file(Path::new("u")).err().map(|e| e.kind()),
            tree.read_dir(Path::new("x")).err().map(|e| e.kind()),
            tree.kind(Path::new("i")).err().map(|e| e.kind()),
        ];
        assert_eq!(kinds, [Some(io::ErrorKind::PermissionDenied); 3]);
    }

    #[test]
    fn a_path_that_is_not_in_the_tree_is_not_found() {
        let tree = FakeFileTree::new("r");
        let kind = tree.kind(Path::new("nope")).err().map(|e| e.kind());
        assert_eq!(kind, Some(io::ErrorKind::NotFound));
    }

    #[test]
    fn a_missing_root_cannot_be_listed() {
        let tree = FakeFileTree::missing("gone");
        let kind = tree.read_dir(Path::new("")).err().map(|e| e.kind());
        assert_eq!(kind, Some(io::ErrorKind::NotFound));
    }

    #[test]
    fn a_file_is_not_a_directory_and_a_directory_is_not_a_file() {
        let tree = FakeFileTree::new("r").file("a.md", b"").dir("d");
        assert!(tree.read_dir(Path::new("a.md")).is_err());
        assert!(tree.read_file(Path::new("d")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_name_is_listed() -> TestResult {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad = OsStr::from_bytes(&[0x66, 0xFF]);
        let tree = FakeFileTree::new("r").file(bad, b"");
        assert_eq!(tree.read_dir(Path::new(""))?, [bad.to_os_string()]);
        Ok(())
    }
}
