//! The one directory walker pack loading, extension loading and the tree hash share.
//!
//! The directory rules are applied once, here, so every caller refuses the same entries with the
//! same words. Pack loading, extension loading and the tree hash must see exactly the same files,
//! which is why none of them owns a walker of its own.
//!
//! # Responsibilities
//!
//! - [`walk()`] lists the regular files under a [`FileTree`], the dot entries it ignored and the
//!   entries it refused. It reads the tree only through that port. For each entry, in this order:
//!   1. A name that starts with `.` is a dot entry. It is ignored whatever it is: a symlink, a
//!      special file or a non-UTF-8 name is never refused, and a dot directory is not entered.
//!   2. A symlink is refused and never followed.
//!   3. An entry that is neither a regular file nor a directory is refused.
//!   4. A path that is not valid UTF-8 is refused.
//!   5. A directory is entered and not listed itself, so an empty directory contributes nothing.
//! - A refused entry gets one refusal, and a refused directory is not entered. An I/O failure is a
//!   [`FormatError::Io`](crate::types::FormatError::Io) at the path that failed. The walk goes on
//!   past every failure: it returns the files it could list together with every refusal, so a
//!   caller can keep checking the walkable files, and it lists the refused entries in [`Walked`]
//!   so a caller can tell a refused entry from an absent one.
//!
//! # Non-responsibilities
//!
//! Reading file contents belongs to the callers and hashing them to
//! [`tree_hash`](crate::types::tree_hash()); judging a file's place in a pack belongs to the pack loader.
//! The symlink refusal carries the pack wording; the extension loader rewords it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::types::port::EntryKind;
use crate::types::port::FileTree;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tree::path::TreePath;
use crate::types::tree::walk::error::WalkError;
use crate::types::tree::walk::outcome::Walked;

/// Walks the tree under the format's directory rules.
///
/// Returns every file it could list together with every refusal it met. A refusal does not stop
/// the walk, so the files beside and below the refused entries are still listed. The
/// [`Diagnostics`] are empty when nothing was refused.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::walk;
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let (walked, refusals) = walk(&StdFileTree::new(pack));
/// assert!(refusals.is_empty());
/// assert!(walked.files().iter().any(|file| file.as_str() == "pack.toml"));
///
/// // A directory that cannot be listed is refused, and nothing is listed.
/// let (walked, refusals) = walk(&StdFileTree::new(format!("{pack}/missing")));
/// assert_eq!(refusals.len(), 1);
/// assert!(walked.files().is_empty());
/// ```
pub fn walk<T: FileTree>(tree: &T) -> (Walked, Diagnostics) {
    let mut files = Vec::new();
    let mut ignored = Vec::new();
    let mut refused = Vec::new();
    let mut diagnostics = Diagnostics::new();
    let mut pending = vec![PathBuf::new()];

    while let Some(relative) = pending.pop() {
        let names = match tree.read_dir(&relative) {
            Ok(names) => names,
            Err(error) => {
                diagnostics.push(Diagnostic::new(
                    Location::file(joined(tree.root(), &relative)),
                    error,
                ));
                refused.push(relative);
                continue;
            }
        };
        for name in names {
            let entry_relative = relative.join(&name);
            if is_dot_entry(&name) {
                ignored.push(entry_relative);
                continue;
            }
            match classify(tree, &entry_relative) {
                Ok(Entry::File(path)) => files.push(path),
                Ok(Entry::Directory) => pending.push(entry_relative),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    refused.push(entry_relative);
                }
            }
        }
    }
    (Walked::new(files, ignored, refused), diagnostics)
}

/// What a non-dot entry turned out to be.
enum Entry {
    File(TreePath),
    Directory,
}

/// Refuses a symlink, then a special file, then a non-UTF-8 path; otherwise says what the entry
/// is. The entry's own type is read, so a symlink is never followed.
fn classify<T: FileTree>(tree: &T, relative: &Path) -> Result<Entry, Diagnostic> {
    let located = || Location::file(joined(tree.root(), relative));
    let kind = tree
        .kind(relative)
        .map_err(|error| Diagnostic::new(located(), error))?;
    match kind {
        EntryKind::Symlink => Err(Diagnostic::new(located(), WalkError::Symlink)),
        EntryKind::Other => Err(Diagnostic::new(located(), WalkError::SpecialFile)),
        EntryKind::Dir => located_tree_path(tree.root(), relative).map(|_| Entry::Directory),
        EntryKind::File => located_tree_path(tree.root(), relative).map(Entry::File),
    }
}

