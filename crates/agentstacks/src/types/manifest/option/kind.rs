//! The type of a pack option.
//!
//! It is its own enum because both a declaration and a value name a type, and one closed vocabulary
//! keeps their wording in step.
//!
//! # Responsibilities
//!
//! - [`OptionType`] is `boolean`, `integer` or `string`; an integer is an `i64`.
//! - [`OptionType::new`] reads the `type` field, and [`OptionType::type_word`] names the type in
//!   the words wrong-type messages use.
//!
//! # Non-responsibilities
//!
//! Checking a value against the type belongs to the code that holds the value.

use std::fmt;
use std::str::FromStr;

use crate::types::manifest::InvalidOptionType;
use crate::types::report::TypeWord;

/// A type an option may declare.
///
/// # Examples
///
/// ```
/// use agentstacks::types::OptionType;
///
/// assert_eq!(OptionType::new("integer")?, OptionType::Integer);
/// assert_eq!(OptionType::Boolean.to_string(), "boolean");
///
/// let refused = OptionType::new("number").err().ok_or("expected a refusal")?;
/// assert_eq!(
///     refused.to_string(),
///     "\"number\" (expected boolean, integer, string)"
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionType {
    /// `true` or `false`.
    Boolean,
    /// A 64-bit signed integer.
    Integer,
    /// A string.
    String,
}

impl OptionType {
    /// Parses the `type` field of an option.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOptionType`] for anything but `boolean`, `integer` and `string`.
    pub fn new(text: &str) -> Result<Self, InvalidOptionType> {
        match text {
            "boolean" => Ok(Self::Boolean),
            "integer" => Ok(Self::Integer),
            "string" => Ok(Self::String),
            _ => Err(InvalidOptionType::new(text)),
        }
    }

    /// The type word a value of this type has.
    #[must_use]
    pub const fn type_word(self) -> TypeWord {
        match self {
            Self::Boolean => TypeWord::Boolean,
            Self::Integer => TypeWord::Integer,
            Self::String => TypeWord::String,
        }
    }
}

impl fmt::Display for OptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.type_word().fmt(f)
    }
}

impl FromStr for OptionType {
    type Err = InvalidOptionType;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for OptionType {
    type Error = InvalidOptionType;

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
    fn option_type_parses_the_three_types() -> TestResult {
        assert_eq!(OptionType::new("boolean")?, OptionType::Boolean);
        assert_eq!(OptionType::new("integer")?, OptionType::Integer);
        assert_eq!(OptionType::new("string")?, OptionType::String);
        assert_eq!(OptionType::Integer.to_string(), "integer");
        assert_eq!(OptionType::Integer.type_word(), TypeWord::Integer);
        Ok(())
    }

    #[test]
    fn option_type_refuses_others() -> TestResult {
        let error = OptionType::new("number")
            .err()
            .ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "options.keep.type");
        assert_eq!(
            Diagnostic::new(location, ManifestError::from(error)).to_string(),
            "packs/core/pack.toml: options.keep.type: \"number\" (expected boolean, integer, string)"
        );
        Ok(())
    }

    #[test]
    fn option_type_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<OptionType, InvalidOptionType>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["integer", "number"] {
            let expected = shown(OptionType::new(text));
            assert_eq!(shown(text.parse::<OptionType>()), expected);
            assert_eq!(shown(OptionType::try_from(text)), expected);
        }
        assert!(OptionType::new("integer").is_ok());
        assert!(OptionType::new("number").is_err());
    }
}
