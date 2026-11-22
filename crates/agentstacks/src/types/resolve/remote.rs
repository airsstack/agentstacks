//! Reading the tag lines `git ls-remote` prints.
//!
//! It is separate from the tag rules because it reads git's output, while the rules only compare
//! the values this module returns.
//!
//! # Responsibilities
//!
//! - [`RemoteTag`] is one tag of a remote and the commit it names.
//! - [`parse_ls_remote`] reads the output of `git ls-remote --tags --refs`: one
//!   `<hex> TAB refs/tags/<tag>` line per tag, in git's order. Empty output is no tags.
//! - [`parse_ls_remote_at`] is the same read with its refusal located at an entry's key.
//!
//! # Non-responsibilities
//!
//! Running git and choosing among the tags belong to the git adapter and to the tag rules.

use crate::types::project::CommitId;
use crate::types::project::GitTag;
use crate::types::report::Diagnostic;
use crate::types::resolve::FetchError;
use crate::types::resolve::GitStep;
use crate::types::resolve::SourceKey;
use crate::types::resolve::unreadable;

/// The prefix of every ref `git ls-remote --tags` lists.
const TAG_REF: &str = "refs/tags/";

/// One tag of a remote repository.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::parse_ls_remote;
///
/// let line = "aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd\trefs/tags/packs-v0.1.0\n";
/// let tags = parse_ls_remote(line.as_bytes())?;
/// assert_eq!(tags[0].tag.as_str(), "packs-v0.1.0");
/// assert_eq!(tags[0].rev.as_str(), "aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd");
/// # Ok::<(), agentstacks::types::resolve::FetchError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTag {
    /// The commit the tag names.
    pub rev: CommitId,
    /// The tag.
    pub tag: GitTag,
}

/// One `<hex> TAB refs/tags/<tag>` line, or `None` when it has another shape.
fn parse_line(line: &str) -> Option<RemoteTag> {
    let (hex, reference) = line.split_once('\t')?;
    let rev = CommitId::new(hex).ok()?;
    let tag = GitTag::new(reference.strip_prefix(TAG_REF)?).ok()?;
    Some(RemoteTag { rev, tag })
}

/// Reads `stdout`, the output of `git ls-remote --tags --refs`, into one [`RemoteTag`] per line,
/// in output order.
///
/// # Errors
///
/// Returns the unlocated [`FetchError::GitUnreadable`] for `ls-remote` when any line has another
/// shape: no tab, an id that is not a commit id, or a ref outside `refs/tags/`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::parse_ls_remote;
///
/// assert!(parse_ls_remote(b"")?.is_empty());
/// assert!(parse_ls_remote(b"not a tag line").is_err());
/// # Ok::<(), agentstacks::types::resolve::FetchError>(())
/// ```
pub fn parse_ls_remote(stdout: &[u8]) -> Result<Vec<RemoteTag>, FetchError> {
    let text = String::from_utf8_lossy(stdout);
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    text.split('\n')
        .map(|line| parse_line(line).ok_or_else(|| unreadable(GitStep::LsRemote, stdout)))
        .collect()
}

/// [`parse_ls_remote`], with its refusal located at the entry `key`.
///
/// # Errors
///
/// The same refusal as [`parse_ls_remote`], at [`SourceKey::location`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
/// use agentstacks::types::resolve::{SourceKey, parse_ls_remote_at};
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// let refusal = parse_ls_remote_at(&key, b"junk").err().ok_or("expected a refusal")?;
/// assert_eq!(
///     refusal.to_string(),
///     "agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: \"junk\"",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_ls_remote_at(key: &SourceKey, stdout: &[u8]) -> Result<Vec<RemoteTag>, Diagnostic> {
    parse_ls_remote(stdout).map_err(|error| Diagnostic::new(key.location(), error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OLD: &str = "aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd";
    const NEW: &str = "90e2a535db371753890122ed088c81bf3867be7a";

    fn core() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("core")?))
    }

    #[test]
    fn ls_remote_lines_parse() -> TestResult {
        let stdout = format!(
            "{OLD}\trefs/tags/packs-v0.1.0\n{NEW}\trefs/tags/packs-v0.10.0\n\
             {NEW}\trefs/tags/packs-v0.2.0\n{NEW}\trefs/tags/packs-v0.9.0\n"
        );
        let tags = parse_ls_remote(stdout.as_bytes())?;
        let seen: Vec<(&str, &str)> = tags
            .iter()
            .map(|tag| (tag.rev.as_str(), tag.tag.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                (OLD, "packs-v0.1.0"),
                (NEW, "packs-v0.10.0"),
                (NEW, "packs-v0.2.0"),
                (NEW, "packs-v0.9.0"),
            ]
        );
        Ok(())
    }

    #[test]
    fn ls_remote_empty_output_is_empty() -> TestResult {
        assert!(parse_ls_remote(b"")?.is_empty());
        let located = parse_ls_remote_at(&core()?, b"").map_err(|refusal| refusal.to_string())?;
        assert!(located.is_empty());
        Ok(())
    }

    #[test]
    fn ls_remote_non_tag_ref_is_unreadable() -> TestResult {
        let line = format!("{NEW}\trefs/heads/packs-v1");
        let error = parse_ls_remote(line.as_bytes())
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::GitUnreadable { .. }));
        assert_eq!(
            error.to_string(),
            format!("git ls-remote printed output agentstacks cannot read: \"{line}\"")
        );
        let located = parse_ls_remote_at(&core()?, line.as_bytes())
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            located.to_string(),
            format!(
                "agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: \"{line}\""
            )
        );
        Ok(())
    }

    #[test]
    fn ls_remote_malformed_line_is_unreadable() -> TestResult {
        let no_tab = format!("{NEW} refs/tags/packs-v1");
        let short = format!("{}\trefs/tags/packs-v1", &NEW[..39]);
        for line in [no_tab, short] {
            let stdout = format!("{line}\nsecond line\n");
            let error = parse_ls_remote(stdout.as_bytes())
                .err()
                .ok_or("expected a refusal")?;
            assert_eq!(
                error.to_string(),
                format!("git ls-remote printed output agentstacks cannot read: \"{line}\"")
            );
        }
        Ok(())
    }

    #[test]
    fn ls_remote_one_bad_line_refuses_the_whole_output() -> TestResult {
        let stdout = format!("{OLD}\trefs/tags/packs-v0.1.0\nnonsense\n");
        let error = parse_ls_remote(stdout.as_bytes())
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            error.to_string(),
            format!(
                "git ls-remote printed output agentstacks cannot read: \"{OLD}\trefs/tags/packs-v0.1.0\""
            )
        );
        Ok(())
    }
}
