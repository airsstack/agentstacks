//! The refusal for git output that cannot be read.
//!
//! It is separate from the parsers because several of them, and the flows that read other git
//! output, refuse the same way.
//!
//! # Responsibilities
//!
//! - [`GitStep`] names the git subcommand whose output is read.
//! - [`first_line`] is the text a refusal quotes: the output decoded as lossy UTF-8, trimmed, up to
//!   the first line break.
//! - [`unreadable`] is the refusal without a location, for the adapters that run git;
//!   [`unreadable_at`] is the same refusal located at an entry's key.
//!
//! # Non-responsibilities
//!
//! Deciding that output is unreadable belongs to the parser of that output.

use std::fmt;

use crate::types::report::Diagnostic;
use crate::types::resolve::FetchError;
use crate::types::resolve::SourceKey;

/// The git subcommand whose output is being read.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::GitStep;
///
/// assert_eq!(GitStep::LsRemote.to_string(), "ls-remote");
/// assert_eq!(GitStep::CatFile.to_string(), "cat-file");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStep {
    /// `git ls-remote`.
    LsRemote,
    /// `git ls-tree`.
    LsTree,
    /// `git rev-parse`.
    RevParse,
    /// `git cat-file`.
    CatFile,
}

impl fmt::Display for GitStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LsRemote => "ls-remote",
            Self::LsTree => "ls-tree",
            Self::RevParse => "rev-parse",
            Self::CatFile => "cat-file",
        })
    }
}

/// The first line of `output`: decoded as lossy UTF-8, trimmed of surrounding whitespace, and cut
/// at the first line break or NUL, so the first record of NUL-separated output counts as a line.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::first_line;
///
/// assert_eq!(first_line(b"  junk\nmore\n"), "junk");
/// ```
#[must_use]
pub fn first_line(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    text.trim()
        .split(['\n', '\0'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The refusal for `step`'s `output`, which could not be read. It carries no location; the flow
/// that holds the entry's key locates it with [`SourceKey::location`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::{GitStep, unreadable};
///
/// assert_eq!(
///     unreadable(GitStep::RevParse, b"nonsense\n").to_string(),
///     "git rev-parse printed output agentstacks cannot read: \"nonsense\"",
/// );
/// ```
#[must_use]
pub fn unreadable(step: GitStep, output: &[u8]) -> FetchError {
    FetchError::GitUnreadable {
        subcommand: step.to_string(),
        first_line: first_line(output),
    }
}

/// The refusal for `step`'s `output`, located at the entry `key`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
/// use agentstacks::types::resolve::{GitStep, SourceKey, unreadable_at};
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// assert_eq!(
///     unreadable_at(&key, GitStep::LsRemote, b"junk").to_string(),
///     "agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: \"junk\"",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn unreadable_at(key: &SourceKey, step: GitStep, output: &[u8]) -> Diagnostic {
    Diagnostic::new(key.location(), unreadable(step, output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn unreadable_quotes_the_first_line() -> TestResult {
        let output = b"  junk\nmore\n";
        assert_eq!(
            unreadable(GitStep::LsRemote, output).to_string(),
            "git ls-remote printed output agentstacks cannot read: \"junk\""
        );
        let key = SourceKey::Pack(PackName::new("core")?);
        assert_eq!(
            unreadable_at(&key, GitStep::LsRemote, output).to_string(),
            "agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: \"junk\""
        );
        Ok(())
    }

    #[test]
    fn unreadable_decodes_lossy_utf8() {
        assert_eq!(
            unreadable(GitStep::LsTree, b"a\xffb").to_string(),
            "git ls-tree printed output agentstacks cannot read: \"a\u{fffd}b\""
        );
    }

    #[test]
    fn first_line_stops_at_the_first_record_of_nul_separated_output() {
        assert_eq!(first_line(b"one\0two\0"), "one");
        assert_eq!(first_line(b""), "");
    }

    #[test]
    fn git_step_words() {
        assert_eq!(GitStep::LsRemote.to_string(), "ls-remote");
        assert_eq!(GitStep::LsTree.to_string(), "ls-tree");
        assert_eq!(GitStep::RevParse.to_string(), "rev-parse");
        assert_eq!(GitStep::CatFile.to_string(), "cat-file");
    }
}
