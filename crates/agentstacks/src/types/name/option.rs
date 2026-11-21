//! The name of a pack option.
//!
//! It differs from the `kebab-case` names only by its separator, and is a type of its own so an
//! option name cannot be used where an item name is wanted. The shared rule lives in a private
//! file.
//!
//! # Responsibilities
//!
//! - [`OptionName`] is a `snake_case` name that parses on construction, so a held value is always
//!   valid: 1 to 64 bytes of `a`-`z`, `0`-`9` and `_`, starting with a letter.
//!
//! # Non-responsibilities
//!
//! Whether a pack declares the option belongs to the checks over a whole pack.

use std::fmt;
use std::str::FromStr;

use super::kebab::is_name_with_separator;
use crate::types::name::InvalidOptionName;

/// The name of a pack option, matching `^[a-z][a-z0-9_]{0,63}$`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::OptionName;
///
/// let name: OptionName = "max_depth".parse()?;
/// assert_eq!(name.to_string(), "max_depth");
/// assert!("max-depth".parse::<OptionName>().is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionName(String);

impl OptionName {
    /// Parses `value` as an option name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOptionName`] when `value` is not `snake_case`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::OptionName;
    ///
    /// assert_eq!(OptionName::new("max_depth")?.as_str(), "max_depth");
    /// let refused = OptionName::new("max-depth").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("\"max-depth\" is not a valid option name"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, InvalidOptionName> {
        if is_name_with_separator(value, b'_') {
            Ok(Self(value.to_owned()))
        } else {
            Err(InvalidOptionName::new(value))
        }
    }

    /// The name as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::OptionName;
    ///
    /// assert_eq!(OptionName::new("verbose")?.as_str(), "verbose");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for OptionName {
    type Err = InvalidOptionName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for OptionName {
    type Error = InvalidOptionName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for OptionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_name_accepts_snake_case() {
        for value in ["archive_keep", "style_reinject", &"a".repeat(64)] {
            assert_eq!(
                OptionName::new(value).as_ref().map(OptionName::as_str),
                Ok(value)
            );
        }
    }

    #[test]
    fn option_name_refuses_bad_start_bad_character_overlong_and_empty() {
        for value in ["a-b", "Archive", "9x", "_x", &"a".repeat(65), ""] {
            assert_eq!(
                OptionName::new(value).map_err(|e| e.to_string()),
                Err(format!("\"{value}\" is not a valid option name"))
            );
        }
    }

    #[test]
    fn from_str_try_from_and_display_agree_with_new() {
        assert_eq!(
            "archive_keep".parse::<OptionName>(),
            OptionName::new("archive_keep")
        );
        assert_eq!(
            OptionName::try_from("archive_keep"),
            OptionName::new("archive_keep")
        );
        assert_eq!(
            OptionName::new("archive_keep").map(|n| n.to_string()),
            Ok("archive_keep".to_owned())
        );
    }
}
