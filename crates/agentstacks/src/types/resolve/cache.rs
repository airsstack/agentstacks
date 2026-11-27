//! Where the git cache lives.
//!
//! It is separate from the git adapter because the layout is a rule the adapter and its tests
//! share, and none of it needs the disk.
//!
//! # Responsibilities
//!
//! - [`cache_home`] takes the home directory the caller read from `HOME` and refuses a missing or
//!   empty one.
//! - [`url_dir`] is the directory that holds every cached revision of one repository URL, and
//!   [`rev_dir`] the directory of one revision inside it.
//! - [`temp_name`] is the name of a scratch directory a fetch fills before it is renamed into place.
//!
//! # Non-responsibilities
//!
//! Reading the environment, creating the directories and the rename belong to the caller and the
//! git adapter; this module only builds paths and names from the values it is given.

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use crate::types::project::CommitId;
use crate::types::project::GitUrl;
use crate::types::resolve::FetchError;
use crate::types::tree::ContentHash;

/// The airsstack root directory the cache lives under.
///
/// `airsstack_home` (the `AIRSSTACK_HOME` value the caller read) wins when it is set and
/// non-empty; otherwise the root is `<HOME>/.airsstack`. An empty value counts as unset,
/// matching the `${AIRSSTACK_HOME:-$HOME/.airsstack}` expansion the hooks use.
///
/// # Errors
///
/// Returns the unlocated [`FetchError::HomeUnset`] when neither value is set and non-empty;
/// the caller locates it at `agentstacks.toml` and reports it once.
///
/// # Examples
///
/// ```
/// use std::ffi::OsStr;
/// use std::path::Path;
///
/// use agentstacks::types::resolve::cache_home;
///
/// assert_eq!(
///     cache_home(Some(OsStr::new("/sandbox/home")), Some(OsStr::new("/home/u")))?,
///     Path::new("/sandbox/home")
/// );
/// assert_eq!(
///     cache_home(None, Some(OsStr::new("/home/u")))?,
///     Path::new("/home/u/.airsstack")
/// );
/// assert_eq!(
///     cache_home(Some(OsStr::new("")), Some(OsStr::new("/home/u")))?,
///     Path::new("/home/u/.airsstack")
/// );
/// assert!(cache_home(None, None).is_err());
/// # Ok::<(), agentstacks::types::resolve::FetchError>(())
/// ```
pub fn cache_home(
    airsstack_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf, FetchError> {
    match airsstack_home {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => match home {
            Some(value) if !value.is_empty() => Ok(PathBuf::from(value).join(".airsstack")),
            _ => Err(FetchError::HomeUnset),
        },
    }
}

/// `<root>/cache/git/<hex sha256 of the url>`: the directory of one repository URL.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::GitUrl;
/// use agentstacks::types::resolve::url_dir;
///
/// let url = GitUrl::new("https://github.com/airsstack/agentstacks")?;
/// let dir = url_dir(Path::new("/home/u/.airsstack"), &url);
/// assert!(dir.starts_with("/home/u/.airsstack/cache/git"));
/// # Ok::<(), agentstacks::types::InvalidGitUrl>(())
/// ```
#[must_use]
pub fn url_dir(root: &Path, url: &GitUrl) -> PathBuf {
    let digest = ContentHash::of_bytes(url.as_str().as_bytes());
    root.join("cache").join("git").join(digest.hex())
}

/// `<url dir>/<rev>.git`: the directory of one cached revision.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::CommitId;
/// use agentstacks::types::resolve::rev_dir;
///
/// let rev = CommitId::new("0123456789abcdef0123456789abcdef01234567")?;
/// assert_eq!(
///     rev_dir(Path::new("/cache/git/abc"), &rev),
///     Path::new("/cache/git/abc/0123456789abcdef0123456789abcdef01234567.git"),
/// );
/// # Ok::<(), agentstacks::types::InvalidCommitId>(())
/// ```
#[must_use]
pub fn rev_dir(url_dir: &Path, rev: &CommitId) -> PathBuf {
    url_dir.join(format!("{rev}.git"))
}

/// `tmp-<pid>-<n>`: the name of a scratch directory, from the caller's process id and counter.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::temp_name;
///
/// assert_eq!(temp_name(42, 0), "tmp-42-0");
/// assert_eq!(temp_name(42, 1), "tmp-42-1");
/// ```
#[must_use]
pub fn temp_name(pid: u32, n: u64) -> String {
    format!("tmp-{pid}-{n}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::Diagnostic;
    use crate::types::report::Location;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";
    const URL: &str = "https://github.com/airsstack/agentstacks";

    #[test]
    fn cache_home_refuses_a_missing_home() -> TestResult {
        let error = cache_home(None, None).err().ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::HomeUnset));
        assert_eq!(
            Diagnostic::new(Location::file("agentstacks.toml"), error).to_string(),
            "agentstacks.toml: AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home"
        );
        Ok(())
    }

    #[test]
    fn cache_home_refuses_an_empty_home() -> TestResult {
        let error = cache_home(Some(OsStr::new("")), Some(OsStr::new("")))
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::HomeUnset));
        assert_eq!(
            Diagnostic::new(Location::file("agentstacks.toml"), error).to_string(),
            "agentstacks.toml: AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home"
        );
        Ok(())
    }

    #[test]
    fn cache_home_prefers_airsstack_home() -> TestResult {
        assert_eq!(
            cache_home(
                Some(OsStr::new("/sandbox/home")),
                Some(OsStr::new("/home/u"))
            )?,
            Path::new("/sandbox/home")
        );
        assert_eq!(
            cache_home(Some(OsStr::new("/sandbox/home")), None)?,
            Path::new("/sandbox/home")
        );
        Ok(())
    }

    #[test]
    fn cache_home_falls_back_to_home_dot_airsstack() -> TestResult {
        assert_eq!(
            cache_home(None, Some(OsStr::new("/home/u")))?,
            Path::new("/home/u/.airsstack")
        );
        assert_eq!(
            cache_home(Some(OsStr::new("")), Some(OsStr::new("/home/u")))?,
            Path::new("/home/u/.airsstack")
        );
        Ok(())
    }

    #[test]
    fn url_dir_is_the_sha256_of_the_url() -> TestResult {
        let url = GitUrl::new(URL)?;
        let dir = url_dir(Path::new("/home/u/.airsstack"), &url);
        let hex = ContentHash::of_bytes(URL.as_bytes());
        assert_eq!(
            dir,
            Path::new("/home/u/.airsstack/cache/git").join(hex.hex())
        );
        assert_eq!(
            dir.parent(),
            Some(Path::new("/home/u/.airsstack/cache/git"))
        );
        Ok(())
    }

    #[test]
    fn rev_dir_appends_rev_dot_git() -> TestResult {
        let dir = url_dir(Path::new("/home/u"), &GitUrl::new(URL)?);
        let rev_dir = rev_dir(&dir, &CommitId::new(REV)?);
        assert_eq!(rev_dir, dir.join(format!("{REV}.git")));
        assert_eq!(
            rev_dir.file_name().and_then(OsStr::to_str),
            Some(format!("{REV}.git").as_str())
        );
        Ok(())
    }

    #[test]
    fn temp_name_counts_up() {
        assert_eq!(temp_name(42, 0), "tmp-42-0");
        assert_eq!(temp_name(42, 1), "tmp-42-1");
    }
}
