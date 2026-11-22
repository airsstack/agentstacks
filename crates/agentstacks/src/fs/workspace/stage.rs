//! The temporary file written beside a target before it is renamed into place.
//!
//! # Responsibilities
//!
//! - [`temp_of`] names the temporary file `.<name>.agentstacks-tmp` in the target's directory, so
//!   the later rename stays inside one directory.
//! - [`stage_bytes`] creates the directories above it, removes whatever non-directory sits at the
//!   temp (a symlink is removed, never followed), then creates the temp fresh and writes it.
//!   [`discard_temp`] removes it.
//!
//! # Non-responsibilities
//!
//! Finding the canonical target and refusing one outside the project belong to `resolve` and the
//! workspace; installing the temp belongs to `swap`.

use std::ffi::OsString;
use std::io::{self, Write};
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
/// Whatever non-directory sits at `temp` is removed first, so a symlink there (live or dangling)
/// is discarded and never followed; the file is then created new and the bytes land in it.
///
/// # Errors
///
/// The platform's error from creating a directory, removing the entry at `temp` (a directory
/// there is refused and left in place) or creating and writing the file.
pub(super) fn stage_bytes(temp: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = temp.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::remove_file(temp) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    // `create_new` refuses any entry at `temp`, so a symlink planted after the removal is never
    // followed. Do not relax it to `create` + `truncate`.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    file.write_all(bytes)
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

#[cfg(all(test, unix))]
mod symlink_tests {
    use std::os::unix::fs::symlink;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn stage_discards_a_symlink_to_an_outside_file() -> TestResult {
        let outside = tempfile::tempdir()?;
        let victim = outside.path().join("victim.txt");
        std::fs::write(&victim, b"untouched")?;
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("x.md"));
        symlink(&victim, &temp)?;
        stage_bytes(&temp, b"new")?;
        assert_eq!(std::fs::read(&victim)?, b"untouched");
        assert!(!std::fs::symlink_metadata(&temp)?.file_type().is_symlink());
        assert_eq!(std::fs::read(&temp)?, b"new");
        Ok(())
    }

    #[test]
    fn stage_discards_a_dangling_symlink() -> TestResult {
        let outside = tempfile::tempdir()?;
        let missing = outside.path().join("missing.txt");
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("x.md"));
        symlink(&missing, &temp)?;
        stage_bytes(&temp, b"new")?;
        assert!(std::fs::symlink_metadata(&missing).is_err());
        assert!(!std::fs::symlink_metadata(&temp)?.file_type().is_symlink());
        assert_eq!(std::fs::read(&temp)?, b"new");
        Ok(())
    }

    #[test]
    fn stage_refuses_a_directory_at_the_temp() -> TestResult {
        let dir = tempfile::tempdir()?;
        let temp = temp_of(&dir.path().join("x.md"));
        std::fs::create_dir(&temp)?;
        assert!(stage_bytes(&temp, b"new").is_err());
        assert!(temp.is_dir());
        Ok(())
    }
}
