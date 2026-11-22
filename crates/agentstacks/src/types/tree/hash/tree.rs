//! The deterministic hash of a pack or extension directory.
//!
//! It is separate from the walker because the walk only lists files; deciding what a tree's hash is
//! made of, and how the pieces combine, is a format of its own that the lock pins.
//!
//! # Responsibilities
//!
//! - [`tree_hash()`] walks a [`FileTree`] with the shared walker and hashes every regular file's
//!   path and bytes into one [`ContentHash`]. Dot entries and empty directories contribute
//!   nothing; symlinks, special files and non-UTF-8 paths are refused with the walker's
//!   diagnostics, and every walkable file is still read, so an unreadable file is reported in the
//!   same run. Nothing is excluded: a file a pack manifest excludes is still hashed.
//! - `hash_walked` is the hashing half on its own: it reads each file of a walk the caller already
//!   holds exactly once and hands the caller every file's bytes, so a loader that walks and reads a
//!   directory itself gets the same tree hash without a second walk or a second read.
//! - The hash covers raw bytes: no line-ending normalisation, and file modes are never read.
//!
//! # Non-responsibilities
//!
//! Comparing the hash against the lock belongs to the plan and apply stage that reconciles a
//! lock with the packs on disk, not to the lock's own types.

use std::path::Path;

use crate::port::FileTree;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tree::hash::content::ContentHash;
use crate::types::tree::path::TreePath;
use crate::types::tree::walk::Walked;
use crate::types::tree::walk::walk;

/// Hashes the tree behind `tree`.
///
/// # Errors
///
/// Every walker refusal, and every file that cannot be read, as located diagnostics.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::tree_hash;
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let hash = tree_hash(&StdFileTree::new(pack)).map_err(|refusals| format!("{refusals:?}"))?;
/// assert!(hash.to_string().starts_with("sha256:"));
///
/// // A directory that cannot be listed has no hash, only its refusal.
/// assert!(tree_hash(&StdFileTree::new(format!("{pack}/missing"))).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn tree_hash<T: FileTree>(tree: &T) -> Result<ContentHash, Diagnostics> {
    let (walked, mut diagnostics) = walk(tree);
    match hash_walked(tree, &walked, &mut diagnostics, |_, _| {}) {
        Some(hash) if diagnostics.is_empty() => Ok(hash),
        _ => Err(diagnostics),
    }
}

/// Reads every file of `walked`, the walk of `tree`, exactly once and returns their tree hash,
/// handing each file's path and bytes to `keep` after hashing them.
///
/// A file that cannot be read is pushed onto `diagnostics`, located at its own path under the
/// tree's root, and the result is `None`: a tree with an unread file has no hash. The walk's own
/// refusals are the caller's to report; they do not change the result.
pub(crate) fn hash_walked<T: FileTree>(
    tree: &T,
    walked: &Walked,
    diagnostics: &mut Diagnostics,
    mut keep: impl FnMut(&TreePath, Vec<u8>),
) -> Option<ContentHash> {
    let mut entries = Vec::with_capacity(walked.files().len());
    let mut unread = false;
    for path in walked.files() {
        match tree.read_file(Path::new(path.as_str())) {
            Ok(bytes) => {
                entries.push((path.clone(), ContentHash::of_bytes(&bytes)));
                keep(path, bytes);
            }
            Err(error) => {
                unread = true;
                diagnostics.push(Diagnostic::new(
                    Location::file(tree.root().join(path.as_str())),
                    error,
                ));
            }
        }
    }
    (!unread).then(|| combine(entries))
}

