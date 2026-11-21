//! The closed set of type words a "wrong type" refusal names.
//!
//! The words are a closed enum so no parser invents a type name as a free string; a refusal can
//! only say what the format actually distinguishes.
//!
//! # Responsibilities
//!
//! - Names the eight type words: `boolean`, `integer`, `float`, `string`, `datetime`, `array`,
//!   `table` and `null`, each displayed as its lowercase word.
//! - `Null` occurs only for YAML values. A YAML sequence is `Array` and a YAML mapping is `Table`,
//!   so TOML and YAML use the same words.
//!
//! # Non-responsibilities
//!
//! Mapping a TOML or YAML value kind onto a word belongs to the parser for that format.

use std::fmt;

/// One of the eight type words used in `expected <type>, got <type>`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::TypeWord;
///
/// assert_eq!(TypeWord::Datetime.to_string(), "datetime");
/// assert_eq!(TypeWord::Null.to_string(), "null");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeWord {
    /// `boolean`.
    Boolean,
    /// `integer`.
    Integer,
    /// `float`.
    Float,
    /// `string`.
    String,
    /// `datetime`.
    Datetime,
    /// `array`; also a YAML sequence.
    Array,
    /// `table`; also a YAML mapping.
    Table,
    /// `null`; YAML values only.
    Null,
}

impl fmt::Display for TypeWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::String => "string",
            Self::Datetime => "datetime",
            Self::Array => "array",
            Self::Table => "table",
            Self::Null => "null",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_word_displays_its_word() {
        let cases = [
            (TypeWord::Boolean, "boolean"),
            (TypeWord::Integer, "integer"),
            (TypeWord::Float, "float"),
            (TypeWord::String, "string"),
            (TypeWord::Datetime, "datetime"),
            (TypeWord::Array, "array"),
            (TypeWord::Table, "table"),
            (TypeWord::Null, "null"),
        ];
        for (word, text) in cases {
            assert_eq!(word.to_string(), text);
        }
    }
}
