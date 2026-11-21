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

/// A `match` selector value.
///
/// # Examples
///
/// ```
/// use agentstacks::types::MatchValue;
///
/// assert_eq!(MatchValue::parse("resume"), Some(MatchValue::Resume));
/// assert_eq!(MatchValue::parse("bash"), None);
/// assert_eq!(MatchValue::Edit.to_string(), "edit");
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
    /// Parses one value of a `match` list; `None` when `text` is not one of the seven.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "startup" => Some(Self::Startup),
            "resume" => Some(Self::Resume),
            "clear" => Some(Self::Clear),
            "compact" => Some(Self::Compact),
            "read" => Some(Self::Read),
            "edit" => Some(Self::Edit),
            "write" => Some(Self::Write),
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_value_parses_the_seven_values() {
        for name in [
            "startup", "resume", "clear", "compact", "read", "edit", "write",
        ] {
            assert_eq!(
                MatchValue::parse(name).map(|value| value.to_string()),
                Some(name.to_owned())
            );
        }
        assert_eq!(MatchValue::parse("bash"), None);
    }
}
