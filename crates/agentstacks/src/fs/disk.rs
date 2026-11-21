//! A [`FileTree`] over the real disk.
//!
//! It is the one place that calls `std::fs`, so the rules in `types` stay testable without a disk
//! and every disk-specific refusal, a symlink or a path leaving the root, is decided here.
//!
//! # Responsibilities
//!
//! - [`StdFileTree`] answers the port's three reads with `std::fs::symlink_metadata`,
//!   `std::fs::read_dir` and `std::fs::read`. It reports a symlink as a symlink and never follows
//!   one: `read_dir` and `read_file` check every component of the path below the root with
//!   `symlink_metadata` just before reading, and refuse a symlink. A path swapped for a symlink
//!   after a walk checked it is therefore refused, not read. The check and the read are two
//!   system calls, so a swap in the instant between them is not prevented.
//! - It reads only below its root: a path that is absolute, or that holds a `..` or a root
//!   component, is refused with [`io::ErrorKind::InvalidInput`] before any disk access.
//!
//! # Non-responsibilities
//!
//! It adds no rules about what a directory may hold and never writes. It is the only `std::fs`
//! caller in the crate.

use std::ffi::OsString;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::types::{EntryKind, FileTree};

/// The refusal for a symlink met on the way to a read, worded without naming a pack or extension.
const SYMLINK_REFUSED: &str = "symlinks are not followed";

/// A directory on the real disk, read through the [`FileTree`] port.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{EntryKind, FileTree};
///
/// let dir = tempfile::tempdir()?;
/// std::fs::write(dir.path().join("pack.toml"), "name = \"core\"\n")?;
///
/// let tree = StdFileTree::new(dir.path());
/// assert_eq!(tree.kind(Path::new("pack.toml"))?, EntryKind::File);
/// assert_eq!(tree.read_text(Path::new("pack.toml"))?, "name = \"core\"\n");
///
/// // A path that could leave the root is refused before any disk access.
/// assert!(tree.read_file(Path::new("../outside")).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct StdFileTree {
    root: PathBuf,
}

impl StdFileTree {
    /// A tree rooted at `root`. The directory is not touched until a read.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::FileTree;
    ///
    /// let tree = StdFileTree::new("packs/core");
    /// assert_eq!(tree.root(), std::path::Path::new("packs/core"));
    /// ```
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `rel` under the root, or the root itself when `rel` is empty.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::InvalidInput`] when `rel` could leave the root: it is absolute or holds a
    /// `..`, a root or a prefix component.
    fn resolve(&self, rel: &Path) -> io::Result<PathBuf> {
        let stays_inside = rel
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
        if !stays_inside {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path must stay inside the tree",
            ));
        }
        Ok(if rel.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.root.join(rel)
        })
    }

    /// Like [`resolve`](Self::resolve), and also refuses when `rel` or any directory above it
    /// below the root is a symlink at this moment. A component that does not exist is left to the
    /// read that follows, which reports it.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::InvalidInput`] when `rel` could leave the root, or when a component is a
    /// symlink, displayed as `symlinks are not followed`.
    fn resolve_unlinked(&self, rel: &Path) -> io::Result<PathBuf> {
        let full = self.resolve(rel)?;
        let mut walked = self.root.clone();
        for part in rel.components() {
            if let Component::Normal(name) = part {
                walked.push(name);
                if std::fs::symlink_metadata(&walked)
                    .is_ok_and(|meta| meta.file_type().is_symlink())
                {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, SYMLINK_REFUSED));
                }
            }
        }
        Ok(full)
    }
}

impl FileTree for StdFileTree {
    fn root(&self) -> &Path {
        &self.root
    }

