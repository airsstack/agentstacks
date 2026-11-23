//! Guards the one-word file-name rule: no Rust source file or folder under `src/` or `tests/`
//! has an underscore in its name.
//!
//! A shared word becomes a subfolder (`types/name/case.rs`), never a prefix (`case_name.rs`). Nothing
//! else enforces this: an underscore-named file compiles, passes clippy and passes every test. The
//! `tests/fixtures` and `tests/corpus` data folders are skipped, since their names belong to the
//! data they hold; `mod.rs`, `lib.rs` and `main.rs` carry no underscore to begin with.

use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

/// The data folders under `tests/` that the rule does not cover.
const DATA_DIRS: [&str; 2] = ["tests/fixtures", "tests/corpus"];

/// Pushes every entry under `dir` whose name breaks the rule onto `found`, as a path relative to
/// the crate root `root`.
fn offenders(root: &Path, dir: &Path, found: &mut Vec<String>) -> TestResult {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry?.path();
        let relative = path.strip_prefix(root)?.to_string_lossy().into_owned();
        if DATA_DIRS.contains(&relative.as_str()) {
            continue;
        }
        let is_dir = path.is_dir();
        let is_rust = path.extension().is_some_and(|e| e == "rs");
        let underscored = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().contains('_'));
        if (is_dir || is_rust) && underscored {
            found.push(relative);
        }
        if is_dir {
            offenders(root, &path, found)?;
        }
    }
    Ok(())
}

#[test]
fn source_and_test_names_are_one_word() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for dir in ["src", "tests"] {
        offenders(root, &root.join(dir), &mut found)?;
    }
    found.sort();
    assert!(
        found.is_empty(),
        "names must be one word, with no underscore; rename: {found:?}"
    );
    Ok(())
}
