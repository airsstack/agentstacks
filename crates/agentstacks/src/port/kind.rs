//! What a directory entry is, read without following a symlink.
//!
//! It is separate from [`FileTree`](super::tree::FileTree) so a reader can match on the four
//! kinds without importing the port, and an implementation of the port has one shared answer type.
//!
//! # Responsibilities
//!
//! - [`EntryKind`] names the four things an entry can be: a regular file, a directory, a symlink,
//!   or anything else (a socket, a device, a named pipe).

/// What a directory entry is. A symlink is reported as [`EntryKind::Symlink`], never as what it
/// points at.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::port::{EntryKind, FileTree};
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let tree = StdFileTree::new(pack);
/// assert_eq!(tree.kind(Path::new("pack.toml"))?, EntryKind::File);
/// assert_eq!(tree.kind(Path::new("skills"))?, EntryKind::Dir);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link.
    Symlink,
    /// Anything that is not a regular file, a directory or a symlink.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_are_distinct() {
        let kinds = [
            EntryKind::File,
            EntryKind::Dir,
            EntryKind::Symlink,
            EntryKind::Other,
        ];
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                assert_eq!(a == b, i == j);
            }
        }
    }
}
