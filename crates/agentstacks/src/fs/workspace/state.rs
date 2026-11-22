//! What a canonical target holds on the real disk.
//!
//! # Responsibilities
//!
//! - [`disk_state`] reads a target into the [`DiskState`] the rules decide on: nothing, a regular
//!   file with its bytes and permission bits, or anything else.
//!
//! # Non-responsibilities
//!
//! Following a link and checking the project boundary belong to `resolve`; whether a mode-only
//! difference counts as a change belongs to the rules in `types`.

use std::io;
use std::path::Path;

use crate::types::plan::{DiskFile, DiskState};

/// What `target`, a canonical path, holds.
///
/// # Errors
///
/// The platform's error from reading it, other than `NotFound`.
pub(super) fn disk_state(target: &Path) -> io::Result<DiskState> {
    let meta = match std::fs::metadata(target) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(DiskState::Absent),
        Err(error) => return Err(error),
    };
    if !meta.is_file() {
        return Ok(DiskState::NotRegular);
    }
    let bytes = std::fs::read(target)?;
    Ok(DiskState::File(DiskFile::new(bytes, mode_of(&meta))))
}

/// The permission bits of `meta`.
#[cfg(unix)]
fn mode_of(meta: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;

    meta.permissions().mode() & 0o7777
}

/// The permission bits of `meta`; a platform without them reports the usual file mode.
#[cfg(not(unix))]
fn mode_of(_meta: &std::fs::Metadata) -> u32 {
    0o644
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn a_missing_target_is_absent() -> TestResult {
        let dir = tempfile::tempdir()?;
        assert_eq!(disk_state(&dir.path().join("none"))?, DiskState::Absent);
        Ok(())
    }

    #[test]
    fn a_regular_file_carries_its_bytes_hash_and_mode() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("f");
        std::fs::write(&file, b"a\n")?;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
        let DiskState::File(found) = disk_state(&file)? else {
            return Err("expected a regular file".into());
        };
        assert_eq!(found.bytes(), b"a\n");
        assert_eq!(found.hash(), &ContentHash::of_bytes(b"a\n"));
        assert_eq!(found.mode(), 0o755);
        Ok(())
    }

    #[test]
    fn a_directory_is_not_regular() -> TestResult {
        let dir = tempfile::tempdir()?;
        assert_eq!(disk_state(dir.path())?, DiskState::NotRegular);
        Ok(())
    }
}
