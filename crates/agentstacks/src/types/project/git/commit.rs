//! A git commit id.
//!
//! It is a type of its own, not a `String` field of the source, so the length and lowercase-hex
//! rule is checked once at construction and every later holder can trust it.
//!
//! # Responsibilities
//!
//! - [`CommitId`] is exactly 40 (SHA-1) or 64 (SHA-256) lowercase hexadecimal characters.
//!
//! # Non-responsibilities
//!
//! Whether the commit exists in the repository is decided when it is fetched.

use std::fmt;
use std::str::FromStr;

use crate::types::project::InvalidCommitId;
use crate::types::report::FieldError;

/// The length of a SHA-1 commit id in hex characters.
const SHA1_LEN: usize = 40;
/// The length of a SHA-256 commit id in hex characters.
const SHA256_LEN: usize = 64;

/// A full-length lowercase hex commit id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitId(String);

impl CommitId {
    /// Parses `text` as a commit id.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCommitId::Empty`] for an empty or whitespace-only string, and
    /// [`InvalidCommitId::NotACommitId`] for anything else that is not exactly 40 or 64 characters
    /// of `[0-9a-f]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{CommitId, InvalidCommitId};
    ///
    /// let id = CommitId::new("0123456789abcdef0123456789abcdef01234567")?;
    /// assert_eq!(id.as_str().len(), 40);
    ///
    /// // A short id, or one with uppercase digits, is not a commit id.
    /// assert_eq!(CommitId::new("0123abc"), Err(InvalidCommitId::NotACommitId));
    /// # Ok::<(), InvalidCommitId>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidCommitId> {
        FieldError::check_not_empty(text).map_err(|_| InvalidCommitId::Empty)?;
        let length_fits = matches!(text.len(), SHA1_LEN | SHA256_LEN);
        let lowercase_hex = text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if length_fits && lowercase_hex {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidCommitId::NotACommitId)
        }
    }

    /// The commit id as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for CommitId {
    type Err = InvalidCommitId;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn commit_id_accepts_40_and_64_lowercase_hex() -> TestResult {
        for length in [SHA1_LEN, SHA256_LEN] {
            let text = "a1b2c3d4e5f6"
                .chars()
                .cycle()
                .take(length)
                .collect::<String>();
            let id = CommitId::new(&text)?;
            assert_eq!(id.as_str(), text);
            assert_eq!(id.to_string(), text);
            assert_eq!(text.parse::<CommitId>()?, id);
        }
        Ok(())
    }

    #[test]
    fn commit_id_refuses_other_forms() {
        let refused = [
            "a".repeat(39),
            "a".repeat(41),
            "A".repeat(40),
            format!("{}g", "a".repeat(39)),
            "main".to_owned(),
        ];
        for text in refused {
            let shown = CommitId::new(&text).err().map(|e| e.to_string());
            assert_eq!(shown.as_deref(), Some("not a commit id"), "{text}");
        }
        let shown = CommitId::new("").err().map(|e| e.to_string());
        assert_eq!(shown.as_deref(), Some("empty"));
    }
}
