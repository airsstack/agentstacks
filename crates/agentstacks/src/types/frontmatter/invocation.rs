//! Who may start a skill.
//!
//! A closed enum rather than a free string, so an unknown value is refused when the file is
//! read, and the refusal's list of valid words comes from [`Invocation::ALL`].
//!
//! # Responsibilities
//!
//! - [`Invocation`] is the closed set `user` and `any`; `any` is the default.
//! - [`Invocation::ALL`] lists the values in declaration order, which is also the order a refusal
//!   names them in.
//!
//! # Non-responsibilities
//!
//! Reading the `invocation` key out of a frontmatter belongs to
//! [`SkillMeta`](crate::types::SkillMeta).

use std::fmt;
use std::str::FromStr;

use crate::types::frontmatter::error::FrontmatterError;

/// Who may start a skill.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Invocation;
///
/// assert_eq!("user".parse::<Invocation>()?, Invocation::User);
/// let refused = "anyone".parse::<Invocation>().err().map(|error| error.to_string());
/// assert_eq!(refused.as_deref(), Some("\"anyone\" (expected user, any)"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Invocation {
    /// Only the user starts it.
    User,
    /// The user or the agent starts it.
    #[default]
    Any,
}

impl Invocation {
    /// Every value, in declaration order.
    pub const ALL: [Self; 2] = [Self::User, Self::Any];

    /// The value as written in frontmatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Invocation;
    ///
    /// assert_eq!(Invocation::User.as_str(), "user");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Any => "any",
        }
    }
}

impl fmt::Display for Invocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Invocation {
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
    fn invocation_parses_each_value() {
        for (text, expected) in [("user", Invocation::User), ("any", Invocation::Any)] {
            assert_eq!(text.parse::<Invocation>().ok(), Some(expected));
            assert_eq!(expected.to_string(), text);
        }
    }

    #[test]
    fn invocation_refuses_other_values() {
        for text in ["manual", "User"] {
            assert!(text.parse::<Invocation>().is_err(), "{text}");
        }
        let refused = "manual".parse::<Invocation>().err().map(|e| e.to_string());
        assert_eq!(refused.as_deref(), Some("\"manual\" (expected user, any)"));
    }

    #[test]
    fn invocation_defaults_to_any() {
        assert_eq!(Invocation::default(), Invocation::Any);
    }
}