/// Combines per-file hashes into the tree hash without touching the disk.
///
/// The entries are sorted by path bytes. Each contributes its path bytes, `0x00`, the hex of its
/// file hash and `0x0A`; the concatenation is hashed.
fn combine(mut entries: Vec<(TreePath, ContentHash)>) -> ContentHash {
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut bytes = Vec::new();
    for (path, hash) in &entries {
        bytes.extend_from_slice(path.as_str().as_bytes());
        bytes.push(0x00);
        bytes.extend_from_slice(hash.hex().as_bytes());
        bytes.push(0x0A);
    }
    ContentHash::of_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const EXAMPLE_HASH: &str =
        "sha256:b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b";
    const ROOT: &str = "packs/core";

    fn example_tree() -> FakeFileTree {
        FakeFileTree::new(ROOT)
            .file("a.md", b"a\n")
            .file("b/c.lua", b"x\n")
    }

    fn hash_text(tree: &FakeFileTree) -> Result<String, Box<dyn Error>> {
        tree_hash(tree)
            .map(|hash| hash.to_string())
            .map_err(|d| format!("{d:?}").into())
    }

    fn messages(result: Result<ContentHash, Diagnostics>) -> Vec<String> {
        result
            .err()
            .unwrap_or_default()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn entry(path: &str, bytes: &[u8]) -> Option<(TreePath, ContentHash)> {
        TreePath::from_relative(Path::new(path))
            .ok()
            .map(|p| (p, ContentHash::of_bytes(bytes)))
    }

    #[test]
    fn combine_sorts_entries_by_path_bytes() {
        let entries: Vec<_> = [entry("b/c.lua", b"x\n"), entry("a.md", b"a\n")]
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(combine(entries).to_string(), EXAMPLE_HASH);
    }

    #[test]
    fn tree_hash_matches_the_example_tree() -> TestResult {
        assert_eq!(hash_text(&example_tree())?, EXAMPLE_HASH);
        Ok(())
    }

    #[test]
    fn tree_hash_ignores_creation_order() -> TestResult {
        let tree = FakeFileTree::new(ROOT)
            .file("b/c.lua", b"x\n")
            .file("a.md", b"a\n");
        assert_eq!(hash_text(&tree)?, EXAMPLE_HASH);
        Ok(())
    }

    #[test]
    fn tree_hash_ignores_dot_entries() -> TestResult {
        let tree = example_tree()
            .file(".DS_Store", b"junk")
            .file(".git/config", b"junk");
        assert_eq!(hash_text(&tree)?, EXAMPLE_HASH);
        Ok(())
    }

    #[test]
    fn tree_hash_ignores_empty_directories() -> TestResult {
        let tree = example_tree().dir("d").dir("e/f");
        assert_eq!(hash_text(&tree)?, EXAMPLE_HASH);
        Ok(())
    }

    #[test]
    fn tree_hash_keeps_line_endings() -> TestResult {
        let lf = FakeFileTree::new(ROOT).file("a.md", b"a\n");
        let crlf = FakeFileTree::new(ROOT).file("a.md", b"a\r\n");
        assert_ne!(hash_text(&lf)?, hash_text(&crlf)?);
        Ok(())
    }

    #[test]
    fn tree_hash_covers_the_path() -> TestResult {
        let upper = FakeFileTree::new(ROOT).file("A.md", b"a\n");
        let lower = FakeFileTree::new(ROOT).file("a.md", b"a\n");
        assert_ne!(hash_text(&upper)?, hash_text(&lower)?);
        Ok(())
    }

    #[test]
    fn tree_hash_refuses_a_symlink() {
        let tree = example_tree().symlink("link");
        assert_eq!(
            messages(tree_hash(&tree)),
            [format!("{ROOT}/link: symlinks are not allowed in a pack")]
        );
    }

    #[test]
    fn tree_hash_refuses_a_special_file() {
        let tree = example_tree().special("sock");
        assert_eq!(
            messages(tree_hash(&tree)),
            [format!(
                "{ROOT}/sock: only regular files and directories are allowed"
            )]
        );
    }

    #[test]
    fn tree_hash_reports_a_refusal_and_an_unreadable_file_together() {
        let tree = example_tree().symlink("link").unreadable("a.md");
        assert_eq!(
            messages(tree_hash(&tree)),
            [
                format!("{ROOT}/a.md: {}", FakeFileTree::denied()),
                format!("{ROOT}/link: symlinks are not allowed in a pack")
            ]
        );
    }

    #[test]
    fn tree_hash_reports_a_missing_directory_as_io() {
        let tree = FakeFileTree::missing(ROOT);
        assert_eq!(
            messages(tree_hash(&tree)),
            [format!("{ROOT}: {}", FakeFileTree::not_found())]
        );
    }

    #[test]
    fn tree_hash_reports_an_unreadable_file_as_io() {
        let tree = example_tree().unreadable("b/c.lua");
        assert_eq!(
            messages(tree_hash(&tree)),
            [format!("{ROOT}/b/c.lua: {}", FakeFileTree::denied())]
        );
    }

    #[test]
    fn tree_hash_reports_every_unreadable_file() {
        let tree = example_tree().unreadable("a.md").unreadable("b/c.lua");
        assert_eq!(
            messages(tree_hash(&tree)),
            [
                format!("{ROOT}/a.md: {}", FakeFileTree::denied()),
                format!("{ROOT}/b/c.lua: {}", FakeFileTree::denied())
            ]
        );
    }

    #[test]
    fn hash_walked_matches_tree_hash_and_reads_each_file_once() {
        let tree = example_tree();
        let (walked, mut diagnostics) = walk(&tree);
        let mut kept = Vec::new();
        let hash = hash_walked(&tree, &walked, &mut diagnostics, |path, bytes| {
            kept.push((path.as_str().to_owned(), bytes));
        });
        assert!(diagnostics.is_empty());
        assert_eq!(hash.map(|h| h.to_string()).as_deref(), Some(EXAMPLE_HASH));
        assert_eq!(
            kept,
            [
                ("a.md".to_owned(), b"a\n".to_vec()),
                ("b/c.lua".to_owned(), b"x\n".to_vec())
            ]
        );
    }

    #[test]
    fn hash_walked_has_no_hash_when_a_file_cannot_be_read() {
        let tree = example_tree().unreadable("a.md");
        let (walked, mut diagnostics) = walk(&tree);
        let mut kept = Vec::new();
        let hash = hash_walked(&tree, &walked, &mut diagnostics, |path, _| {
            kept.push(path.as_str().to_owned());
        });
        assert!(hash.is_none());
        assert_eq!(kept, ["b/c.lua"]);
        assert_eq!(
            messages(Err(diagnostics)),
            [format!("{ROOT}/a.md: {}", FakeFileTree::denied())]
        );
    }

    #[test]
    fn hash_walked_leaves_walk_refusals_to_the_caller() {
        let tree = example_tree().symlink("link");
        let (walked, mut diagnostics) = walk(&tree);
        let before = diagnostics.len();
        let hash = hash_walked(&tree, &walked, &mut diagnostics, |_, _| {});
        assert_eq!(hash.map(|h| h.to_string()).as_deref(), Some(EXAMPLE_HASH));
        assert_eq!(diagnostics.len(), before);
    }
}
