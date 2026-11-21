//! The two strings that identify what a merge touched: a file and a key inside it.
//!
//! They are separate from [`MergeEntry`](crate::types::MergeEntry) because the lock identifies a
//! merge by this pair alone, so the duplicate check and the sort order key on them without the hash
//! or the owner.
//!
//! # Responsibilities
//!
//! - [`MergeFile`] is the non-empty `[[merge]].file` string. The lock states no path rule for it.
//! - [`MergeKey`] is the non-empty `[[merge]].key` locator.
//! - They are separate types so a file and a key cannot be swapped at a call site.
//!
//! # Non-responsibilities
//!
//! What a key means inside its file belongs to the code that applies merges.

use std::fmt;

use crate::types::report::FieldError;

/// The file a merge changed, as the lock records it.
///
/// # Examples
///
/// ```
/// use agentstacks::types::MergeFile;
///
/// let file = MergeFile::new(".claude/settings.json")?;
/// assert_eq!(file.as_str(), ".claude/settings.json");
/// assert_eq!(file.to_string(), ".claude/settings.json");
/// # Ok::<(), agentstacks::types::FieldError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergeFile(String);

impl MergeFile {
    /// Wraps `value` as written.
    ///
    /// # Errors
    ///
    /// Returns [`FieldError::Empty`] for an empty or whitespace-only string.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::MergeFile;
    ///
    /// assert!(MergeFile::new(".claude/settings.json").is_ok());
    /// assert_eq!(
    ///     MergeFile::new("  ").err().map(|error| error.to_string()).as_deref(),
    ///     Some("empty")
    /// );
    /// ```
    pub fn new(value: &str) -> Result<Self, FieldError> {
        FieldError::check_not_empty(value)?;
        Ok(Self(value.to_owned()))
    }

    /// The file as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MergeFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The locator of the key a merge changed inside its file.
///
/// # Examples
///
/// ```
/// use agentstacks::types::MergeKey;
///
/// let key = MergeKey::new("hooks")?;
/// assert_eq!(key.as_str(), "hooks");
/// assert_eq!(key.to_string(), "hooks");
/// # Ok::<(), agentstacks::types::FieldError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergeKey(String);

impl MergeKey {
    /// Wraps `value` as written.
    ///
    /// # Errors
    ///
    /// Returns [`FieldError::Empty`] for an empty or whitespace-only string.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::MergeKey;
    ///
    /// assert!(MergeKey::new("hooks").is_ok());
    /// assert_eq!(
    ///     MergeKey::new("").err().map(|error| error.to_string()).as_deref(),
    ///     Some("empty")
    /// );
    /// ```
    pub fn new(value: &str) -> Result<Self, FieldError> {
        FieldError::check_not_empty(value)?;
        Ok(Self(value.to_owned()))
    }

    /// The locator as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MergeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn shown<T>(result: Result<T, FieldError>) -> Option<String> {
        result.err().map(|error| error.to_string())
    }

    #[test]
    fn merge_key_accepts_a_locator() -> TestResult {
        let key = MergeKey::new("<locator>")?;
        assert_eq!(key.as_str(), "<locator>");
        assert_eq!(key.to_string(), "<locator>");
        Ok(())
    }

    #[test]
    fn merge_key_refuses_empty_and_whitespace() {
        for text in ["", " \t"] {
            assert_eq!(
                shown(MergeKey::new(text)).as_deref(),
                Some("empty"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn merge_file_accepts_a_settings_path() -> TestResult {
        let file = MergeFile::new(".claude/settings.json")?;
        assert_eq!(file.as_str(), ".claude/settings.json");
        assert_eq!(file.to_string(), ".claude/settings.json");
        Ok(())
    }

    #[test]
    fn merge_file_refuses_empty_and_whitespace() {
        for text in ["", " \t"] {
            assert_eq!(
                shown(MergeFile::new(text)).as_deref(),
                Some("empty"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn merge_strings_order_by_text() -> TestResult {
        assert!(MergeKey::new("a")? < MergeKey::new("z")?);
        assert!(MergeFile::new("a")? < MergeFile::new("b")?);
        Ok(())
    }
}
