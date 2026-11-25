//! Where a tool's extension and fixtures live, and the temporary project a plan runs in.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// The shipped extension directory of `tool` at the repository root.
pub(crate) fn extension_dir(tool: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../extensions")
        .join(tool)
}

/// The fixture directory `case` of `tool`.
pub(crate) fn fixture(tool: &str, case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(tool)
        .join(case)
}

/// Copies `from` into `to`, leaving out a top-level entry named `skip`.
pub(crate) fn copy_tree(from: &Path, to: &Path, skip: Option<&str>) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        if skip.is_some_and(|name| entry.file_name() == name) {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, None)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A copy of the fixture project `case` of `tool` with the shipped extension at
/// `extensions/<tool>`.
pub(crate) fn project(tool: &str, case: &str) -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    copy_tree(&fixture(tool, case), dir.path(), Some("expected"))?;
    copy_tree(
        &extension_dir(tool),
        &dir.path().join("extensions").join(tool),
        None,
    )?;
    Ok(dir)
}
