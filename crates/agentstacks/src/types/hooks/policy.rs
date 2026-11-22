//! The sandbox policy a pack hook runs under.
//!
//! It is its own enum so that `trusted` is unrepresentable for a pack hook: a closed type makes that
//! a fact of the type instead of a check every reader repeats.
//!
//! # Responsibilities
//!
//! - [`Policy`] is `confined`, the only policy api 0 allows, and the default. The airsl policy
//!   `trusted` is deliberately not representable, because a pack hook may not run with it, and
//!   `pure` is refused because airsl has no `require` under it.
//!
//! # Non-responsibilities
//!
//! Turning a policy into airsl flags belongs to the extension host.

use std::fmt;
use std::str::FromStr;

use crate::types::hooks::InvalidPolicy;

/// A policy a pack hook may use.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Policy;
///
/// assert_eq!(Policy::new("confined")?, Policy::Confined);
/// assert_eq!(Policy::default(), Policy::Confined);
///
/// let refused = Policy::new("trusted").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"trusted\" is not allowed for pack hooks");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Policy {
    /// Reads and writes only what the grants name.
    #[default]
    Confined,
}

impl Policy {
    /// Parses a `grants.policy` value.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPolicy::PureUnsupported`] for `pure`, [`InvalidPolicy::NotAllowed`] for
    /// `trusted`, and [`InvalidPolicy::Unknown`] for any other value but `confined`.
    pub fn new(text: &str) -> Result<Self, InvalidPolicy> {
        let text = text.to_owned();
        match text.as_str() {
            "confined" => Ok(Self::Confined),
            "pure" => Err(InvalidPolicy::PureUnsupported),
            "trusted" => Err(InvalidPolicy::NotAllowed { text }),
            _ => Err(InvalidPolicy::Unknown { text }),
        }
    }

    /// The policy as written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confined => "confined",
        }
    }
}

impl fmt::Display for Policy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Policy {
    type Err = InvalidPolicy;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for Policy {
    type Error = InvalidPolicy;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn policy_parses_confined() -> TestResult {
        assert_eq!(Policy::new("confined")?, Policy::Confined);
        assert_eq!(Policy::default(), Policy::Confined);
        assert_eq!(Policy::Confined.to_string(), "confined");
        Ok(())
    }

    #[test]
    fn policy_refuses_pure_in_api_0() -> TestResult {
        let error = Policy::new("pure").err().ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "hooks[1].grants.policy");
        assert_eq!(
            Diagnostic::new(location, ManifestError::from(error)).to_string(),
            "packs/core/pack.toml: hooks[1].grants.policy: \"pure\" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)"
        );
        Ok(())
    }

    #[test]
    fn policy_refuses_trusted_as_not_allowed() -> TestResult {
        let error = Policy::new("trusted").err().ok_or("expected a refusal")?;
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
            let error = Policy::new(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "hooks[1].grants.policy");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!(
                    "packs/core/pack.toml: hooks[1].grants.policy: \"{text}\" (expected confined)"
                )
            );
        }
        Ok(())
    }

    #[test]
    fn policy_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<Policy, InvalidPolicy>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["confined", "pure", "trusted"] {
            let expected = shown(Policy::new(text));
            assert_eq!(shown(text.parse::<Policy>()), expected);
            assert_eq!(shown(Policy::try_from(text)), expected);
        }
        assert!(Policy::new("confined").is_ok());
        assert!(Policy::new("pure").is_err());
        assert!(Policy::new("trusted").is_err());
    }
}
