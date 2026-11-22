//! The files of one commit, kept in memory and served as a `FileTree`.
//!
//! # Responsibilities
//!
//! - [`Snapshot`] is the tree under a source's `subdir` (or the whole commit), read once. It is
//!   built from the `ls-tree -r -z --full-tree` listing, and the bytes of the regular files that
//!   the caller reads for it. Every path loses the `<subdir>/` prefix, so the source's root is the
//!   `subdir`.
//! - A record's kind comes from its mode: a regular file is a file, a symlink a symlink, a gitlink
//!   or any other mode is [`EntryKind::Other`]. Every ancestor of a record is a directory and the
//!   empty path is the root.
//! - [`Snapshot`] refuses a `subdir` that is missing or is a file, with the refusals of
//!   `types::resolve`.
//!
//! # Non-responsibilities
//!
//! Running git and reading the blobs belong to the caller; whether a symlink or a special entry
//! is allowed in a pack belongs to the tree walk that reads this tree.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::port::{EntryKind, FileTree};
use crate::types::project::{Pin, SubPath};
use crate::types::resolve::{
    FetchError, GitStep, check_subdir, parse_ls_tree, root_label, unreadable,
};

/// What one path of the snapshot holds.
#[derive(Debug)]
enum Node {
    File(Vec<u8>),
    /// A directory and the names directly under it.
    Dir(BTreeSet<OsString>),
    Symlink,
    Other,
}

/// The files of one commit under a `subdir`, in memory.
#[derive(Debug)]
pub struct Snapshot {
    root: PathBuf,
    /// Keyed by the path below the root; the empty path is the root directory.
    nodes: BTreeMap<PathBuf, Node>,
}

impl Snapshot {
    /// Builds the snapshot from `listing`, the standard output of `ls-tree -r -z --full-tree`.
    ///
    /// `at` is the pin the source names. `read` gets the object ids of the regular files and
    /// answers their bytes in the same order.
    ///
    /// # Errors
    ///
    /// - [`FetchError::GitUnreadable`] for `ls-tree` when a record cannot be read.
    /// - [`FetchError::SubdirNotFound`] when a `subdir` lists nothing, and
    ///   [`FetchError::SubdirIsFile`] when it is one file.
    /// - Whatever `read` returns.
    pub(super) fn from_listing<F>(
        listing: &[u8],
        subdir: Option<&SubPath>,
        at: &Pin,
        read: F,
    ) -> Result<Self, FetchError>
    where
        F: FnOnce(&[&str]) -> Result<Vec<Vec<u8>>, FetchError>,
    {
        let records = parse_ls_tree(listing)?;
        if let Some(subdir) = subdir {
            check_subdir(&records, subdir, at)?;
        }
        let entries: Vec<_> = records
            .iter()
            .filter_map(|record| record.relative(subdir).map(|path| (record, path)))
            .collect();
        let ids: Vec<&str> = entries
            .iter()
            .filter(|(record, _)| record.kind() == EntryKind::File)
            .map(|(record, _)| record.oid.as_str())
            .collect();
        let mut contents = read(&ids)?.into_iter();

        let mut nodes = BTreeMap::new();
        nodes.insert(PathBuf::new(), Node::Dir(BTreeSet::new()));
        let mut snapshot = Self {
            root: root_label(subdir),
            nodes,
        };
        for (record, path) in entries {
            let node = match record.kind() {
                EntryKind::File => Node::File(
                    contents
                        .next()
                        .ok_or_else(|| unreadable(GitStep::CatFile, b""))?,
                ),
                EntryKind::Symlink => Node::Symlink,
                EntryKind::Dir | EntryKind::Other => Node::Other,
            };
            snapshot.insert(path_of(path), node);
        }
        Ok(snapshot)
    }

    /// Adds `node` at `path`, and every directory above it.
    fn insert(&mut self, path: PathBuf, node: Node) {
        let mut current = path.clone();
        self.nodes.insert(path, node);
        while let Some(parent) = current.parent().map(Path::to_path_buf) {
            let Some(name) = current.file_name().map(OsString::from) else {
                break;
            };
            let dir = self
                .nodes
                .entry(parent.clone())
                .or_insert_with(|| Node::Dir(BTreeSet::new()));
            if let Node::Dir(names) = dir
                && !names.insert(name)
            {
                break;
            }
            current = parent;
        }
    }

    fn node(&self, rel: &Path) -> io::Result<&Node> {
        self.nodes
            .get(rel)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such file or directory"))
    }
}

/// The path of the bytes git printed. On Unix they are the name as it is on disk, so a name that is
/// not UTF-8 reaches the tree walk, which refuses it.
fn path_of(bytes: Vec<u8>) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
    }
}

