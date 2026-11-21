//! Checks that the lock records airsl exactly when a locked pack needs it.
//!
//! The check is its own unit because it crosses two inputs the lock reader never holds together:
//! the lock and the manifests of the packs it locks, which the lock records only by name and hash.
//!
//! # Responsibilities
//!
//! - [`check_airsl_presence`] refuses a lock with no `[airsl]` while a locked pack declares
//!   `requires.airsl`, and a lock with `[airsl]` while none does. Both refusals are located at the
//!   lock file alone.
//!
//! # Non-responsibilities
//!
//! Loading the manifests of the locked packs belongs to the caller, because the lock holds only
//! names and hashes. Reading the lock belongs to [`Lockfile`].

use std::path::Path;

use crate::types::lock::error::LockError;
use crate::types::lock::file::Lockfile;
use crate::types::manifest::PackManifest;
use crate::types::report::Diagnostic;
use crate::types::report::Location;

/// Checks `lock`, read from `file`, against the manifests of the packs it locks.
///
/// # Errors
///
/// Returns [`LockError::AirslMissing`] when a manifest declares `requires.airsl` and the lock has
/// no `[airsl]`, and [`LockError::AirslUnneeded`] when the lock has `[airsl]` and no manifest
/// declares it.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{LockContents, Lockfile, PackManifest, check_airsl_presence};
///
/// let manifest = PackManifest::parse(
///     Path::new("packs/core/pack.toml"),
///     "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n\
///      [requires]\nairsl = \">=0.1.4\"\n",
/// )
/// .map_err(|refusals| format!("{refusals:?}"))?;
/// let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
///     .map_err(|refusals| format!("{refusals:?}"))?;
///
/// // The pack needs airsl and the lock records none.
/// let refusal = check_airsl_presence(Path::new("agentstacks.lock"), &lock, [&manifest]).err();
/// assert_eq!(
///     refusal.map(|diagnostic| diagnostic.to_string()).as_deref(),
///     Some("agentstacks.lock: [airsl] missing, but locked packs require airsl")
/// );
/// # Ok::<(), String>(())
/// ```
pub fn check_airsl_presence<'a>(
    file: &Path,
    lock: &Lockfile,
    locked_packs: impl IntoIterator<Item = &'a PackManifest>,
) -> Result<(), Diagnostic> {
    let needed = locked_packs
        .into_iter()
        .any(|manifest| manifest.requires().airsl().is_some());
    let error = match (needed, lock.airsl().is_some()) {
        (true, false) => LockError::AirslMissing,
        (false, true) => LockError::AirslUnneeded,
        (true, true) | (false, false) => return Ok(()),
    };
    Err(Diagnostic::new(Location::file(file), error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::lock::file::LockContents;
    use crate::types::manifest::PackVersion;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const LOCK_FILE: &str = "agentstacks.lock";

    fn manifest(requires: &str) -> Result<PackManifest, Box<dyn std::error::Error>> {
        let text = format!(
            "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n{requires}"
        );
        PackManifest::parse(Path::new("packs/core/pack.toml"), &text).map_err(|diagnostics| {
            let shown: Vec<String> = diagnostics
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect();
            shown.join("; ").into()
        })
    }

    fn needing() -> Result<PackManifest, Box<dyn std::error::Error>> {
        manifest("[requires]\nairsl = \">=0.1.4\"\n")
    }

    fn lock(airsl: bool) -> Result<Lockfile, Box<dyn std::error::Error>> {
        Lockfile::new(
            Path::new(LOCK_FILE),
            LockContents {
                airsl: airsl.then(|| PackVersion::new("0.1.4")).transpose()?,
                ..LockContents::default()
            },
        )
        .map_err(|d| format!("{:?}", d.into_sorted()).into())
    }

    fn shown(result: Result<(), Diagnostic>) -> Option<String> {
        result.err().map(|diagnostic| diagnostic.to_string())
    }

    #[test]
    fn airsl_presence_refuses_a_missing_section() -> TestResult {
        let packs = [manifest("")?, needing()?];
        assert_eq!(
            shown(check_airsl_presence(
                Path::new(LOCK_FILE),
                &lock(false)?,
                &packs
            ))
            .as_deref(),
            Some("agentstacks.lock: [airsl] missing, but locked packs require airsl")
        );
        Ok(())
    }

    #[test]
    fn airsl_presence_refuses_an_unneeded_section() -> TestResult {
        let expected = Some("agentstacks.lock: [airsl] present, but no locked pack requires airsl");
        let plain = [manifest("")?];
        assert_eq!(
            shown(check_airsl_presence(
                Path::new(LOCK_FILE),
                &lock(true)?,
                &plain
            ))
            .as_deref(),
            expected
        );
        assert_eq!(
            shown(check_airsl_presence(Path::new(LOCK_FILE), &lock(true)?, [])).as_deref(),
            expected
        );
        Ok(())
    }

    #[test]
    fn airsl_presence_accepts_consistent_locks() -> TestResult {
        let needing = [needing()?];
        check_airsl_presence(Path::new(LOCK_FILE), &lock(true)?, &needing)
            .map_err(|d| d.to_string())?;
        let plain = [manifest("")?];
        check_airsl_presence(Path::new(LOCK_FILE), &lock(false)?, &plain)
            .map_err(|d| d.to_string())?;
        Ok(())
    }
}
