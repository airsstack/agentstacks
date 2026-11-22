//! Installing a staged file, removing one, and writing the lock.
//!
//! # Responsibilities
//!
//! - [`swap_onto`] renames a temp onto its target, first copying the permissions of a target that
//!   is an existing regular file onto the temp, because a rename installs the temp's mode.
//! - [`delete_file`] removes a file and leaves the directories above it.
//! - [`write_lock`] writes `agentstacks.lock` through a temp beside it and a rename, keeping the
//!   mode of a lock that exists.
//!
//! # Non-responsibilities
//!
//! Finding the canonical target and refusing one outside the project belong to `resolve` and the
//! workspace. Every failure is the platform's error, handed back unchanged; what to remove and what
//! to lock after one belongs to apply.

use std::io;
use std::path::Path;

use crate::fs::workspace::stage::temp_of;

/// Renames `temp` onto `target`, keeping the permissions of a regular file already there.
///
/// # Errors
///
/// The platform's error from copying the permissions or from the rename.
pub(super) fn swap_onto(temp: &Path, target: &Path) -> io::Result<()> {
    if let Ok(existing) = std::fs::metadata(target)
        && existing.is_file()
    {
        std::fs::set_permissions(temp, existing.permissions())?;
    }
    std::fs::rename(temp, target)
}

/// Removes the file `target`.
///
/// # Errors
///
/// The platform's error from the removal.
pub(super) fn delete_file(target: &Path) -> io::Result<()> {
    std::fs::remove_file(target)
}

/// Writes `text` to `lock` through `.agentstacks.lock.agentstacks-tmp` beside it.
///
/// # Errors
///
/// The platform's error from the write, the permission copy or the rename.
pub(super) fn write_lock(lock: &Path, text: &str) -> io::Result<()> {
    let temp = temp_of(lock);
    std::fs::write(&temp, text)?;
    swap_onto(&temp, lock)
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::fs::workspace::stage::{stage_bytes, temp_of};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn mode_of(path: &Path) -> io::Result<u32> {
        Ok(std::fs::metadata(path)?.permissions().mode() & 0o7777)
    }

    #[test]
    fn swap_keeps_the_old_mode() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("x.sh");
        std::fs::write(&target, b"old")?;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
        stage_bytes(&temp_of(&target), b"new")?;
        swap_onto(&temp_of(&target), &target)?;
        assert_eq!(std::fs::read(&target)?, b"new");
        assert_eq!(mode_of(&target)?, 0o755);
        Ok(())
    }

    #[test]
    fn swap_gives_a_new_file_the_write_mode() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("new.md");
        stage_bytes(&temp_of(&target), b"x")?;
        swap_onto(&temp_of(&target), &target)?;
        let plain = dir.path().join("plain.md");
        std::fs::write(&plain, b"x")?;
        assert_eq!(mode_of(&target)?, mode_of(&plain)?);
        Ok(())
    }

    #[test]
    fn swap_without_a_temp_fails_with_not_found() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("x.md");
        let error = swap_onto(&temp_of(&target), &target)
            .err()
            .ok_or("expected a failure")?;
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn delete_removes_the_file_and_leaves_its_directory() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("d"))?;
        let file = dir.path().join("d/x.md");
        std::fs::write(&file, b"x")?;
        delete_file(&file)?;
        assert!(!file.exists());
        assert!(dir.path().join("d").is_dir());
        Ok(())
    }

    #[test]
    fn write_lock_replaces_the_lock_and_leaves_no_temp() -> TestResult {
        let dir = tempfile::tempdir()?;
        let lock = dir.path().join("agentstacks.lock");
        std::fs::write(&lock, "old")?;
        std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o600))?;
        write_lock(&lock, "new = 1\n")?;
        assert_eq!(std::fs::read_to_string(&lock)?, "new = 1\n");
        assert_eq!(mode_of(&lock)?, 0o600);
        assert!(
            !dir.path()
                .join(".agentstacks.lock.agentstacks-tmp")
                .exists()
        );
        Ok(())
    }
}
