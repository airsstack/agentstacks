//! A value a hook's `match` list may hold.
//!
//! It is its own enum because every event that has selectors draws from the same value set, so one
//! closed type states it once.
//!
//! # Responsibilities
//!
//! - [`MatchValue`] is the closed set of selector values: the four session sources `startup`,
//!   `resume`, `clear` and `compact`, and the three tool kinds `read`, `edit` and `write`.
//!
//! # Non-responsibilities
//!
//! Which values an event accepts belongs to [`HookEvent`](crate::types::HookEvent).

use std::fmt;
use std::str::FromStr;

use crate::types::hooks::InvalidMatchValue;

/// A `match` selector value.
///
/// # Examples
///
/// ```
/// use agentstacks::types::MatchValue;
///
/// assert_eq!(MatchValue::new("resume")?, MatchValue::Resume);
/// assert!(MatchValue::new("bash").is_err());
/// assert_eq!(MatchValue::Edit.to_string(), "edit");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchValue {
    /// A session started fresh.
    Startup,
    /// A session was resumed.
    Resume,
    /// A session was cleared.
    Clear,
    /// A session was compacted.
    Compact,
    /// The tool reads files.
    Read,
    /// The tool edits files.
    Edit,
    /// The tool writes files.
    Write,
}

impl MatchValue {
    /// Parses one value of a `match` list.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidMatchValue`] when `text` is not one of the seven values.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::MatchValue;
    ///
    /// assert_eq!(MatchValue::new("write")?, MatchValue::Write);
    /// let refused = MatchValue::new("bash").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("\"bash\" unknown"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidMatchValue> {
        match text {
            "startup" => Ok(Self::Startup),
            "resume" => Ok(Self::Resume),
            "clear" => Ok(Self::Clear),
            "compact" => Ok(Self::Compact),
            "read" => Ok(Self::Read),
            "edit" => Ok(Self::Edit),
            "write" => Ok(Self::Write),
            _ => Err(InvalidMatchValue::new(text)),
        }
    }

    /// The value as written in a manifest.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Resume => "resume",
            Self::Clear => "clear",
            Self::Compact => "compact",
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Write => "write",
        }
    }
}

impl fmt::Display for MatchValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MatchValue {
    type Err = InvalidMatchValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for MatchValue {
    type Error = InvalidMatchValue;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_value_parses_the_seven_values() {
        for name in [
            "startup", "resume", "clear", "compact", "read", "edit", "write",
        ] {
            assert_eq!(
                MatchValue::new(name).map(|value| value.to_string()),
                Ok(name.to_owned())
            );
        }
        assert_eq!(MatchValue::new("bash"), Err(InvalidMatchValue::new("bash")));
    }

    #[test]
    fn match_value_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<MatchValue, InvalidMatchValue>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["read", "bash"] {
            let expected = shown(MatchValue::new(text));
            assert_eq!(shown(text.parse::<MatchValue>()), expected);
            assert_eq!(shown(MatchValue::try_from(text)), expected);
        }
        assert!(MatchValue::new("read").is_ok());
        assert!(MatchValue::new("bash").is_err());
    }
}
