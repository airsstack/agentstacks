//! The name of a tool.
//!
//! It is a type of its own, so a tool name cannot be passed where a pack or item name is wanted,
//! even though all three follow the same rule.
//!
//! # Responsibilities
//!
//! - [`ToolName`] is a `kebab-case` name that parses on construction, so a held value is always
//!   valid.
//!
//! # Non-responsibilities
//!
//! Callers whose message words a bad name differently map [`InvalidName`] into their own message.

use std::fmt;
use std::str::FromStr;

use super::kebab::check_kebab_name;
use crate::types::name::InvalidName;

/// The name of a tool: 1 to 64 bytes of `a`-`z`, `0`-`9` and `-`, starting with a letter.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ToolName;
///
/// let name: ToolName = "claude".parse()?;
/// assert_eq!(name.to_string(), "claude");
/// assert!("Claude".parse::<ToolName>().is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolName(String);

impl ToolName {
    /// Parses `value` as a tool name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidName`] when `value` is not `kebab-case`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ToolName;
    ///
    /// assert_eq!(ToolName::new("my-name")?.as_str(), "my-name");
    /// let refused = ToolName::new("My_Name").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("\"My_Name\" is not a valid name"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, InvalidName> {
        check_kebab_name(value)?;
        Ok(Self(value.to_owned()))
    }

    /// The name as text.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ToolName;
    ///
    /// assert_eq!(ToolName::new("core")?.as_str(), "core");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ToolName {
    type Err = InvalidName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ToolName {
    type Error = InvalidName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_name_parses_and_refuses() {
        assert_eq!(
            ToolName::new("claude").as_ref().map(ToolName::as_str),
            Ok("claude")
        );
        assert_eq!(
            ToolName::new("Claude").map_err(|e| e.to_string()),
            Err("\"Claude\" is not a valid name".to_owned())
        );
    }

    #[test]
    fn from_str_try_from_and_display_agree_with_new() {
        assert_eq!("claude".parse::<ToolName>(), ToolName::new("claude"));
        assert_eq!(ToolName::try_from("claude"), ToolName::new("claude"));
        assert_eq!(
            ToolName::new("claude").map(|n| n.to_string()),
            Ok("claude".to_owned())
        );
    }
}
