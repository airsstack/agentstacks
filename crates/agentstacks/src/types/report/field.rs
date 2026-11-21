//! The generic message shapes every field uses when it has no message of its own.
//!
//! The shapes are one enum so a missing key, a wrong type, an empty string and an unknown key read
//! the same wherever a field is refused.
//!
//! # Responsibilities
//!
//! - [`FieldError`] has four variants, displayed as `required`, `expected <type>, got <type>`,
//!   `empty` and `unknown key`. The location in front of the message names the key.
//! - [`FieldError::check_not_empty`] is the one blank-string check: it refuses a string that is
//!   empty or holds only whitespace.
//!
//! # Non-responsibilities
//!
//! Messages a field words differently belong to the error enum of the module that owns the field.

use crate::types::report::TypeWord;

/// A field refused in one of the four generic ways.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{FieldError, TypeWord};
///
/// assert_eq!(FieldError::Required.to_string(), "required");
/// assert_eq!(
///     FieldError::WrongType { expected: TypeWord::String, got: TypeWord::Integer }.to_string(),
///     "expected string, got integer",
/// );
/// assert_eq!(FieldError::Empty.to_string(), "empty");
/// assert_eq!(FieldError::UnknownKey.to_string(), "unknown key");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// A required key is missing.
    #[error("required")]
    Required,
    /// A value has a type the field does not accept.
    #[error("expected {expected}, got {got}")]
    WrongType {
        /// The type the field accepts.
        expected: TypeWord,
        /// The type the value has.
        got: TypeWord,
    },
    /// A string is empty or only whitespace.
    #[error("empty")]
    Empty,
    /// A key the format does not define.
    #[error("unknown key")]
    UnknownKey,
}

impl FieldError {
    /// Refuses an empty or whitespace-only string.
    ///
    /// # Errors
    ///
    /// Returns [`FieldError::Empty`] when `value` is empty or holds only whitespace.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::FieldError;
    ///
    /// assert_eq!(FieldError::check_not_empty("core"), Ok(()));
    /// assert_eq!(FieldError::check_not_empty("  "), Err(FieldError::Empty));
    /// ```
    pub fn check_not_empty(value: &str) -> Result<(), Self> {
        if value.trim().is_empty() {
            Err(Self::Empty)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_displays_required() {
        assert_eq!(FieldError::Required.to_string(), "required");
    }

    #[test]
    fn wrong_type_displays_expected_then_got() {
        let error = FieldError::WrongType {
            expected: TypeWord::Integer,
            got: TypeWord::Datetime,
        };
        assert_eq!(error.to_string(), "expected integer, got datetime");
    }

    #[test]
    fn wrong_type_uses_yaml_words() {
        let error = FieldError::WrongType {
            expected: TypeWord::Table,
            got: TypeWord::Null,
        };
        assert_eq!(error.to_string(), "expected table, got null");
    }

    #[test]
    fn empty_displays_empty() {
        assert_eq!(FieldError::Empty.to_string(), "empty");
    }

    #[test]
    fn unknown_key_displays_unknown_key() {
        assert_eq!(FieldError::UnknownKey.to_string(), "unknown key");
    }

    #[test]
    fn check_not_empty_refuses_empty_string() {
        assert_eq!(FieldError::check_not_empty(""), Err(FieldError::Empty));
    }

    #[test]
    fn check_not_empty_refuses_whitespace_only() {
        assert_eq!(FieldError::check_not_empty(" \t "), Err(FieldError::Empty));
    }

    #[test]
    fn check_not_empty_accepts_text_with_surrounding_spaces() {
        assert_eq!(FieldError::check_not_empty(" core "), Ok(()));
    }
}
