//! Installing a staged file, removing one, and writing the lock.
//!
//! # Responsibilities
//!
//! - [`swap_onto`] renames a temp onto its target, first copying the permissions of a target that
//!   is an existing regular file onto the temp, because a rename installs the temp's mode.
//! - [`delete_file`] removes a file and leaves the directories above it.
//! - [`write_project`] writes `agentstacks.toml` the same way.
//! - [`write_lock`] writes `agentstacks.lock` through a temp beside it and a rename, keeping the
//!   mode of a lock that exists.
//!
//! Both temps are written through `stage_bytes`, so a symlink planted at one is removed, never
//! followed.
//!
//! # Non-responsibilities
//!
//! Finding the canonical target and refusing one outside the project belong to `resolve` and the
//! workspace. Every failure is the platform's error, handed back unchanged; what to remove and what
//! to lock after one belongs to apply.

use std::io;
use std::path::Path;

use crate::fs::workspace::stage::{stage_bytes, temp_of};

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

/// Writes `text` to `project` through `.agentstacks.toml.agentstacks-tmp` beside it, creating the
/// file when absent and keeping the mode of one that exists. A symlink at the temp is removed,
/// never followed.
///
/// # Errors
///
/// The platform's error from staging the temp (a directory there is refused), the permission
/// copy or the rename.
pub(super) fn write_project(project: &Path, text: &str) -> io::Result<()> {
    let temp = temp_of(project);
    stage_bytes(&temp, text.as_bytes())?;
    swap_onto(&temp, project)
}

/// Writes `text` to `lock` through `.agentstacks.lock.agentstacks-tmp` beside it. A symlink at the
/// temp is removed, never followed.
///
/// # Errors
///
/// The platform's error from staging the temp (a directory there is refused), the permission
/// copy or the rename.
pub(super) fn write_lock(lock: &Path, text: &str) -> io::Result<()> {
    let temp = temp_of(lock);
    stage_bytes(&temp, text.as_bytes())?;
    swap_onto(&temp, lock)
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

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
    fn write_project_replaces_the_file_and_leaves_no_temp() -> TestResult {
        let dir = tempfile::tempdir()?;
        let project = dir.path().join("agentstacks.toml");
        std::fs::write(&project, "old")?;
        write_project(&project, "tools = []\n")?;
        assert_eq!(std::fs::read_to_string(&project)?, "tools = []\n");
        assert!(
            !dir.path()
                .join(".agentstacks.toml.agentstacks-tmp")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn write_project_keeps_the_old_mode() -> TestResult {
        let dir = tempfile::tempdir()?;
        let project = dir.path().join("agentstacks.toml");
        std::fs::write(&project, "old")?;
        std::fs::set_permissions(&project, std::fs::Permissions::from_mode(0o600))?;
        write_project(&project, "new = 1\n")?;
        assert_eq!(mode_of(&project)?, 0o600);
        Ok(())
    }

    #[test]
    fn write_project_creates_the_file_when_absent() -> TestResult {
        let dir = tempfile::tempdir()?;
        let project = dir.path().join("agentstacks.toml");
        write_project(&project, "new = 1\n")?;
        assert_eq!(std::fs::read_to_string(&project)?, "new = 1\n");
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

    type Writer = fn(&Path, &str) -> io::Result<()>;

    fn discards_a_symlink(name: &str, write: Writer) -> TestResult {
        let outside = tempfile::tempdir()?;
        let victim = outside.path().join("victim.txt");
        std::fs::write(&victim, b"untouched")?;
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(name);
        let temp = temp_of(&target);
        std::os::unix::fs::symlink(&victim, &temp)?;
        write(&target, "new = 1\n")?;
        assert_eq!(std::fs::read(&victim)?, b"untouched");
        assert!(!std::fs::symlink_metadata(&target)?.file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(&target)?, "new = 1\n");
        assert!(std::fs::symlink_metadata(&temp).is_err());
        Ok(())
    }

    fn discards_a_dangling_symlink(name: &str, write: Writer) -> TestResult {
        let outside = tempfile::tempdir()?;
        let missing = outside.path().join("missing.txt");
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(name);
        std::os::unix::fs::symlink(&missing, temp_of(&target))?;
        write(&target, "new = 1\n")?;
        assert!(std::fs::symlink_metadata(&missing).is_err());
        assert_eq!(std::fs::read_to_string(&target)?, "new = 1\n");
        Ok(())
    }

    fn refuses_a_directory(name: &str, write: Writer) -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join(name);
        std::fs::write(&target, "old")?;
        let temp = temp_of(&target);
        std::fs::create_dir(&temp)?;
        assert!(write(&target, "new = 1\n").is_err());
        assert!(temp.is_dir());
        assert_eq!(std::fs::read_to_string(&target)?, "old");
        Ok(())
    }

    #[test]
    fn write_project_discards_a_symlink_at_its_temp() -> TestResult {
        discards_a_symlink("agentstacks.toml", write_project)
    }

    #[test]
    fn write_project_discards_a_dangling_symlink_at_its_temp() -> TestResult {
        discards_a_dangling_symlink("agentstacks.toml", write_project)
    }

    #[test]
    fn write_project_refuses_a_directory_at_its_temp() -> TestResult {
        refuses_a_directory("agentstacks.toml", write_project)
    }

    #[test]
    fn write_lock_discards_a_symlink_at_its_temp() -> TestResult {
        discards_a_symlink("agentstacks.lock", write_lock)
    }

    #[test]
    fn write_lock_discards_a_dangling_symlink_at_its_temp() -> TestResult {
        discards_a_dangling_symlink("agentstacks.lock", write_lock)
    }

    #[test]
    fn write_lock_refuses_a_directory_at_its_temp() -> TestResult {
        refuses_a_directory("agentstacks.lock", write_lock)
    }
}
