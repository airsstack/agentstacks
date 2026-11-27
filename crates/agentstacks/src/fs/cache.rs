//! The git cache directory on the real disk.
//!
//! # Responsibilities
//!
//! - `Cache` is `<home>/.airsstack/cache`: where a cached revision is, a scratch directory a
//!   fetch fills, the rename that installs it as `<rev>.git`, and the removal of a scratch directory
//!   after a failed fetch. The layout and names come from the rules in `types::resolve`; the caller
//!   builds the scratch name from its process id, so this module never reads it.
//! - Two runs that fetch the same revision settle by the rename: the one that loses finds the
//!   directory non-empty, removes its scratch directory and uses the one in place.
//! - Every disk failure is a [`FetchError::CacheIo`] with the path it failed on.
//!
//! # Non-responsibilities
//!
//! Running `git` belongs to the `process` adapter, which calls this module for every disk step.
//! `Cache` is private to the crate: only that adapter needs it, and a caller outside could point
//! it at any directory to remove.
//! A scratch directory left by a killed process is never cleaned.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use crate::types::project::{CommitId, GitUrl};
use crate::types::resolve::{FetchError, cache_home, rev_dir, url_dir};

/// The git cache under the resolved airsstack root.
#[derive(Debug, Clone)]
pub(crate) struct Cache {
    root: PathBuf,
}

impl Cache {
    /// The cache under the root resolved from `AIRSSTACK_HOME` and `HOME`.
    ///
    /// # Errors
    ///
    /// The unlocated [`FetchError::HomeUnset`] when both are `None` or empty.
    pub(crate) fn from_home(
        airsstack_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> Result<Self, FetchError> {
        Ok(Self {
            root: cache_home(airsstack_home, home)?,
        })
    }

    /// The directory of `rev` for `url` when it is cached, so a locked revision needs no network.
    ///
    /// # Errors
    ///
    /// [`FetchError::CacheIo`] at the `<rev>.git` path when it cannot be looked at.
    pub(crate) fn repo(&self, url: &GitUrl, rev: &CommitId) -> Result<Option<PathBuf>, FetchError> {
        let installed = rev_dir(&url_dir(&self.root, url), rev);
        match installed.try_exists() {
            Ok(true) => Ok(Some(installed)),
            Ok(false) => Ok(None),
            Err(error) => Err(cache_io(installed, error)),
        }
    }

    /// A scratch path `name` in the directory of `url`, which is created. A leftover entry of that
    /// name is removed. The path itself is not created.
    ///
    /// # Errors
    ///
    /// [`FetchError::CacheIo`] at the directory being created, or at the leftover being removed.
    pub(crate) fn temp(&self, url: &GitUrl, name: &str) -> Result<PathBuf, FetchError> {
        let dir = url_dir(&self.root, url);
        std::fs::create_dir_all(&dir).map_err(|error| cache_io(dir.clone(), error))?;
        let temp = dir.join(name);
        remove_entry(&temp)?;
        Ok(temp)
    }

    /// Renames the scratch directory `temp` to the directory of `rev` for `url` and returns it.
    /// When another run installed it first, `temp` is removed and the directory in place is
    /// returned.
    ///
    /// # Errors
    ///
    /// [`FetchError::CacheIo`] at the `<rev>.git` path when the rename fails for any other reason,
    /// or at `temp` when it cannot be removed.
    pub(crate) fn install(
        &self,
        temp: &Path,
        url: &GitUrl,
        rev: &CommitId,
    ) -> Result<PathBuf, FetchError> {
        let target = rev_dir(&url_dir(&self.root, url), rev);
        match std::fs::rename(temp, &target) {
            Ok(()) => Ok(target),
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {
                remove_entry(temp)?;
                Ok(target)
            }
            Err(error) => Err(cache_io(target, error)),
        }
    }

    /// Removes the scratch directory `temp` after a failed fetch. One that is gone is not an
    /// error.
    ///
    /// # Errors
    ///
    /// [`FetchError::CacheIo`] at `temp`.
    pub(crate) fn discard(temp: &Path) -> Result<(), FetchError> {
        remove_entry(temp)
    }
}

/// The cache failure of `error` at `path`.
const fn cache_io(path: PathBuf, error: io::Error) -> FetchError {
    FetchError::CacheIo { path, error }
}

/// Removes whatever is at `path`, a directory with its contents or a file. Nothing there is not an
/// error.
fn remove_entry(path: &Path) -> Result<(), FetchError> {
    let removed = match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(error) => Err(error),
    };
    match removed {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            Err(cache_io(path.to_path_buf(), error))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::types::report::{Diagnostic, Location};
    use crate::types::resolve::temp_name;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const URL: &str = "https://github.com/airsstack/agentstacks";
    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A cache over a temp home.
    fn cache() -> Result<(tempfile::TempDir, Cache), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let cache = Cache::from_home(None, Some(home.path().as_os_str()))?;
        Ok((home, cache))
    }

    fn url() -> Result<GitUrl, Box<dyn std::error::Error>> {
        Ok(GitUrl::new(URL)?)
    }

    fn rev() -> Result<CommitId, Box<dyn std::error::Error>> {
        Ok(CommitId::new(REV)?)
    }

    /// `<home>/.airsstack/cache/git/<hex sha256 of the url>`, spelled out.
    fn url_directory(home: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(url_dir(&home.join(".airsstack"), &url()?))
    }

