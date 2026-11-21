//! What the directory walker found.
//!
//! It is a type of its own so a caller can hold the walk's result, and ask questions of it, apart
//! from the diagnostics the walk reported.
//!
//! # Responsibilities
//!
//! - [`Walked`] holds the regular files of a walked directory as [`TreePath`]s, the dot entries
//!   the walk ignored and the entries it refused, each list sorted by path bytes.
//! - [`Walked::covers`] answers whether a path is, or lies below, a refused entry: the one rule a
//!   caller uses to spare the author a second refusal about a path the walk already refused.
//!
//! # Non-responsibilities
//!
//! Finding the entries and refusing the bad ones belongs to [`walk`](super::walk()).

use std::path::{Path, PathBuf};

use crate::types::tree::path::TreePath;

/// The files, ignored dot entries and refused entries of a walked directory.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::walk;
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let (walked, _) = walk(&StdFileTree::new(pack));
/// assert_eq!(walked.files().len(), 3);
/// assert!(walked.ignored().is_empty());
/// assert!(walked.refused().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walked {
    files: Vec<TreePath>,
    ignored: Vec<PathBuf>,
    refused: Vec<PathBuf>,
}

impl Walked {
    /// Builds a result, sorting every list by path bytes.
    pub(crate) fn new(
        mut files: Vec<TreePath>,
        mut ignored: Vec<PathBuf>,
        mut refused: Vec<PathBuf>,
    ) -> Self {
        files.sort();
        ignored.sort_by(|a, b| by_bytes(a, b));
        refused.sort_by(|a, b| by_bytes(a, b));
        Self {
            files,
            ignored,
            refused,
        }
    }

    /// Every regular file, relative to the walked directory, sorted by path bytes.
    #[must_use]
    pub fn files(&self) -> &[TreePath] {
        &self.files
    }

    /// Every ignored dot entry, relative to the walked directory, sorted by path bytes.
    ///
    /// A dot directory is listed once; nothing beneath it is listed.
    #[must_use]
    pub fn ignored(&self) -> &[PathBuf] {
        &self.ignored
    }

    /// Every entry the walk refused or could not inspect, relative to the walked directory and
    /// sorted by path bytes.
    ///
    /// A refused directory is listed once and not entered, so nothing beneath it appears in
    /// [`Walked::files`]. A caller that finds a file missing can tell a refused entry from an
    /// absent one and spare the author a second, misleading refusal.
    #[must_use]
    pub fn refused(&self) -> &[PathBuf] {
        &self.refused
    }

    /// Whether the walk refused `path` or a directory above it, so an author who was already told
    /// about the refusal needs no second, misleading one about what `path` would have held.
    ///
    /// The empty path stands for the walked directory itself: a directory that could not be
    /// listed covers every path below it. The comparison runs on the raw entries, so a refused
    /// entry whose name is not UTF-8 covers its descendants too.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::walk;
    ///
    /// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
    /// let (walked, _) = walk(&StdFileTree::new(format!("{pack}/missing")));
    /// assert!(walked.covers(Path::new("pack.toml")));
    ///
    /// let (walked, _) = walk(&StdFileTree::new(pack));
    /// assert!(!walked.covers(Path::new("pack.toml")));
    /// ```
    #[must_use]
    pub fn covers(&self, path: &Path) -> bool {
        self.refused.iter().any(|refused| path.starts_with(refused))
    }
}

/// Orders two paths by their bytes.
fn by_bytes(a: &Path, b: &Path) -> std::cmp::Ordering {
    a.as_os_str()
        .as_encoded_bytes()
        .cmp(b.as_os_str().as_encoded_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_path(text: &str) -> Vec<TreePath> {
        TreePath::from_relative(Path::new(text))
            .into_iter()
            .collect()
    }

    #[test]
    fn walked_sorts_files_ignored_and_refused_by_path_bytes() {
        let files: Vec<TreePath> = ["b/c.lua", "a.md"]
            .iter()
            .flat_map(|t| tree_path(t))
            .collect();
        let ignored = vec![PathBuf::from(".git"), PathBuf::from(".DS_Store")];
        let refused = vec![PathBuf::from("y/link"), PathBuf::from("x")];
        let walked = Walked::new(files, ignored, refused);

        let files: Vec<&str> = walked.files().iter().map(TreePath::as_str).collect();
        assert_eq!(files, ["a.md", "b/c.lua"]);
        assert_eq!(
            walked.ignored(),
            [PathBuf::from(".DS_Store"), PathBuf::from(".git")]
        );
        assert_eq!(
            walked.refused(),
            [PathBuf::from("x"), PathBuf::from("y/link")]
        );
    }

    #[test]
    fn covers_matches_a_refused_entry_and_everything_below_it() {
        let walked = Walked::new(
            Vec::new(),
            Vec::new(),
            vec![PathBuf::from("skills/a"), PathBuf::from("pack.toml")],
        );
        assert!(walked.covers(Path::new("pack.toml")));
        assert!(walked.covers(Path::new("skills/a")));
        assert!(walked.covers(Path::new("skills/a/SKILL.md")));
        assert!(!walked.covers(Path::new("skills")));
        assert!(!walked.covers(Path::new("skills/ab/SKILL.md")));
        assert!(!walked.covers(Path::new("agents/a.md")));
    }

    #[test]
    fn covers_treats_a_refused_root_as_covering_every_path() {
        let walked = Walked::new(Vec::new(), Vec::new(), vec![PathBuf::new()]);
        assert!(walked.covers(Path::new("pack.toml")));
        assert!(walked.covers(Path::new("skills/a/SKILL.md")));
    }

    #[cfg(unix)]
    #[test]
    fn covers_sees_through_a_refused_non_utf8_entry() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad = PathBuf::from("skills").join(OsStr::from_bytes(&[0x66, 0xFF]));
        let walked = Walked::new(Vec::new(), Vec::new(), vec![bad.clone()]);
        assert!(walked.covers(&bad.join("SKILL.md")));
        assert!(!walked.covers(Path::new("skills/f")));
    }
}
