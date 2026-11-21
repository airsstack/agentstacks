//! How much reasoning effort an agent spends.
//!
//! A closed enum rather than a free string, so an unknown effort is refused when the file is
//! read, and the refusal's list of valid words comes from [`Effort::ALL`].
//!
//! # Responsibilities
//!
//! - [`Effort`] is the closed set `low`, `medium` and `high`.
//! - [`Effort::ALL`] lists the values in declaration order, which is also the order a refusal
//!   names them in.
//!
//! # Non-responsibilities
//!
//! Reading the `effort` key out of a frontmatter belongs to [`AgentMeta`](crate::types::AgentMeta).

use std::fmt;
use std::str::FromStr;

use crate::types::frontmatter::error::FrontmatterError;

/// A tool-neutral reasoning effort.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Effort;
///
/// assert_eq!("high".parse::<Effort>()?, Effort::High);
/// let refused = "max".parse::<Effort>().err().map(|error| error.to_string());
/// assert_eq!(refused.as_deref(), Some("\"max\" (expected low, medium, high)"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    /// The least effort.
    Low,
    /// The middle of the scale.
    Medium,
    /// The most effort.
    High,
}

impl Effort {
    /// Every value, in declaration order.
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    /// The value as written in frontmatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Effort;
    ///
    /// assert_eq!(Effort::High.as_str(), "high");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Effort {
    type Err = FrontmatterError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or_else(|| {
                FrontmatterError::not_one_of(value, Self::ALL.into_iter().map(Self::as_str))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effort_parses_each_value() {
        for (text, expected) in [
            ("low", Effort::Low),
            ("medium", Effort::Medium),
            ("high", Effort::High),
        ] {
            assert_eq!(text.parse::<Effort>().ok(), Some(expected));
            assert_eq!(expected.to_string(), text);
        }
    }

    #[test]
    fn effort_matching_is_case_sensitive() {
        for text in ["High", "LOW", "Medium"] {
            assert!(text.parse::<Effort>().is_err(), "{text}");
        }
    }

    #[test]
    fn effort_refuses_max() {
        let refused = "max".parse::<Effort>().err().map(|e| e.to_string());
        assert_eq!(
            refused.as_deref(),
            Some("\"max\" (expected low, medium, high)")
        );
    }
}