    #[test]
    fn no_home_is_refused() {
        for home in [None, Some(OsString::new())] {
            let error = Cache::from_home(None, home.as_deref()).err();
            assert!(matches!(error, Some(FetchError::HomeUnset)));
            assert_eq!(
                error.map(|e| e.to_string()).as_deref(),
                Some(
                    "AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home"
                )
            );
        }
    }

    #[test]
    fn a_cache_io_failure_names_its_path() -> TestResult {
        let (home, cache) = cache()?;
        std::fs::write(home.path().join(".airsstack"), b"not a directory")?;
        let error = cache
            .temp(&url()?, &temp_name(1, 0))
            .err()
            .ok_or("expected a failure")?;
        let FetchError::CacheIo { path, error: io } = &error else {
            return Err("expected the cache I/O variant".into());
        };
        assert_eq!(path, &url_directory(home.path())?);
        let expected = format!(
            "{}: {io} (set AIRSSTACK_HOME to a writable directory to move the cache)",
            path.display()
        );
        assert_eq!(
            Diagnostic::new(Location::file(path.clone()), error).to_string(),
            expected
        );
        Ok(())
    }

    #[test]
    fn a_leftover_temp_is_removed_and_reused() -> TestResult {
        let (home, cache) = cache()?;
        let leftover = url_directory(home.path())?.join(temp_name(7, 3));
        std::fs::create_dir_all(&leftover)?;
        std::fs::write(leftover.join("old"), b"old")?;
        let temp = cache.temp(&url()?, &temp_name(7, 3))?;
        assert_eq!(temp, leftover);
        assert!(!temp.exists());
        Ok(())
    }

    #[test]
    fn layout_follows_the_url_hash_and_rev() -> TestResult {
        let (home, cache) = cache()?;
        assert_eq!(cache.repo(&url()?, &rev()?)?, None);
        let temp = cache.temp(&url()?, &temp_name(1, 0))?;
        std::fs::create_dir(&temp)?;
        let installed = cache.install(&temp, &url()?, &rev()?)?;
        let expected = home
            .path()
            .join(".airsstack/cache/git")
            .join(crate::types::tree::ContentHash::of_bytes(URL.as_bytes()).hex())
            .join(format!("{REV}.git"));
        assert_eq!(installed, expected);
        assert_eq!(cache.repo(&url()?, &rev()?)?, Some(expected));
        let other = CommitId::new("fedcba9876543210fedcba9876543210fedcba98")?;
        assert_eq!(cache.repo(&url()?, &other)?, None);
        Ok(())
    }

    #[test]
    fn temp_creates_the_url_directory() -> TestResult {
        let (home, cache) = cache()?;
        let temp = cache.temp(&url()?, &temp_name(42, 0))?;
        assert_eq!(temp, url_directory(home.path())?.join("tmp-42-0"));
        assert!(url_directory(home.path())?.is_dir());
        Ok(())
    }

    #[test]
    fn install_renames_the_temp_into_place() -> TestResult {
        let (_home, cache) = cache()?;
        let temp = cache.temp(&url()?, &temp_name(1, 0))?;
        std::fs::create_dir(&temp)?;
        std::fs::write(temp.join("HEAD"), b"ref")?;
        let installed = cache.install(&temp, &url()?, &rev()?)?;
        assert!(!temp.exists());
        assert_eq!(std::fs::read(installed.join("HEAD"))?, b"ref");
        Ok(())
    }

    #[test]
    fn install_race_uses_the_existing_directory() -> TestResult {
        let (_home, cache) = cache()?;
        let first = cache.temp(&url()?, &temp_name(1, 0))?;
        std::fs::create_dir(&first)?;
        std::fs::write(first.join("HEAD"), b"first")?;
        let installed = cache.install(&first, &url()?, &rev()?)?;

        let second = cache.temp(&url()?, &temp_name(2, 0))?;
        std::fs::create_dir(&second)?;
        std::fs::write(second.join("HEAD"), b"second")?;
        let again = cache.install(&second, &url()?, &rev()?)?;
        assert_eq!(again, installed);
        assert_eq!(std::fs::read(installed.join("HEAD"))?, b"first");
        assert!(!second.exists());
        Ok(())
    }

    #[test]
    fn install_onto_an_empty_directory_replaces_it() -> TestResult {
        let (home, cache) = cache()?;
        let target = rev_dir(&url_directory(home.path())?, &rev()?);
        std::fs::create_dir_all(&target)?;
        let temp = cache.temp(&url()?, &temp_name(1, 0))?;
        std::fs::create_dir(&temp)?;
        std::fs::write(temp.join("HEAD"), b"new")?;
        assert_eq!(cache.install(&temp, &url()?, &rev()?)?, target);
        assert_eq!(std::fs::read(target.join("HEAD"))?, b"new");
        Ok(())
    }

    #[test]
    fn discard_removes_a_temp_and_tolerates_none() -> TestResult {
        let (_home, cache) = cache()?;
        let temp = cache.temp(&url()?, &temp_name(1, 0))?;
        std::fs::create_dir(&temp)?;
        std::fs::write(temp.join("HEAD"), b"x")?;
        Cache::discard(&temp)?;
        assert!(!temp.exists());
        Cache::discard(&temp)?;
        Ok(())
    }

    #[test]
    fn a_failed_install_names_the_target() -> TestResult {
        let (_home, cache) = cache()?;
        let missing = cache.temp(&url()?, &temp_name(1, 0))?;
        let error = cache
            .install(&missing, &url()?, &rev()?)
            .err()
            .ok_or("expected a failure")?;
        let FetchError::CacheIo { path, error } = error else {
            return Err("expected the cache I/O variant".into());
        };
        assert!(path.ends_with(format!("{REV}.git")));
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }
}
