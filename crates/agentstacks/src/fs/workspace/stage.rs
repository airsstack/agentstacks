//! The temporary file apply writes beside a target before renaming it into place.
//!
//! # Responsibilities
//!
//! - [`temp_of`] names the temporary file `.<name>.agentstacks-tmp` in the target's directory, so
//!   the later rename stays inside one directory.
//! - [`stage_bytes`] creates the directories above it and writes it, replacing a leftover temp of
//!   the same name. [`discard_temp`] removes it.
//!
//! # Non-responsibilities
//!
//! Finding the canonical target and refusing one outside the project belong to `resolve` and the
//! workspace; installing the temp belongs to `swap`.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// The temporary file of `target`: `.<name>.agentstacks-tmp` in the same directory.
pub(super) fn temp_of(target: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(target.file_name().unwrap_or_default());
    name.push(".agentstacks-tmp");
    target.with_file_name(name)
}

/// Writes `bytes` to `temp`, creating its directories. Directories created stay.
///
/// # Errors
///
/// The platform's error from creating a directory or writing the file.
pub(super) fn stage_bytes(temp: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = temp.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(temp, bytes)
}

/// Removes `temp`. One that is already gone is not an error.
///
/// # Errors
///
/// The platform's error from the removal, other than `NotFound`.
pub(super) fn discard_temp(temp: &Path) -> io::Result<()> {
    match std::fs::remove_file(temp) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn temp_sits_beside_the_target_with_a_dot_name() {
        assert_eq!(
            temp_of(Path::new("/p/dir/x.md")),
            Path::new("/p/dir/.x.md.agentstacks-tmp")
        );
    }

    #[test]
    fn stage_creates_parents() -> TestResult {
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("a/b/c.md"));
        stage_bytes(&temp, b"x")?;
        assert_eq!(
            std::fs::read(dir.path().join("a/b/.c.md.agentstacks-tmp"))?,
            b"x"
        );
        Ok(())
    }

    #[test]
    fn stage_overwrites_a_leftover_temp() -> TestResult {
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("x.md"));
        std::fs::write(&temp, b"a much longer leftover")?;
        stage_bytes(&temp, b"new")?;
        assert_eq!(std::fs::read(&temp)?, b"new");
        Ok(())
    }

    #[test]
    fn discard_removes_the_temp_and_tolerates_none() -> TestResult {
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("x.md"));
        stage_bytes(&temp, b"x")?;
        discard_temp(&temp)?;
        assert!(!temp.exists());
        discard_temp(&temp)?;
        Ok(())
    }

    #[test]
    fn discard_leaves_created_directories() -> TestResult {
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("a/b/c.md"));
        stage_bytes(&temp, b"x")?;
        discard_temp(&temp)?;
        assert!(dir.path().join("a/b").is_dir());
        Ok(())
    }
}
