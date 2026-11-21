//! The name of an environment variable a hook may read.
//!
//! Environment variable names are upper-case, unlike the `kebab-case` rule behind the other
//! names, so they have their own type and their own refusal rather than a variant of the shared
//! one.
//!
//! # Responsibilities
//!
//! - [`EnvName`] is a name matching `^[A-Z_][A-Z0-9_]*$`, checked by hand on construction.
//!
//! # Non-responsibilities
//!
//! Whether the variable is set belongs to the run of the hook.

use std::fmt;
use std::str::FromStr;

use crate::types::name::InvalidEnvName;

/// An environment variable name.
///
/// # Examples
///
/// ```
/// use agentstacks::types::EnvName;
///
/// let name = EnvName::new("HOME")?;
/// assert_eq!(name.to_string(), "HOME");
/// assert!(EnvName::new("home").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvName(String);

impl EnvName {
    /// Parses `text` as an environment variable name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidEnvName`] when `text` is empty, starts with a digit, or holds
    /// anything but `A-Z`, `0-9` and `_`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::EnvName;
    ///
    /// assert_eq!(EnvName::new("HOME")?.as_str(), "HOME");
    /// let refused = EnvName::new("1HOME").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("\"1HOME\" is not an environment variable name"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(text: &str) -> Result<Self, InvalidEnvName> {
        let bytes = text.as_bytes();
        let starts_well = bytes
            .first()
            .is_some_and(|b| b.is_ascii_uppercase() || *b == b'_');
        let continues_well = bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_');
        if starts_well && continues_well {
            Ok(Self(text.to_owned()))
        } else {
            Err(InvalidEnvName::new(text))
        }
    }

    /// The name as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::EnvName;
    ///
    /// assert_eq!(EnvName::new("PATH")?.as_str(), "PATH");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EnvName {
    type Err = InvalidEnvName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for EnvName {
    type Error = InvalidEnvName;

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
    fn env_name_accepts_the_real_grants() -> TestResult {
        for text in [
            "HOME",
            "TMPDIR",
            "AIRSSTACK_HOME",
            "AIRSSTACK_ENFORCE_REGISTRY",
            "CLAUDE_PLUGIN_OPTION_STYLE_REINJECT",
            "_X",
        ] {
            assert_eq!(EnvName::new(text)?.as_str(), text);
            assert_eq!(EnvName::new(text)?.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn env_name_refuses_other_shapes() -> TestResult {
        for text in ["home", "1X", "", "A-B", "A B"] {
            let error = EnvName::new(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "hooks[1].grants.env");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!(
                    "packs/core/pack.toml: hooks[1].grants.env: \"{text}\" is not an environment variable name"
                )
            );
        }
        Ok(())
    }

    #[test]
    fn env_name_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<EnvName, InvalidEnvName>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["HOME", "home"] {
            let expected = shown(EnvName::new(text));
            assert_eq!(shown(text.parse::<EnvName>()), expected);
            assert_eq!(shown(EnvName::try_from(text)), expected);
        }
        assert!(EnvName::new("HOME").is_ok());
        assert!(EnvName::new("home").is_err());
    }
}