/// `dir` joined with `relative`, or `dir` itself when `relative` is empty.
fn joined(dir: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        dir.to_path_buf()
    } else {
        dir.join(relative)
    }
}

/// Whether `name` is a dot entry, decided on its first byte so it never needs UTF-8.
fn is_dot_entry(name: &OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

/// The [`TreePath`] of `relative`, or the diagnostic located at `dir` joined with it.
fn located_tree_path(dir: &Path, relative: &Path) -> Result<TreePath, Diagnostic> {
    TreePath::from_relative(relative)
        .map_err(|error| Diagnostic::new(Location::file(joined(dir, relative)), error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FormatError;
    use crate::types::port::FakeFileTree;

    const SYMLINK: &str = "symlinks are not allowed in a pack";
    const SPECIAL: &str = "only regular files and directories are allowed";
    const ROOT: &str = "packs/core";

    fn tree() -> FakeFileTree {
        FakeFileTree::new(ROOT)
    }

    fn files(walked: &Walked) -> Vec<&str> {
        walked.files().iter().map(TreePath::as_str).collect()
    }

    fn ignored(walked: &Walked) -> Vec<String> {
        walked
            .ignored()
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }

    fn refused(walked: &Walked) -> Vec<String> {
        walked
            .refused()
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }

    fn messages(diagnostics: Diagnostics) -> Vec<String> {
        diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn is_dot_entry_reads_the_first_byte() {
        use std::os::unix::ffi::OsStrExt;

        assert!(is_dot_entry(OsStr::new(".DS_Store")));
        assert!(is_dot_entry(OsStr::from_bytes(&[0x2E, 0xFF])));
        assert!(!is_dot_entry(OsStr::new("a.md")));
        assert!(!is_dot_entry(OsStr::new("a.")));
        assert!(!is_dot_entry(OsStr::from_bytes(&[0x66, 0xFF])));
    }

    #[test]
    fn walk_lists_files_sorted_with_slash_separators() {
        let tree = tree().file("b/c.lua", b"x\n").file("a.md", b"a\n");
        let (walked, diagnostics) = walk(&tree);
        assert!(diagnostics.is_empty());
        assert_eq!(files(&walked), ["a.md", "b/c.lua"]);
        assert!(walked.ignored().is_empty());
        assert!(walked.refused().is_empty());
    }

    #[test]
    fn walk_ignores_empty_directories() {
        let tree = tree()
            .file("a.md", b"a\n")
            .dir("d")
            .dir("e/f")
            .dir("skills/a");
        let (walked, diagnostics) = walk(&tree);
        assert!(diagnostics.is_empty());
        assert_eq!(files(&walked), ["a.md"]);
        assert!(walked.ignored().is_empty());
    }

    #[test]
    fn walk_ignores_dot_entries_and_lists_each_once() {
        let tree = tree()
            .file("a.md", b"a\n")
            .file(".DS_Store", b"")
            .file(".git/config", b"")
            .file("skills/x/.hidden", b"");
        let (walked, diagnostics) = walk(&tree);
        assert!(diagnostics.is_empty());
        assert_eq!(files(&walked), ["a.md"]);
        assert_eq!(ignored(&walked), [".DS_Store", ".git", "skills/x/.hidden"]);
    }

    #[test]
    fn walk_refuses_a_symlinked_file() {
        let tree = tree().file("a.md", b"a\n").symlink("link");
        let (_, diagnostics) = walk(&tree);
        assert_eq!(messages(diagnostics), [format!("{ROOT}/link: {SYMLINK}")]);
    }

    #[test]
    fn walk_refuses_a_symlinked_directory_without_following_it() {
        let tree = tree().symlink("sub");
        let (_, diagnostics) = walk(&tree);
        assert_eq!(messages(diagnostics), [format!("{ROOT}/sub: {SYMLINK}")]);
    }

    #[test]
    fn walk_reports_every_symlink() {
        let tree = tree().file("a.md", b"a\n").symlink("l1").symlink("x/l2");
        let (_, diagnostics) = walk(&tree);
        assert_eq!(
            messages(diagnostics),
            [
                format!("{ROOT}/l1: {SYMLINK}"),
                format!("{ROOT}/x/l2: {SYMLINK}")
            ]
        );
    }

    #[test]
    fn walk_refuses_a_special_file() {
        let tree = tree().file("a.md", b"a\n").special("sock");
        let (_, diagnostics) = walk(&tree);
        assert_eq!(messages(diagnostics), [format!("{ROOT}/sock: {SPECIAL}")]);
    }

    #[test]
    fn walk_lists_the_walkable_files_beside_a_refusal() {
        let tree = tree()
            .file("a.md", b"a\n")
            .file("x/b.md", b"b\n")
            .symlink("x/link")
            .special("sock");
        let (walked, diagnostics) = walk(&tree);
        assert_eq!(files(&walked), ["a.md", "x/b.md"]);
        assert_eq!(refused(&walked), ["sock", "x/link"]);
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn walk_applies_the_dot_rule_first() {
        let tree = tree()
            .file("a.md", b"a\n")
            .symlink(".link")
            .special(".sock");
        let (walked, diagnostics) = walk(&tree);
        assert!(diagnostics.is_empty());
        assert_eq!(files(&walked), ["a.md"]);
        assert_eq!(ignored(&walked), [".link", ".sock"]);
        assert!(walked.refused().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn located_tree_path_refuses_non_utf8_with_location() {
        use std::os::unix::ffi::OsStrExt;

        let relative = PathBuf::from(OsStr::from_bytes(&[0x66, 0xFF]));
        let dir = Path::new("packs/core");
        let shown = dir.join(&relative);
        let message = located_tree_path(dir, &relative)
            .err()
            .map(|d| d.to_string());
        assert_eq!(
            message,
            Some(format!("{}: path is not valid UTF-8", shown.display()))
        );
    }

    #[cfg(unix)]
    #[test]
    fn walk_does_not_enter_a_refused_non_utf8_directory() {
        use std::os::unix::ffi::OsStrExt;

        let bad = OsStr::from_bytes(&[0x66, 0xFF]);
        let tree = tree().file(Path::new(bad).join("inner.md"), b"i\n");
        let (walked, diagnostics) = walk(&tree);
        assert!(walked.files().is_empty());
        let reported = messages(diagnostics);
        assert_eq!(reported.len(), 1);
        assert!(
            reported
                .iter()
                .all(|m| m.ends_with("path is not valid UTF-8"))
        );
    }

    #[test]
    fn walk_reports_a_missing_directory_as_io() {
        let tree = FakeFileTree::missing(ROOT);
        let (walked, diagnostics) = walk(&tree);
        assert!(walked.files().is_empty());
        let diagnostics = diagnostics.into_sorted();
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics
                .iter()
                .all(|d| matches!(d.error(), FormatError::Io(_)))
        );
        assert_eq!(
            diagnostics.first().map(ToString::to_string),
            Some(format!("{ROOT}: {}", FakeFileTree::not_found()))
        );
    }

    #[test]
    fn walk_reports_an_unlistable_nested_directory_and_goes_on() {
        let tree = tree().file("a.md", b"a\n").unlistable_dir("x").symlink("y");
        let (walked, diagnostics) = walk(&tree);
        assert_eq!(files(&walked), ["a.md"]);
        assert_eq!(refused(&walked), ["x", "y"]);
        assert_eq!(
            messages(diagnostics),
            [
                format!("{ROOT}/x: {}", FakeFileTree::denied()),
                format!("{ROOT}/y: {SYMLINK}")
            ]
        );
    }

    #[test]
    fn walk_reports_an_uninspectable_entry_as_io() {
        let tree = tree().file("a.md", b"a\n").uninspectable("odd");
        let (walked, diagnostics) = walk(&tree);
        assert_eq!(files(&walked), ["a.md"]);
        assert_eq!(refused(&walked), ["odd"]);
        assert_eq!(
            messages(diagnostics),
            [format!("{ROOT}/odd: {}", FakeFileTree::denied())]
        );
    }
}
