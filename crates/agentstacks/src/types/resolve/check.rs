//! Checking a fetched git tree against the tree the lock pins.
//!
//! It is separate from reconciling because it needs the fetched tree's hash, which only a flow
//! holding the fetched source can supply.
//!
//! # Responsibilities
//!
//! - [`check_tree`] refuses a git source whose fetched tree hashes differently from the lock's,
//!   for a locked entry and for a tool the project file dropped alike.
//!
//! # Non-responsibilities
//!
//! A `path` source always passes: a changed path tree is a normal change that the plan shows.
//! Hashing the tree and fetching it belong to the callers.

use crate::types::lock::LockedSource;
use crate::types::report::Diagnostic;
use crate::types::resolve::FetchError;
use crate::types::resolve::SourceKey;
use crate::types::tree::ContentHash;

/// Refuses a git `locked` source whose `fetched` tree differs from `lock_tree`.
///
/// The refusal is located at the entry's key and names both hashes and the locked revision.
///
/// # Errors
///
/// Returns the [`FetchError::TreeMismatch`] diagnostic when `locked` is a git source and the two
/// hashes differ.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ContentHash, LocalPath, LockedSource, PackName};
/// use agentstacks::types::resolve::{SourceKey, check_tree};
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// let source = LockedSource::Path(LocalPath::new("packs/core")?);
/// // A path source passes even when its tree changed.
/// let changed = check_tree(&key, &source, &ContentHash::of_bytes(b"a"), &ContentHash::of_bytes(b"b"));
/// assert!(changed.is_ok());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn check_tree(
    key: &SourceKey,
    locked: &LockedSource,
    fetched: &ContentHash,
    lock_tree: &ContentHash,
) -> Result<(), Diagnostic> {
    match locked {
        LockedSource::Git { rev, .. } if fetched != lock_tree => Err(Diagnostic::new(
            key.location(),
            FetchError::TreeMismatch {
                found: fetched.clone(),
                rev: rev.clone(),
                locked: lock_tree.clone(),
            },
        )),
        LockedSource::Git { .. } | LockedSource::Path(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;
    use crate::types::name::ToolName;
    use crate::types::project::CommitId;
    use crate::types::project::GitUrl;
    use crate::types::project::LocalPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    fn git_source() -> Result<LockedSource, Box<dyn std::error::Error>> {
        Ok(LockedSource::Git {
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            tag: None,
            subdir: None,
            rev: CommitId::new(REV)?,
        })
    }

    fn refusal(
        key: &SourceKey,
        a: &ContentHash,
        b: &ContentHash,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let result = check_tree(key, &git_source()?, a, b);
        Ok(result.err().ok_or("expected a refusal")?.to_string())
    }

    #[test]
    fn git_tree_mismatch_is_refused() -> TestResult {
        let key = SourceKey::Pack(PackName::new("core")?);
        let (a, b) = (ContentHash::of_bytes(b"a"), ContentHash::of_bytes(b"b"));
        assert_eq!(
            refusal(&key, &a, &b)?,
            format!(
                "agentstacks.toml: packs.core: tree sha256:{} at rev {REV} differs from the lock's sha256:{}",
                a.hex(),
                b.hex()
            )
        );
        Ok(())
    }

    #[test]
    fn removed_tool_tree_mismatch_is_refused() -> TestResult {
        let key = SourceKey::Tool(ToolName::new("claude")?);
        let (a, b) = (ContentHash::of_bytes(b"a"), ContentHash::of_bytes(b"b"));
        assert_eq!(
            refusal(&key, &a, &b)?,
            format!(
                "agentstacks.toml: tools.claude: tree sha256:{} at rev {REV} differs from the lock's sha256:{}",
                a.hex(),
                b.hex()
            )
        );
        Ok(())
    }

    #[test]
    fn git_tree_equal_passes() -> TestResult {
        let key = SourceKey::Pack(PackName::new("core")?);
        let hash = ContentHash::of_bytes(b"a");
        assert!(check_tree(&key, &git_source()?, &hash, &hash.clone()).is_ok());
        Ok(())
    }

    #[test]
    fn path_tree_change_is_not_refused() -> TestResult {
        let key = SourceKey::Pack(PackName::new("core")?);
        let source = LockedSource::Path(LocalPath::new("packs/core")?);
        let (a, b) = (ContentHash::of_bytes(b"a"), ContentHash::of_bytes(b"b"));
        assert!(check_tree(&key, &source, &a, &b).is_ok());
        Ok(())
    }
}