impl FileTree for Snapshot {
    fn root(&self) -> &Path {
        &self.root
    }

    fn kind(&self, rel: &Path) -> io::Result<EntryKind> {
        Ok(match self.node(rel)? {
            Node::File(_) => EntryKind::File,
            Node::Dir(_) => EntryKind::Dir,
            Node::Symlink => EntryKind::Symlink,
            Node::Other => EntryKind::Other,
        })
    }

    fn read_dir(&self, rel: &Path) -> io::Result<Vec<OsString>> {
        match self.node(rel)? {
            Node::Dir(names) => Ok(names.iter().cloned().collect()),
            Node::Symlink => Err(symlink_refused()),
            Node::File(_) | Node::Other => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "not a directory",
            )),
        }
    }

    fn read_file(&self, rel: &Path) -> io::Result<Vec<u8>> {
        match self.node(rel)? {
            Node::File(bytes) => Ok(bytes.clone()),
            Node::Symlink => Err(symlink_refused()),
            Node::Dir(_) | Node::Other => {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"))
            }
        }
    }
}

/// The refusal to read through a symlink, worded as the real-disk tree words it.
fn symlink_refused() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "symlinks are not followed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;
    use crate::process::git::testing::TestResult;
    use crate::types::project::GitTag;
    use crate::types::tree::tree_hash;

    const OID: &str = "136c05e0d0290e2dc7eb89bceaa27b5f011e780a";

    /// One listing record.
    fn record(mode: &str, object: &str, path: &str) -> String {
        format!("{mode} {object} {OID}\t{path}\0")
    }

    /// The listing of the `packs/core` directory of the test repository.
    fn core_listing() -> Vec<u8> {
        [
            record("100644", "blob", "packs/core/crlf.md"),
            record("120000", "blob", "packs/core/link"),
            record("100644", "blob", "packs/core/nested/deeper/file.md"),
            record("100644", "blob", "packs/core/pack.toml"),
            record("100755", "blob", "packs/core/run.sh"),
            record("160000", "commit", "packs/core/sub"),
        ]
        .concat()
        .into_bytes()
    }

    fn pin() -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Tag(GitTag::new("packs-v0.1.0")?))
    }

    /// Reads every id as the bytes `<id>`, so a test sees which file got which content.
    #[expect(clippy::unnecessary_wraps, reason = "the shape `from_listing` takes")]
    fn echo(ids: &[&str]) -> Result<Vec<Vec<u8>>, FetchError> {
        Ok(ids.iter().map(|id| id.as_bytes().to_vec()).collect())
    }

    fn core() -> Result<Snapshot, Box<dyn std::error::Error>> {
        let subdir = SubPath::new("packs/core")?;
        Ok(Snapshot::from_listing(
            &core_listing(),
            Some(&subdir),
            &pin()?,
            echo,
        )?)
    }

    fn names(tree: &Snapshot, rel: &str) -> io::Result<Vec<String>> {
        Ok(tree
            .read_dir(Path::new(rel))?
            .iter()
            .map(|name| name.to_string_lossy().into_owned())
            .collect())
    }

    #[test]
    fn snapshot_strips_the_subdir_prefix() -> TestResult {
        let tree = core()?;
        assert_eq!(tree.read_file(Path::new("pack.toml"))?, OID.as_bytes());
        assert_eq!(
            tree.read_file(Path::new("nested/deeper/file.md"))?,
            OID.as_bytes()
        );
        assert!(tree.kind(Path::new("packs")).is_err());
        Ok(())
    }

    #[test]
    fn snapshot_synthesises_directories() -> TestResult {
        let tree = core()?;
        assert_eq!(tree.kind(Path::new("nested"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new("nested/deeper"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new(""))?, EntryKind::Dir);
        assert_eq!(
            names(&tree, "")?,
            ["crlf.md", "link", "nested", "pack.toml", "run.sh", "sub"]
        );
        assert_eq!(names(&tree, "nested")?, ["deeper"]);
        Ok(())
    }

    #[test]
    fn snapshot_maps_symlink_and_gitlink_kinds() -> TestResult {
        let tree = core()?;
        assert_eq!(tree.kind(Path::new("link"))?, EntryKind::Symlink);
        assert_eq!(tree.kind(Path::new("sub"))?, EntryKind::Other);
        assert_eq!(tree.kind(Path::new("run.sh"))?, EntryKind::File);
        Ok(())
    }

    #[test]
    fn snapshot_refuses_to_read_a_symlink() -> TestResult {
        let error = core()?
            .read_file(Path::new("link"))
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "symlinks are not followed");
        Ok(())
    }

    #[test]
    fn snapshot_answers_the_wrong_kind_like_the_fake() -> TestResult {
        let tree = core()?;
        let kinds = [
            tree.read_file(Path::new("nested")).err().map(|e| e.kind()),
            tree.read_file(Path::new("sub")).err().map(|e| e.kind()),
            tree.read_dir(Path::new("pack.toml"))
                .err()
                .map(|e| e.kind()),
            tree.read_dir(Path::new("link")).err().map(|e| e.kind()),
            tree.read_file(Path::new("nope")).err().map(|e| e.kind()),
        ];
        assert_eq!(
            kinds,
            [
                Some(io::ErrorKind::InvalidInput),
                Some(io::ErrorKind::InvalidInput),
                Some(io::ErrorKind::NotADirectory),
                Some(io::ErrorKind::InvalidInput),
                Some(io::ErrorKind::NotFound),
            ]
        );
        Ok(())
    }

    #[test]
    fn only_regular_files_are_read() -> TestResult {
        let subdir = SubPath::new("packs/core")?;
        let mut asked = Vec::new();
        let tree = Snapshot::from_listing(&core_listing(), Some(&subdir), &pin()?, |ids| {
            asked.extend(ids.iter().map(|id| (*id).to_owned()));
            echo(ids)
        })?;
        assert_eq!(asked.len(), 4);
        assert_eq!(tree.kind(Path::new("run.sh"))?, EntryKind::File);
        Ok(())
    }

    #[test]
    fn empty_listing_with_subdir_is_not_found() -> TestResult {
        let subdir = SubPath::new("packs/nope")?;
        let shown = Snapshot::from_listing(b"", Some(&subdir), &pin()?, echo)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown.as_deref(),
            Some("\"packs/nope\" not found at packs-v0.1.0")
        );
        Ok(())
    }

    #[test]
    fn single_blob_equal_to_subdir_is_a_file() -> TestResult {
        let subdir = SubPath::new("packs/core/pack.toml")?;
        let listing = record("100644", "blob", "packs/core/pack.toml");
        let shown = Snapshot::from_listing(listing.as_bytes(), Some(&subdir), &pin()?, echo)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown.as_deref(),
            Some("\"packs/core/pack.toml\" is a file, not a directory")
        );
        Ok(())
    }

    #[test]
    fn subdir_is_quoted_as_written() -> TestResult {
        let subdir = SubPath::new("./packs//nope")?;
        let shown = Snapshot::from_listing(b"", Some(&subdir), &pin()?, echo)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown.as_deref(),
            Some("\"./packs//nope\" not found at packs-v0.1.0")
        );
        Ok(())
    }

    #[test]
    fn an_unreadable_listing_is_a_failure() -> TestResult {
        let shown = Snapshot::from_listing(b"no tab here\0", None, &pin()?, echo)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            shown.as_deref(),
            Some("git ls-tree printed output agentstacks cannot read: \"no tab here\"")
        );
        Ok(())
    }

    #[test]
    fn an_unknown_mode_is_other() -> TestResult {
        let listing = record("100664", "blob", "odd");
        let tree = Snapshot::from_listing(listing.as_bytes(), None, &pin()?, echo)?;
        assert_eq!(tree.kind(Path::new("odd"))?, EntryKind::Other);
        Ok(())
    }

    #[test]
    fn root_label_is_the_subdir_as_written_or_dot() -> TestResult {
        assert_eq!(core()?.root(), Path::new("packs/core"));
        let whole = Snapshot::from_listing(&core_listing(), None, &pin()?, echo)?;
        assert_eq!(whole.root(), Path::new("."));
        assert_eq!(
            whole.read_file(Path::new("packs/core/pack.toml"))?,
            OID.as_bytes()
        );
        Ok(())
    }

    #[test]
    fn a_listing_without_a_subdir_still_has_a_root() -> TestResult {
        let tree = Snapshot::from_listing(b"", None, &pin()?, echo)?;
        assert!(names(&tree, "")?.is_empty());
        Ok(())
    }

    #[test]
    fn snapshot_hashes_like_the_same_fake_tree() -> TestResult {
        let listing = [
            record("100644", "blob", "packs/core/nested/deeper/file.md"),
            record("100644", "blob", "packs/core/pack.toml"),
        ]
        .concat();
        let subdir = SubPath::new("packs/core")?;
        let snapshot = Snapshot::from_listing(listing.as_bytes(), Some(&subdir), &pin()?, echo)?;
        let fake = FakeFileTree::new("packs/core")
            .file("nested/deeper/file.md", OID.as_bytes())
            .file("pack.toml", OID.as_bytes());
        let (snapshot, fake) = (
            tree_hash(&snapshot).map_err(|refusals| format!("{refusals:?}"))?,
            tree_hash(&fake).map_err(|refusals| format!("{refusals:?}"))?,
        );
        assert_eq!(snapshot, fake);
        Ok(())
    }
}
