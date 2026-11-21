//! The name of a pack item.
//!
//! It is a type of its own, apart from [`PackName`](crate::types::PackName), so an item name
//! cannot be passed where a pack name is wanted, even though both follow the same rule.
//!
//! # Responsibilities
//!
//! - [`ItemName`] is a `kebab-case` name that parses on construction, so a held value is always
//!   valid.
//!
//! # Non-responsibilities
//!
//! Callers whose message words a bad name differently map [`InvalidName`] into their own message.

use std::fmt;
use std::str::FromStr;

use super::kebab::check_kebab_name;
use crate::types::name::InvalidName;

/// The name of a pack item such as a skill or an agent: 1 to 64 bytes of `a`-`z`, `0`-`9` and
/// `-`, starting with a letter.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ItemName;
///
/// let name: ItemName = "my-skill".parse()?;
/// assert_eq!(name.to_string(), "my-skill");
/// assert!("My_Skill".parse::<ItemName>().is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemName(String);

impl ItemName {
    /// Parses `value` as an item name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidName`] when `value` is not `kebab-case`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ItemName;
    ///
    /// assert_eq!(ItemName::new("my-name")?.as_str(), "my-name");
    /// let refused = ItemName::new("My_Name").err().map(|error| error.to_string());
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
    /// use agentstacks::types::ItemName;
    ///
    /// assert_eq!(ItemName::new("core")?.as_str(), "core");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ItemName {
    type Err = InvalidName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ItemName {
    type Error = InvalidName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for ItemName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_name_parses_and_refuses() {
        assert_eq!(
            ItemName::new("discuss").as_ref().map(ItemName::as_str),
            Ok("discuss")
        );
        assert_eq!(
            ItemName::new("My_Skill").map_err(|e| e.to_string()),
            Err("\"My_Skill\" is not a valid name".to_owned())
        );
    }

    #[test]
    fn from_str_try_from_and_display_agree_with_new() {
        assert_eq!("discuss".parse::<ItemName>(), ItemName::new("discuss"));
        assert_eq!(ItemName::try_from("discuss"), ItemName::new("discuss"));
        assert_eq!(
            ItemName::new("discuss").map(|n| n.to_string()),
            Ok("discuss".to_owned())
        );
    }
}
