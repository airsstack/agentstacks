//! Every refusal of fetching a source.
//!
//! The enum holds no location of its own. The git and cache adapters build a [`FetchError`]
//! without one, and the flow that turns it into a [`Diagnostic`](crate::types::Diagnostic) adds
//! the `agentstacks.toml` location of the source, or the carried path for
//! [`FetchError::CacheIo`].
//!
//! # Responsibilities
//!
//! - [`FetchError`] has one variant per refusal: a tree that differs from the lock, git missing,
//!   failing or printing what cannot be read, a failed cache operation, a `subdir` that is not a
//!   directory in the source, and a missing `HOME`.
//!
//! # Usage
//!
//! `std::io::Error` does not implement `PartialEq`, so neither does [`FetchError`]; compare
//! displayed text.
//!
//! # Non-responsibilities
//!
//! Running git and touching the cache belong to the adapters; this enum only words their refusals.

use std::path::PathBuf;

use crate::types::project::CommitId;
use crate::types::report::Quoted;
use crate::types::tree::ContentHash;

/// A source that could not be fetched.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::FetchError;
///
/// assert_eq!(
///     FetchError::HomeUnset.to_string(),
///     "AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home",
/// );
/// ```
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// The tree at a locked revision hashes differently from the lock.
    #[error("tree {found} at rev {rev} differs from the lock's {locked}")]
    TreeMismatch {
        /// The hash of the tree that was fetched.
        found: ContentHash,
        /// The revision that was fetched.
        rev: CommitId,
        /// The hash the lock pins.
        locked: ContentHash,
    },
    /// Git is not on `PATH`. The list holds the keys of the sources that needed it and may be empty
    /// until the caller merges the answers for every source; this enum only joins the keys it is
    /// given.
    #[error("git not found on PATH (needed by {})", .needed_by.join(", "))]
    GitNotFound {
        /// The keys of the sources that needed git (`packs.core`, `tools.claude`).
        needed_by: Vec<String>,
    },
    /// Git could not be started.
    #[error("git: {0}")]
    GitSpawn(std::io::Error),
    /// A cache operation failed. The caller locates it at the carried path.
    #[error("{error} (set AIRSSTACK_HOME to a writable directory to move the cache)")]
    CacheIo {
        /// The cache path the operation failed on; for a failed rename, the target.
        path: PathBuf,
        /// What the operating system reported.
        error: std::io::Error,
    },
    /// A git command exited with a failure.
    #[error("git {subcommand} failed: {stderr}")]
    GitFailed {
        /// The git subcommand that failed.
        subcommand: String,
        /// What git printed on stderr, already trimmed.
        stderr: String,
    },
    /// A git command printed output that cannot be read.
    #[error("git {subcommand} printed output agentstacks cannot read: \"{first_line}\"")]
    GitUnreadable {
        /// The git subcommand.
        subcommand: String,
        /// The first line of the output, decoded lossily and trimmed.
        first_line: String,
    },
    /// The `subdir` does not exist in the fetched source.
    #[error("{subdir} not found at {at}")]
    SubdirNotFound {
        /// The `subdir`, as written.
        subdir: Quoted,
        /// The tag, or the revision when there is no tag.
        at: String,
    },
    /// The `subdir` is a file.
    #[error("{subdir} is a file, not a directory")]
    SubdirIsFile {
        /// The `subdir`, as written.
        subdir: Quoted,
    },
    /// `AIRSSTACK_HOME` and `HOME` are unset or empty, so the cache has no home.
    #[error("AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home")]
    HomeUnset,
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};
    use std::path::PathBuf;

    use super::*;
    use crate::types::report::{Diagnostic, FormatError, Location};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";
    const HEX: &str = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7";

    fn core_key() -> Location {
        Location::at_key("agentstacks.toml", "packs.core")
    }

    fn denied() -> Error {
        Error::new(ErrorKind::PermissionDenied, "permission denied")
    }

    #[test]
    fn fetch_error_tree_mismatch() -> TestResult {
        let found = ContentHash::of_bytes(b"found");
        let locked = ContentHash::new(&format!("sha256:{HEX}"))?;
        let error = FetchError::TreeMismatch {
            found: found.clone(),
            rev: CommitId::new(REV)?,
            locked,
        };
        assert_eq!(
            Diagnostic::new(core_key(), error).to_string(),
            format!(
                "agentstacks.toml: packs.core: tree sha256:{} at rev {REV} differs from the lock's sha256:{HEX}",
                found.hex()
            )
        );
        Ok(())
    }

    #[test]
    fn fetch_error_git_not_found() {
        let error = FetchError::GitNotFound {
            needed_by: vec!["packs.core".to_owned(), "tools.claude".to_owned()],
        };
        assert_eq!(
            Diagnostic::new(Location::file("agentstacks.toml"), error).to_string(),
            "agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)"
        );
    }

    #[test]
    fn fetch_error_git_not_found_starts_empty() -> TestResult {
        let mut error = FetchError::GitNotFound {
            needed_by: Vec::new(),
        };
        let FetchError::GitNotFound { needed_by } = &mut error else {
            return Err("built a GitNotFound".into());
        };
        needed_by.push("packs.core".to_owned());
        needed_by.push("tools.claude".to_owned());
        assert_eq!(
            Diagnostic::new(Location::file("agentstacks.toml"), error).to_string(),
            "agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)"
        );
        Ok(())
    }

    #[test]
    fn fetch_error_git_spawn() {
        assert_eq!(
            Diagnostic::new(core_key(), FetchError::GitSpawn(denied())).to_string(),
            "agentstacks.toml: packs.core: git: permission denied"
        );
    }

    #[test]
    fn fetch_error_cache_io_is_located_at_its_path() {
        let path = PathBuf::from(format!("/home/u/.airsstack/cache/git/{HEX}/{REV}.git"));
        let error = FetchError::CacheIo {
            path: path.clone(),
            error: denied(),
        };
        assert_eq!(
            Diagnostic::new(Location::file(path), error).to_string(),
            format!(
                "/home/u/.airsstack/cache/git/{HEX}/{REV}.git: permission denied (set AIRSSTACK_HOME to a writable directory to move the cache)"
            )
        );
    }

    #[test]
    fn fetch_error_git_failed() {
        let fetch = FetchError::GitFailed {
            subcommand: "fetch".to_owned(),
            stderr: "fatal: couldn't find remote ref refs/tags/packs-v9.9.9".to_owned(),
        };
        assert_eq!(
            Diagnostic::new(core_key(), fetch).to_string(),
            "agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9"
        );
        let ls_remote = FetchError::GitFailed {
            subcommand: "ls-remote".to_owned(),
            stderr:
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
                    .to_owned(),
        };
        assert_eq!(
            Diagnostic::new(core_key(), ls_remote).to_string(),
            "agentstacks.toml: packs.core: git ls-remote failed: fatal: could not read Username for 'https://github.com': terminal prompts disabled"
        );
    }

    #[test]
    fn fetch_error_git_unreadable() {
        let error = FetchError::GitUnreadable {
            subcommand: "rev-parse".to_owned(),
            first_line: "nonsense".to_owned(),
        };
        assert_eq!(
            Diagnostic::new(core_key(), error).to_string(),
            "agentstacks.toml: packs.core: git rev-parse printed output agentstacks cannot read: \"nonsense\""
        );
    }

    #[test]
    fn fetch_error_subdir_not_found() {
        let error = FetchError::SubdirNotFound {
            subdir: Quoted::new("packs/nope"),
            at: "packs-v0.1.0".to_owned(),
        };
        let location = Location::at_key("agentstacks.toml", "packs.core.subdir");
        assert_eq!(
            Diagnostic::new(location, error).to_string(),
            "agentstacks.toml: packs.core.subdir: \"packs/nope\" not found at packs-v0.1.0"
        );
    }

    #[test]
    fn fetch_error_subdir_is_a_file() {
        let error = FetchError::SubdirIsFile {
            subdir: Quoted::new("packs/core/pack.toml"),
        };
        let location = Location::at_key("agentstacks.toml", "packs.core.subdir");
        assert_eq!(
            Diagnostic::new(location, error).to_string(),
            "agentstacks.toml: packs.core.subdir: \"packs/core/pack.toml\" is a file, not a directory"
        );
    }

    #[test]
    fn fetch_error_home_unset() {
        assert_eq!(
            Diagnostic::new(Location::file("agentstacks.toml"), FetchError::HomeUnset).to_string(),
            "agentstacks.toml: AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home"
        );
    }

    #[test]
    fn fetch_error_converts_and_displays_unchanged() {
        assert_eq!(
            FormatError::from(FetchError::HomeUnset).to_string(),
            "AIRSSTACK_HOME and HOME are not set, so the cache (~/.airsstack/cache) has no home"
        );
    }
}
