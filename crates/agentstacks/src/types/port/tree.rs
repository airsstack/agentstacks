//! The read-only port to one directory on a disk.
//!
//! It is a trait of its own so the rules in `types` stay free of `std::fs`: one implementation reads
//! the real disk and another, in tests, reads memory, and no rule can tell them apart.
//!
//! # Responsibilities
//!
//! - [`FileTree`] is a directory root plus three reads below it: what an entry is, which names a
//!   directory holds, and a file's bytes. Every path it takes is relative to the root.
//! - [`FileTree::read_text`] reads a file's bytes as UTF-8 text, so every reader of a text file
//!   words "not valid UTF-8" the same way.
//!
//! # Non-responsibilities
//!
//! It never writes. [`FileTree::kind`] reports a symlink as the link itself, and an implementation
//! over a real disk refuses to read through one: [`FileTree::read_dir`] and
//! [`FileTree::read_file`] fail when the path, or a directory above it below the root, is a
//! symlink when the read begins. What cannot be guaranteed is a swap made between that check and
//! the read. Applying the dot rule, refusing entries and hashing belong to the logic built on
//! top.

use std::ffi::OsString;
use std::io;
use std::path::Path;

use crate::types::port::kind::EntryKind;

/// A read-only directory tree rooted at one directory.
///
/// An empty `rel` names the root itself. Failures are the platform's own [`io::Error`], so a
/// caller reports them with the platform's wording.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::FileTree;
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let tree = StdFileTree::new(pack);
/// assert_eq!(tree.root(), Path::new(pack));
///
/// let mut names = tree.read_dir(Path::new(""))?;
/// names.sort();
/// assert_eq!(names, ["hooks", "pack.toml", "skills"]);
/// assert!(tree.read_text(Path::new("pack.toml"))?.starts_with("[pack]"));
/// assert!(!tree.read_file(Path::new("skills/a/SKILL.md"))?.is_empty());
///
/// // A path that could leave the root is refused, and never read.
/// assert!(tree.read_dir(Path::new("..")).is_err());
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait FileTree {
    /// The root directory, as it should appear in messages.
    fn root(&self) -> &Path;

    /// What `rel` is, without following a symlink.
    ///
    /// # Errors
    ///
    /// The I/O failure when the entry cannot be inspected.
    fn kind(&self, rel: &Path) -> io::Result<EntryKind>;

    /// The names of the entries directly under the directory `rel`, in any order.
    ///
    /// # Errors
    ///
    /// The I/O failure when the directory cannot be listed, or an [`io::ErrorKind::InvalidInput`]
    /// error when it, or a directory above it, is a symlink.
    fn read_dir(&self, rel: &Path) -> io::Result<Vec<OsString>>;

    /// The bytes of the file `rel`.
    ///
    /// # Errors
    ///
    /// The I/O failure when the file cannot be read, or an [`io::ErrorKind::InvalidInput`] error
    /// when it, or a directory above it, is a symlink.
    fn read_file(&self, rel: &Path) -> io::Result<Vec<u8>>;

    /// The text of the file `rel`.
    ///
    /// # Errors
    ///
    /// The I/O failure when the file cannot be read, or an [`io::ErrorKind::InvalidData`] error
    /// displayed as `not valid UTF-8` when its bytes are not UTF-8.
    fn read_text(&self, rel: &Path) -> io::Result<String> {
        String::from_utf8(self.read_file(rel)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not valid UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::port::FakeFileTree;

    #[test]
    fn read_text_returns_utf8_text() -> io::Result<()> {
        let tree = FakeFileTree::new("r").file("a.md", "a\u{e9}\n".as_bytes());
        assert_eq!(tree.read_text(Path::new("a.md"))?, "a\u{e9}\n");
        Ok(())
    }

    #[test]
    fn read_text_refuses_bytes_that_are_not_utf8() {
        let tree = FakeFileTree::new("r").file("a.md", &[0x66, 0xFF]);
        let error = tree.read_text(Path::new("a.md")).err();
        assert_eq!(
            error.as_ref().map(io::Error::kind),
            Some(io::ErrorKind::InvalidData)
        );
        assert_eq!(
            error.map(|e| e.to_string()),
            Some("not valid UTF-8".to_owned())
        );
    }

    #[test]
    fn read_text_passes_a_read_failure_through() {
        let tree = FakeFileTree::new("r").unreadable("a.md");
        let error = tree.read_text(Path::new("a.md")).err();
        assert_eq!(
            error.map(|e| e.to_string()),
            Some(FakeFileTree::denied().to_string())
        );
    }
}