    fn kind(&self, rel: &Path) -> io::Result<EntryKind> {
        let file_type = std::fs::symlink_metadata(self.resolve(rel)?)?.file_type();
        Ok(if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_file() {
            EntryKind::File
        } else if file_type.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::Other
        })
    }

    fn read_dir(&self, rel: &Path) -> io::Result<Vec<OsString>> {
        std::fs::read_dir(self.resolve_unlinked(rel)?)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect()
    }

    fn read_file(&self, rel: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(self.resolve_unlinked(rel)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{tree_hash, walk};
    use std::error::Error;
    use std::fs;

    type TestResult = Result<(), Box<dyn Error>>;

    const EXAMPLE_HASH: &str =
        "sha256:b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b";

    fn write(dir: &Path, relative: &str, bytes: &[u8]) -> io::Result<()> {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)
    }

    fn sorted_names(tree: &StdFileTree, rel: &str) -> io::Result<Vec<OsString>> {
        let mut names = tree.read_dir(Path::new(rel))?;
        names.sort();
        Ok(names)
    }

    #[test]
    fn root_is_the_directory_given() {
        assert_eq!(
            StdFileTree::new("packs/core").root(),
            Path::new("packs/core")
        );
    }

    #[test]
    fn kind_tells_a_file_from_a_directory() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "d/a.md", b"a\n")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.kind(Path::new("d"))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new("d/a.md"))?, EntryKind::File);
        Ok(())
    }

    #[test]
    fn read_dir_lists_names_below_a_relative_path_and_the_root() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"")?;
        write(temp.path(), "d/b.md", b"")?;
        write(temp.path(), "d/e/c.md", b"")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(sorted_names(&tree, "")?, ["a.md", "d"]);
        assert_eq!(sorted_names(&tree, "d")?, ["b.md", "e"]);
        Ok(())
    }

    #[test]
    fn read_file_returns_the_bytes() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "d/a.md", b"a\r\n")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.read_file(Path::new("d/a.md"))?, b"a\r\n");
        Ok(())
    }

    #[test]
    fn a_missing_path_is_not_found() -> TestResult {
        let temp = tempfile::tempdir()?;
        let tree = StdFileTree::new(temp.path().join("missing"));
        let kinds = [
            tree.kind(Path::new("")).err().map(|e| e.kind()),
            tree.read_dir(Path::new("")).err().map(|e| e.kind()),
            tree.read_file(Path::new("a.md")).err().map(|e| e.kind()),
        ];
        assert_eq!(kinds, [Some(io::ErrorKind::NotFound); 3]);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_reported_and_not_followed() -> TestResult {
        let temp = tempfile::tempdir()?;
        let target = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        std::os::unix::fs::symlink(temp.path().join("a.md"), temp.path().join("link"))?;
        std::os::unix::fs::symlink(target.path(), temp.path().join("sub"))?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.kind(Path::new("link"))?, EntryKind::Symlink);
        assert_eq!(tree.kind(Path::new("sub"))?, EntryKind::Symlink);
        assert_eq!(tree.kind(Path::new("a.md"))?, EntryKind::File);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_file_swapped_for_a_symlink_after_the_walk_is_refused() -> TestResult {
        let temp = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        write(outside.path(), "secret", b"secret\n")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.read_file(Path::new("a.md"))?, b"a\n");

        fs::remove_file(temp.path().join("a.md"))?;
        std::os::unix::fs::symlink(outside.path().join("secret"), temp.path().join("a.md"))?;
        let error = tree.read_file(Path::new("a.md")).err();
        assert_eq!(
            error.as_ref().map(io::Error::kind),
            Some(io::ErrorKind::InvalidInput)
        );
        assert_eq!(
            error.map(|e| e.to_string()),
            Some("symlinks are not followed".to_owned())
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_swapped_for_a_symlink_after_the_walk_is_refused() -> TestResult {
        let temp = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        write(temp.path(), "d/a.md", b"a\n")?;
        write(outside.path(), "a.md", b"outside\n")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.read_dir(Path::new("d"))?, ["a.md"]);

        fs::remove_dir_all(temp.path().join("d"))?;
        std::os::unix::fs::symlink(outside.path(), temp.path().join("d"))?;
        let refused = [
            tree.read_dir(Path::new("d")).err().map(|e| e.to_string()),
            tree.read_file(Path::new("d/a.md"))
                .err()
                .map(|e| e.to_string()),
        ];
        let message = Some("symlinks are not followed".to_owned());
        assert_eq!(refused, [message.clone(), message]);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_root_is_still_read() -> TestResult {
        let temp = tempfile::tempdir()?;
        let real = tempfile::tempdir()?;
        write(real.path(), "a.md", b"a\n")?;
        let link = temp.path().join("root");
        std::os::unix::fs::symlink(real.path(), &link)?;
        assert_eq!(
            StdFileTree::new(&link).read_file(Path::new("a.md"))?,
            b"a\n"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_unix_socket_is_other() -> TestResult {
        let temp = tempfile::tempdir()?;
        let _socket = std::os::unix::net::UnixListener::bind(temp.path().join("sock"))?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.kind(Path::new("sock"))?, EntryKind::Other);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_non_utf8_name_is_listed_as_is() -> TestResult {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let temp = tempfile::tempdir()?;
        let bad = OsStr::from_bytes(&[0x66, 0xFF]);
        fs::write(temp.path().join(bad), b"x")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.read_dir(Path::new(""))?, [bad.to_os_string()]);
        assert_eq!(tree.kind(Path::new(bad))?, EntryKind::File);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_returns_an_io_error() -> TestResult {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        let locked = temp.path().join("a.md");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))?;
        let readable = fs::read(&locked).is_ok();
        let result = StdFileTree::new(temp.path()).read_file(Path::new("a.md"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644))?;
        // A process that ignores file modes (root) can still read the file; nothing to refuse.
        if !readable {
            assert_eq!(
                result.err().map(|e| e.kind()),
                Some(io::ErrorKind::PermissionDenied)
            );
        }
        Ok(())
    }

    #[test]
    fn a_path_that_could_leave_the_root_is_refused_before_any_read() -> TestResult {
        let temp = tempfile::tempdir()?;
        let outside = temp.path().join("outside.md");
        write(temp.path(), "outside.md", b"secret\n")?;
        write(temp.path(), "root/inside.md", b"inside\n")?;
        let tree = StdFileTree::new(temp.path().join("root"));
        let refused = [
            Path::new("../outside.md"),
            Path::new("a/../../outside.md"),
            Path::new(".."),
            outside.as_path(),
            Path::new("/"),
        ];
        for rel in refused {
            let kinds = [
                tree.kind(rel).err().map(|e| e.kind()),
                tree.read_dir(rel).err().map(|e| e.kind()),
                tree.read_file(rel).err().map(|e| e.kind()),
            ];
            assert_eq!(
                kinds,
                [Some(io::ErrorKind::InvalidInput); 3],
                "{}",
                rel.display()
            );
        }
        let error = tree.read_file(Path::new("../outside.md")).err();
        assert_eq!(
            error.map(|e| e.to_string()),
            Some("path must stay inside the tree".to_owned())
        );
        Ok(())
    }

    #[test]
    fn a_path_below_the_root_is_still_read() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "d/a.md", b"a\n")?;
        let tree = StdFileTree::new(temp.path());
        assert_eq!(tree.read_file(Path::new("./d/a.md"))?, b"a\n");
        assert_eq!(tree.kind(Path::new("."))?, EntryKind::Dir);
        assert_eq!(tree.kind(Path::new(""))?, EntryKind::Dir);
        Ok(())
    }

    #[test]
    fn tree_hash_over_the_disk_matches_the_example_tree() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        write(temp.path(), "b/c.lua", b"x\n")?;
        let hash = tree_hash(&StdFileTree::new(temp.path())).map_err(|d| format!("{d:?}"))?;
        assert_eq!(hash.to_string(), EXAMPLE_HASH);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn tree_hash_over_the_disk_ignores_file_modes() -> TestResult {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        write(temp.path(), "b/c.lua", b"x\n")?;
        fs::set_permissions(
            temp.path().join("b/c.lua"),
            fs::Permissions::from_mode(0o755),
        )?;
        let hash = tree_hash(&StdFileTree::new(temp.path())).map_err(|d| format!("{d:?}"))?;
        assert_eq!(hash.to_string(), EXAMPLE_HASH);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn walk_over_the_disk_refuses_a_symlink_at_the_root_path() -> TestResult {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "a.md", b"a\n")?;
        std::os::unix::fs::symlink(temp.path().join("a.md"), temp.path().join("link"))?;
        let (walked, diagnostics) = walk(&StdFileTree::new(temp.path()));
        let messages: Vec<String> = diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(walked.files().len(), 1);
        assert_eq!(
            messages,
            [format!(
                "{}/link: symlinks are not allowed in a pack",
                temp.path().display()
            )]
        );
        Ok(())
    }

    #[test]
    fn walk_over_a_missing_directory_reports_the_platform_error() -> TestResult {
        let temp = tempfile::tempdir()?;
        let missing = temp.path().join("missing");
        let cause = fs::read_dir(&missing).err().map(|e| e.to_string());
        let messages: Vec<String> = walk(&StdFileTree::new(&missing))
            .1
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            messages,
            cause
                .map(|c| format!("{}: {c}", missing.display()))
                .into_iter()
                .collect::<Vec<_>>()
        );
        Ok(())
    }
}
