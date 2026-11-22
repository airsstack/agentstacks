//! The facts about one path on disk that a step is decided on.
//!
//! # Responsibilities
//!
//! - [`DiskFile`] is a regular file's bytes with their hash and permission mode; the hash is
//!   computed from the bytes, so the two cannot disagree.
//! - [`DiskState`] is what is at a path: nothing, a regular file, or something else.
//! - [`Link`] says whether the path itself is a symlink.
//! - [`Resolution`] is where the path leads: inside the project with a [`DiskState`], or outside.
//!
//! # Non-responsibilities
//!
//! Reading the disk, following symlinks and checking the project boundary belong to the
//! workspace adapter that produces these values.

use crate::types::tree::ContentHash;

/// A regular file as found on disk.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ContentHash;
/// use agentstacks::types::plan::DiskFile;
///
/// let file = DiskFile::new(b"a\n".to_vec(), 0o644);
/// assert_eq!(file.hash(), &ContentHash::of_bytes(b"a\n"));
/// assert_eq!(file.mode(), 0o644);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskFile {
    bytes: Vec<u8>,
    hash: ContentHash,
    mode: u32,
}

impl DiskFile {
    /// A file holding `bytes` with permission `mode`.
    #[must_use]
    pub fn new(bytes: Vec<u8>, mode: u32) -> Self {
        let hash = ContentHash::of_bytes(&bytes);
        Self { bytes, hash, mode }
    }

    /// The bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The hash of the bytes.
    #[must_use]
    pub const fn hash(&self) -> &ContentHash {
        &self.hash
    }

    /// The permission mode.
    #[must_use]
    pub const fn mode(&self) -> u32 {
        self.mode
    }
}

/// What is at a path, after following any symlink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskState {
    /// Nothing is there.
    Absent,
    /// A regular file.
    File(DiskFile),
    /// A directory, a special file, or a symlink that leads nowhere.
    NotRegular,
}

/// Whether the path itself is a symlink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// The path is not a symlink.
    Direct,
    /// The path is a symlink.
    Symlink,
}

/// Where a path leads.
///
/// A dangling symlink is `Inside` with [`Link::Symlink`] and [`DiskState::NotRegular`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The canonical target is inside the project.
    Inside {
        /// Whether the path itself is a symlink.
        link: Link,
        /// The state of the canonical target.
        state: DiskState,
    },
    /// The canonical target is outside the project.
    Outside {
        /// Whether the path itself is a symlink.
        link: Link,
    },
}

impl Resolution {
    /// Whether the path itself is a symlink.
    #[must_use]
    pub const fn link(&self) -> Link {
        match self {
            Self::Inside { link, .. } | Self::Outside { link } => *link,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_file_hash_matches_its_bytes() {
        let file = DiskFile::new(b"a\n".to_vec(), 0o640);
        assert_eq!(file.hash(), &ContentHash::of_bytes(b"a\n"));
        assert_eq!(file.bytes(), b"a\n");
        assert_eq!(file.mode(), 0o640);
    }

    #[test]
    fn resolution_reports_its_link() {
        let inside = Resolution::Inside {
            link: Link::Symlink,
            state: DiskState::Absent,
        };
        assert_eq!(inside.link(), Link::Symlink);
        assert_eq!(
            Resolution::Outside { link: Link::Direct }.link(),
            Link::Direct
        );
    }
}
