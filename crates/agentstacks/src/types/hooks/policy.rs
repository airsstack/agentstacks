//! The sandbox policy a pack hook runs under.
//!
//! It is its own enum so that `trusted` is unrepresentable for a pack hook: a closed type makes that
//! a fact of the type instead of a check every reader repeats.
//!
//! # Responsibilities
//!
//! - [`Policy`] is `confined` or `pure`; `confined` is the default. The airsl policy `trusted` is
//!   deliberately not representable, because a pack hook may not run with it.
//!
//! # Non-responsibilities
//!
//! Turning a policy into airsl flags belongs to the extension host.

use std::fmt;

use crate::types::hooks::InvalidPolicy;

/// A policy a pack hook may use.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Policy;
///
/// assert_eq!(Policy::parse("pure")?, Policy::Pure);
/// assert_eq!(Policy::default(), Policy::Confined);
///
/// let refused = Policy::parse("trusted").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"trusted\" is not allowed for pack hooks");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Policy {
    /// Reads and writes only what the grants name.
    #[default]
    Confined,
    /// No access beyond the script itself.
    Pure,
}

impl Policy {
    /// Parses a `grants.policy` value.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPolicy::NotAllowed`] for `trusted`, and [`InvalidPolicy::Unknown`] for any
    /// other value but `confined` and `pure`.
    pub fn parse(text: &str) -> Result<Self, InvalidPolicy> {
        let text = text.to_owned();
        match text.as_str() {
            "confined" => Ok(Self::Confined),
            "pure" => Ok(Self::Pure),
            "trusted" => Err(InvalidPolicy::NotAllowed { text }),
            _ => Err(InvalidPolicy::Unknown { text }),
        }
    }

    /// The policy as written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confined => "confined",
            Self::Pure => "pure",
        }
    }
}

impl fmt::Display for Policy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn policy_parses_confined_and_pure() -> TestResult {
        assert_eq!(Policy::parse("confined")?, Policy::Confined);
        assert_eq!(Policy::parse("pure")?, Policy::Pure);
        assert_eq!(Policy::default(), Policy::Confined);
        assert_eq!(Policy::Confined.to_string(), "confined");
        assert_eq!(Policy::Pure.to_string(), "pure");
        Ok(())
    }

    #[test]
    fn policy_refuses_trusted_as_not_allowed() -> TestResult {
        let error = Policy::parse("trusted").err().ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "hooks[1].grants.policy");
        assert_eq!(
            Diagnostic::new(location, ManifestError::from(error)).to_string(),
            "packs/core/pack.toml: hooks[1].grants.policy: \"trusted\" is not allowed for pack hooks"
        );
        Ok(())
    }

    #[test]
    fn policy_refuses_an_unknown_value_naming_the_allowed_set() -> TestResult {
        for text in ["foo", "open", ""] {
            let error = Policy::parse(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "hooks[1].grants.policy");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!(
                    "packs/core/pack.toml: hooks[1].grants.policy: \"{text}\" (expected confined, pure)"
                )
            );
        }
        Ok(())
    }
}
