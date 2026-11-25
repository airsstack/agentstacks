//! The name of a binary a pack requires or a hook may run.
//!
//! It is a type of its own, wrapping [`ItemName`], so a field that wants a binary cannot be
//! handed a skill or agent name, while the naming rule stays defined once.
//!
//! # Responsibilities
//!
//! - [`BinaryName`] is a `requires.binaries` or `grants.exec` entry, valid on construction under
//!   the same name rule as every other name.
//! - [`AIRSL`] is the name of the `airsl` binary, written once.
//!
//! # Non-responsibilities
//!
//! Whether the binary exists on the machine belongs to the checks that run at apply time.

use std::fmt;
use std::str::FromStr;

use crate::types::name::InvalidName;
use crate::types::name::item::ItemName;

/// The name of the `airsl` binary, which every hook may run without the pack requiring it.
pub const AIRSL: &str = "airsl";

/// The name of a binary.
///
/// # Examples
///
/// ```
/// use agentstacks::types::BinaryName;
///
/// let name = BinaryName::new("git")?;
/// assert_eq!(name.to_string(), "git");
/// assert!(BinaryName::new("Git").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinaryName(ItemName);

impl BinaryName {
    /// Parses `text` as a binary name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidName`] when `text` is not a valid name.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::BinaryName;
    ///
    /// assert_eq!(BinaryName::new("ripgrep")?.as_str(), "ripgrep");
    /// let refused = BinaryName::new("Ripgrep").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("\"Ripgrep\" is not a valid name"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidName> {
        Ok(Self(ItemName::new(text)?))
    }

    /// The name as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::BinaryName;
    ///
    /// assert_eq!(BinaryName::new("airsl")?.as_str(), "airsl");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for BinaryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for BinaryName {
    type Err = InvalidName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for BinaryName {
    type Error = InvalidName;

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
    fn binary_name_accepts_the_real_binaries() -> TestResult {
        for text in ["git", "cmux", "airsl", "tar"] {
            assert_eq!(BinaryName::new(text)?.as_str(), text);
            assert_eq!(BinaryName::new(text)?.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn binary_name_refuses_an_invalid_name() -> TestResult {
        let error = BinaryName::new("Git").err().ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "requires.binaries");
        assert_eq!(
            Diagnostic::new(location, ManifestError::from(error)).to_string(),
            "packs/core/pack.toml: requires.binaries: \"Git\" is not a valid name"
        );
        Ok(())
    }

    #[test]
    fn binary_name_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<BinaryName, InvalidName>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["git", "Git"] {
            let expected = shown(BinaryName::new(text));
            assert_eq!(shown(text.parse::<BinaryName>()), expected);
            assert_eq!(shown(BinaryName::try_from(text)), expected);
        }
        assert!(BinaryName::new("git").is_ok());
        assert!(BinaryName::new("Git").is_err());
    }
}
