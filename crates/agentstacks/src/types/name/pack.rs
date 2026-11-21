//! The name of a pack.
//!
//! It is a type of its own, apart from [`ItemName`](crate::types::ItemName), so a pack name
//! cannot be passed where an item name is wanted, even though both follow the same rule.
//!
//! # Responsibilities
//!
//! - [`PackName`] is a `kebab-case` name that parses on construction, so a held value is always
//!   valid.
//!
//! # Non-responsibilities
//!
//! Callers whose message words a bad name differently map [`InvalidName`] into their own message.

use std::fmt;
use std::str::FromStr;

use super::kebab::check_kebab_name;
use crate::types::name::InvalidName;

/// The name of a pack: 1 to 64 bytes of `a`-`z`, `0`-`9` and `-`, starting with a letter.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
///
/// let name: PackName = "core".parse()?;
/// assert_eq!(name.to_string(), "core");
/// assert!("Core".parse::<PackName>().is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackName(String);

impl PackName {
    /// Parses `value` as a pack name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidName`] when `value` is not `kebab-case`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::PackName;
    ///
    /// assert_eq!(PackName::new("my-name")?.as_str(), "my-name");
    /// let refused = PackName::new("My_Name").err().map(|error| error.to_string());
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
    /// use agentstacks::types::PackName;
    ///
    /// assert_eq!(PackName::new("core")?.as_str(), "core");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PackName {
    type Err = InvalidName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for PackName {
    type Error = InvalidName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for PackName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_name_parses_through_new_from_str_and_try_from() {
        for value in ["core", "guideline-rust"] {
            let parsed = [
                PackName::new(value),
                value.parse::<PackName>(),
                PackName::try_from(value),
            ];
            for name in parsed {
                assert_eq!(name.as_ref().map(PackName::as_str), Ok(value));
                assert_eq!(name.map(|n| n.to_string()), Ok(value.to_owned()));
            }
        }
    }

    #[test]
    fn pack_name_refuses_option_style_name() {
        let error = PackName::new("archive_keep").map_err(|e| e.to_string());
        assert_eq!(
            error,
            Err("\"archive_keep\" is not a valid name".to_owned())
        );
    }
}
