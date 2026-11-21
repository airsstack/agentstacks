//! A value an option can hold.
//!
//! It is separate from `OptionType` because a type is a declaration and a value is data: a default
//! holds a value, and the `Display` rendering is that of the value.
//!
//! # Responsibilities
//!
//! - [`OptionValue`] is a boolean, an integer (`i64`) or a string: the three option types.
//! - [`OptionValue::option_type`] names the type of a held value.
//! - Its `Display` is the rendering a token takes: a boolean as `true` or `false`, an integer in
//!   decimal, a string as it is.
//!
//! # Non-responsibilities
//!
//! Checking that a value fits an option's declared type belongs to the code that reads the
//! declaration.

use std::fmt;

use toml::Value;

use crate::types::manifest::OptionType;

/// A value of one of the three option types.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{OptionType, OptionValue};
///
/// assert_eq!(OptionValue::Integer(-3).to_string(), "-3");
/// assert_eq!(OptionValue::Boolean(true).to_string(), "true");
/// assert_eq!(OptionValue::String("{{skill x}}".to_owned()).to_string(), "{{skill x}}");
/// assert_eq!(OptionValue::Integer(20).option_type(), OptionType::Integer);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue {
    /// `true` or `false`.
    Boolean(bool),
    /// A 64-bit signed integer.
    Integer(i64),
    /// A string.
    String(String),
}

impl OptionValue {
    /// Reads a TOML value; `None` for a type options do not have (float, datetime, array, table).
    pub(crate) fn from_toml(value: &Value) -> Option<Self> {
        match value {
            Value::Boolean(flag) => Some(Self::Boolean(*flag)),
            Value::Integer(number) => Some(Self::Integer(*number)),
            Value::String(text) => Some(Self::String(text.clone())),
            _ => None,
        }
    }

    /// The type of the held value.
    #[must_use]
    pub const fn option_type(&self) -> OptionType {
        match self {
            Self::Boolean(_) => OptionType::Boolean,
            Self::Integer(_) => OptionType::Integer,
            Self::String(_) => OptionType::String,
        }
    }
}

impl fmt::Display for OptionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(flag) => flag.fmt(f),
            Self::Integer(number) => number.fmt(f),
            Self::String(text) => f.write_str(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_value_renders_per_spec() {
        let cases = [
            (OptionValue::Boolean(true), "true"),
            (OptionValue::Boolean(false), "false"),
            (OptionValue::Integer(20), "20"),
            (OptionValue::Integer(-3), "-3"),
            (OptionValue::String("{{skill x}}".to_owned()), "{{skill x}}"),
            (OptionValue::String(String::new()), ""),
        ];
        for (value, rendered) in cases {
            assert_eq!(value.to_string(), rendered);
        }
    }

    #[test]
    fn option_value_reads_the_three_toml_types() {
        assert_eq!(
            OptionValue::from_toml(&Value::Boolean(true)),
            Some(OptionValue::Boolean(true))
        );
        assert_eq!(
            OptionValue::from_toml(&Value::Integer(20)),
            Some(OptionValue::Integer(20))
        );
        assert_eq!(
            OptionValue::from_toml(&Value::String("x".to_owned())),
            Some(OptionValue::String("x".to_owned()))
        );
    }

    #[test]
    fn option_value_refuses_other_toml_types() {
        assert_eq!(OptionValue::from_toml(&Value::Float(1.5)), None);
        assert_eq!(OptionValue::from_toml(&Value::Array(Vec::new())), None);
    }

    #[test]
    fn option_value_names_its_type() {
        assert_eq!(
            OptionValue::Boolean(false).option_type(),
            OptionType::Boolean
        );
        assert_eq!(OptionValue::Integer(1).option_type(), OptionType::Integer);
        assert_eq!(
            OptionValue::String(String::new()).option_type(),
            OptionType::String
        );
    }
}
