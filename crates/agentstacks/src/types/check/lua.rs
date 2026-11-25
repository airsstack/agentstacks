//! The two decisions about an extension's Lua tests.
//!
//! # Responsibilities
//!
//! - [`has_tests`] says whether a walked file list holds a Lua test file, which is what decides
//!   that `airsl test` runs at all, because `airsl test` exits 1 when it finds no test file.
//! - [`summary`] picks the line of an `airsl test` run that the report shows.
//!
//! # Non-responsibilities
//!
//! Running `airsl` belongs to the `Binary` port; this file only reads what it gave back.

use std::path::Path;

use crate::types::tree::TreePath;

/// Whether `files` holds a file named `*_test.lua` or `test_*.lua`.
///
/// `files` is a walk's file list, which already leaves out dot entries.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::check::has_tests;
/// use agentstacks::types::walk;
///
/// let root = env!("CARGO_MANIFEST_DIR");
/// let (tested, _) = walk(&StdFileTree::new(format!("{root}/tests/fixtures/lua/with-tests")));
/// assert!(has_tests(tested.files()));
/// let (pack, _) = walk(&StdFileTree::new(format!("{root}/tests/fixtures/packs/valid-tokens")));
/// assert!(!has_tests(pack.files()));
/// ```
#[must_use]
pub fn has_tests(files: &[TreePath]) -> bool {
    files.iter().any(|path| {
        let file = Path::new(path.as_str());
        file.extension().is_some_and(|extension| extension == "lua")
            && file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.ends_with("_test") || stem.starts_with("test_"))
    })
}

/// The last non-empty line of `stdout`, which `airsl test` ends with its counts, or `None` when
/// there is none.
///
/// # Examples
///
/// ```
/// use agentstacks::types::check::summary;
///
/// assert_eq!(summary("ok a\n1 passed, 0 failed (1 files)\n"), Some("1 passed, 0 failed (1 files)"));
/// assert_eq!(summary("\n \n"), None);
/// ```
#[must_use]
pub fn summary(stdout: &str) -> Option<&str> {
    stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;
    use crate::types::tree::walk;

    fn walked_files(paths: &[&str]) -> Vec<TreePath> {
        let tree = paths.iter().fold(FakeFileTree::new("ext/x"), |tree, path| {
            tree.file(path, b"\n")
        });
        walk(&tree).0.files().to_vec()
    }

    #[test]
    fn has_tests_by_name() {
        assert!(has_tests(&walked_files(&["lib/a_test.lua"])));
        assert!(has_tests(&walked_files(&["test_b.lua"])));
        assert!(!has_tests(&walked_files(&["lib/a.lua"])));
        assert!(!has_tests(&walked_files(&[".hidden/x_test.lua"])));
    }

    #[test]
    fn has_tests_reads_the_file_name_only() {
        assert!(!has_tests(&walked_files(&["test_dir/a.lua"])));
        assert!(!has_tests(&walked_files(&["a_test.lua.md", "a_test.txt"])));
        assert!(!has_tests(&[]));
    }

    #[test]
    fn summary_is_last_nonempty_line() {
        assert_eq!(
            summary("a_test.lua\n  ok    runs\n\n1 passed, 0 failed (1 files)\n"),
            Some("1 passed, 0 failed (1 files)")
        );
        assert_eq!(summary("a\n\n  b  \n\n"), Some("b"));
        assert_eq!(summary(" \n"), None);
        assert_eq!(summary(""), None);
    }
}
