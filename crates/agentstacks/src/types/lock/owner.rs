//! The name a `by` field holds.
//!
//! It is its own type, not a [`PackName`] or a [`ToolName`], because a file or merge may be
//! written by either, and the lock needs one value that holds whichever it was.
//!
//! # Responsibilities
//!
//! - [`Owner`] is the name of the pack or tool that wrote a file or merge. Packs and tools share
//!   one name rule, so an owner parses as either and converts from both without failing.
//!
//! # Non-responsibilities
//!
//! Whether the owner is actually locked belongs to [`Lockfile`](crate::types::Lockfile), which
//! holds the entries.

use std::fmt;
use std::str::FromStr;

use crate::types::name::InvalidName;
use crate::types::name::PackName;
use crate::types::name::ToolName;

/// The name of the pack or tool that wrote a locked file or merge.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Owner, PackName, ToolName};
///
/// // A pack name and a tool name both convert without failing.
/// let from_pack = Owner::from(&PackName::new("core")?);
/// let from_tool = Owner::from(&ToolName::new("claude")?);
/// assert_eq!(from_pack.as_str(), "core");
/// assert_eq!(from_tool.to_string(), "claude");
/// assert_eq!(Owner::new("core")?, from_pack);
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Owner(String);

impl Owner {
    /// Parses `value` as an owner: a valid pack or tool name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidName`] when `value` is not `kebab-case`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Owner;
    ///
    /// assert_eq!(Owner::new("team-lint")?.as_str(), "team-lint");
    /// assert_eq!(
    ///     Owner::new("Core").err().map(|error| error.to_string()).as_deref(),
    ///     Some("\"Core\" is not a valid name")
    /// );
    /// # Ok::<(), agentstacks::types::InvalidName>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, InvalidName> {
        PackName::new(value).map(|name| Self::from(&name))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Owner {
    type Err = InvalidName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Owner {
    type Error = InvalidName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<&PackName> for Owner {
    fn from(name: &PackName) -> Self {
        Self(name.as_str().to_owned())
    }
}

impl From<&ToolName> for Owner {
    fn from(name: &ToolName) -> Self {
        Self(name.as_str().to_owned())
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn owner_new_takes_names_and_converts_from_pack_and_tool() -> TestResult {
        let parsed = Owner::new("core")?;
        assert_eq!(parsed.to_string(), "core");
        assert_eq!(parsed.as_str(), "core");

        let refused = Owner::new("Kore").err().ok_or("expected a refusal")?;
        assert_eq!(refused.to_string(), "\"Kore\" is not a valid name");

        assert_eq!(Owner::from(&PackName::new("core")?), parsed);
        assert_eq!(Owner::from(&ToolName::new("claude")?).to_string(), "claude");
        Ok(())
    }

    #[test]
    fn owner_orders_by_name() -> TestResult {
        assert!(Owner::new("core")? < Owner::new("team-lint")?);
        Ok(())
    }

    #[test]
    fn owner_from_str_and_try_from_match_new() -> TestResult {
        assert_eq!("core".parse::<Owner>()?, Owner::new("core")?);
        assert_eq!(Owner::try_from("claude")?, Owner::new("claude")?);
        assert_eq!("Core".parse::<Owner>().err(), Owner::new("Core").err());
        assert_eq!(Owner::try_from("Core").err(), Owner::new("Core").err());
        Ok(())
    }
}
