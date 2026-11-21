//! The capability tier of the model an agent runs on.
//!
//! A closed enum rather than a model name, so one pack serves every tool, and the refusal's
//! list of valid words comes from [`ModelTier::ALL`].
//!
//! # Responsibilities
//!
//! - [`ModelTier`] is the closed set `fast`, `balanced` and `deep`, tool-neutral: a tool extension
//!   maps each tier to a model of its own.
//! - [`ModelTier::ALL`] lists the values in declaration order, which is also the order a refusal
//!   names them in.
//!
//! # Non-responsibilities
//!
//! Reading the `model` key out of a frontmatter belongs to [`AgentMeta`](crate::types::AgentMeta).

use std::fmt;
use std::str::FromStr;

use crate::types::frontmatter::error::FrontmatterError;

/// A tool-neutral model tier.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ModelTier;
///
/// assert_eq!("deep".parse::<ModelTier>()?, ModelTier::Deep);
/// let refused = "huge".parse::<ModelTier>().err().map(|error| error.to_string());
/// assert_eq!(refused.as_deref(), Some("\"huge\" (expected fast, balanced, deep)"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// The quickest, cheapest tier.
    Fast,
    /// The everyday tier.
    Balanced,
    /// The most capable tier.
    Deep,
}

impl ModelTier {
    /// Every value, in declaration order.
    pub const ALL: [Self; 3] = [Self::Fast, Self::Balanced, Self::Deep];

    /// Parses `value` as written in frontmatter.
    ///
    /// # Errors
    ///
    /// Returns [`FrontmatterError::NotOneOf`] when `value` is not one of [`ModelTier::ALL`].
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ModelTier;
    ///
    /// assert_eq!(ModelTier::new("deep")?.as_str(), "deep");
    /// assert!(ModelTier::new("huge").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, FrontmatterError> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or_else(|| {
                FrontmatterError::not_one_of(value, Self::ALL.into_iter().map(Self::as_str))
            })
    }

    /// The value as written in frontmatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ModelTier;
    ///
    /// assert_eq!(ModelTier::Deep.as_str(), "deep");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Deep => "deep",
        }
    }
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModelTier {
    type Err = FrontmatterError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ModelTier {
    type Error = FrontmatterError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_tier_parses_each_value() {
        for (text, expected) in [
            ("fast", ModelTier::Fast),
            ("balanced", ModelTier::Balanced),
            ("deep", ModelTier::Deep),
        ] {
            assert_eq!(text.parse::<ModelTier>().ok(), Some(expected));
            assert_eq!(expected.to_string(), text);
        }
    }

    #[test]
    fn model_tier_matching_is_case_sensitive() {
        for text in ["Fast", "BALANCED", "Deep"] {
            assert!(text.parse::<ModelTier>().is_err(), "{text}");
        }
    }

    #[test]
    fn model_tier_refuses_sonnet() {
        let refused = "sonnet".parse::<ModelTier>().err().map(|e| e.to_string());
        assert_eq!(
            refused.as_deref(),
            Some("\"sonnet\" (expected fast, balanced, deep)")
        );
    }

    #[test]
    fn model_tier_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<ModelTier, FrontmatterError>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["deep", "huge"] {
            let expected = shown(ModelTier::new(text));
            assert_eq!(shown(text.parse::<ModelTier>()), expected);
            assert_eq!(shown(ModelTier::try_from(text)), expected);
        }
        assert!(ModelTier::new("deep").is_ok());
        assert!(ModelTier::new("huge").is_err());
    }
}
