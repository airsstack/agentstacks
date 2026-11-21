//! What a git source is pinned to.
//!
//! The pin is an enum rather than two optional fields so a source can never hold both or neither
//! by construction.
//!
//! # Responsibilities
//!
//! - [`Pin`] is either a tag or a commit id. It holds data only.
//!
//! # Non-responsibilities
//!
//! Which of the two a file requires belongs to [`PinRule`](crate::types::PinRule).

use crate::types::project::CommitId;
use crate::types::project::GitTag;

/// The tag or commit a git source is pinned to.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{CommitId, GitTag, Pin};
///
/// let tag = Pin::Tag(GitTag::new("v1")?);
/// let rev = Pin::Rev(CommitId::new("0123456789abcdef0123456789abcdef01234567")?);
/// assert_ne!(tag, rev);
/// assert_eq!(tag.clone(), tag);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// A tag name.
    Tag(GitTag),
    /// A commit id.
    Rev(CommitId),
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn pin_tells_a_tag_from_a_commit_and_clones_equal() -> TestResult {
        let tag = Pin::Tag(GitTag::new("v1")?);
        let rev = Pin::Rev(CommitId::new("0123456789abcdef0123456789abcdef01234567")?);
        assert_ne!(tag, rev);
        assert_eq!(tag.clone(), tag);
        assert_eq!(rev.clone(), rev);
        Ok(())
    }
}
