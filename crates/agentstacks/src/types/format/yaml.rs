//! A generic YAML value that frontmatter checks walk by hand.
//!
//! serde-saphyr exports no generic value type, and deserialising straight into a struct stops at
//! the first error. Frontmatter is therefore read into this enum and its keys and types are checked
//! afterwards, so every field error in a file is reported.
//!
//! # Responsibilities
//!
//! - [`YamlValue`] has a variant per YAML kind and keeps mapping keys in source order. Mapping keys
//!   are unique because the parser refuses a duplicate key.
//! - Its `Deserialize` reads any YAML node through `deserialize_any`, and refuses an integer above
//!   `i64::MAX` with the message `integer beyond i64`.
//! - [`YamlValue::kind_word`] gives the type word a wrong-type refusal names.
//!
//! # Non-responsibilities
//!
//! Which keys a document may hold, and what type each has, belongs to the checks over the mapping.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::types::report::TypeWord;

/// The message of the refusal for an unsigned integer above `i64::MAX`, which no [`YamlValue::Int`]
/// holds. It is this crate's own wording, not the parser's.
const INTEGER_BEYOND_I64: &str = "integer beyond i64";

/// One YAML node.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{TypeWord, YamlValue};
///
/// let list = YamlValue::Seq(vec![YamlValue::Str("read".to_owned()), YamlValue::Int(3)]);
/// assert_eq!(list.kind_word(), TypeWord::Array);
/// assert_eq!(YamlValue::Map(Vec::new()).kind_word(), TypeWord::Table);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum YamlValue {
    /// `null`, `~` or an empty document.
    Null,
    /// A YAML 1.1 boolean.
    Bool(bool),
    /// An integer that fits `i64`.
    Int(i64),
    /// A floating-point number.
    Float(f64),
    /// A string.
    Str(String),
    /// A sequence.
    Seq(Vec<Self>),
    /// A mapping, entries in source order.
    Map(Vec<(String, Self)>),
}

impl YamlValue {
    /// The type word naming this value's kind: a sequence is `array`, a mapping is `table`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{TypeWord, YamlValue};
    ///
    /// assert_eq!(YamlValue::Null.kind_word(), TypeWord::Null);
    /// assert_eq!(YamlValue::Bool(true).kind_word().to_string(), "boolean");
    /// ```
    #[must_use]
    pub const fn kind_word(&self) -> TypeWord {
        match self {
            Self::Null => TypeWord::Null,
            Self::Bool(_) => TypeWord::Boolean,
            Self::Int(_) => TypeWord::Integer,
            Self::Float(_) => TypeWord::Float,
            Self::Str(_) => TypeWord::String,
            Self::Seq(_) => TypeWord::Array,
            Self::Map(_) => TypeWord::Table,
        }
    }

    /// The text of a string value, `None` for every other kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::YamlValue;
    ///
    /// assert_eq!(YamlValue::Str("fast".to_owned()).as_str(), Some("fast"));
    /// assert_eq!(YamlValue::Int(1).as_str(), None);
    /// ```
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(text) => Some(text),
            _ => None,
        }
    }
}

/// Reads one node of any kind.
struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = YamlValue;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a YAML value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<YamlValue, E> {
        Ok(YamlValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<YamlValue, E> {
        Ok(YamlValue::Int(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<YamlValue, E> {
        i64::try_from(value)
            .map(YamlValue::Int)
            .map_err(|_| E::custom(INTEGER_BEYOND_I64))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<YamlValue, E> {
        Ok(YamlValue::Float(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<YamlValue, E> {
        Ok(YamlValue::Str(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<YamlValue, E> {
        Ok(YamlValue::Str(value))
    }

    fn visit_unit<E: de::Error>(self) -> Result<YamlValue, E> {
        Ok(YamlValue::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<YamlValue, E> {
        Ok(YamlValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<YamlValue, D::Error> {
        YamlValue::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<YamlValue, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(YamlValue::Seq(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<YamlValue, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry::<String, YamlValue>()? {
            entries.push(entry);
        }
        Ok(YamlValue::Map(entries))
    }
}

impl<'de> Deserialize<'de> for YamlValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::{Error as ValueError, U64Deserializer};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn parse(text: &str) -> Result<YamlValue, serde_saphyr::Error> {
        serde_saphyr::from_str(text)
    }

    fn entry(key: &str, value: YamlValue) -> (String, YamlValue) {
        (key.to_owned(), value)
    }

    #[test]
    fn yaml_value_fills_every_kind_from_nested_mapping() -> TestResult {
        let text = "name: x\nn: 3\nf: 1.5\nb: true\nz: null\nl: [a, 1]\nm:\n  k: v\n";
        let expected = YamlValue::Map(vec![
            entry("name", YamlValue::Str("x".to_owned())),
            entry("n", YamlValue::Int(3)),
            entry("f", YamlValue::Float(1.5)),
            entry("b", YamlValue::Bool(true)),
            entry("z", YamlValue::Null),
            entry(
                "l",
                YamlValue::Seq(vec![YamlValue::Str("a".to_owned()), YamlValue::Int(1)]),
            ),
            entry(
                "m",
                YamlValue::Map(vec![entry("k", YamlValue::Str("v".to_owned()))]),
            ),
        ]);
        assert_eq!(parse(text)?, expected);
        Ok(())
    }

    #[test]
    fn yaml_value_keeps_key_order() -> TestResult {
        let expected = YamlValue::Map(vec![
            entry("z", YamlValue::Int(1)),
            entry("a", YamlValue::Int(2)),
        ]);
        assert_eq!(parse("z: 1\na: 2\n")?, expected);
        Ok(())
    }

    #[test]
    fn yaml_value_reads_yaml_1_1_booleans_and_nulls() -> TestResult {
        let cases = [
            ("k: yes", YamlValue::Bool(true)),
            ("k: on", YamlValue::Bool(true)),
            ("k: y", YamlValue::Bool(true)),
            ("k: True", YamlValue::Bool(true)),
            ("k: no", YamlValue::Bool(false)),
            ("k: off", YamlValue::Bool(false)),
            ("k: ~", YamlValue::Null),
            ("k: null", YamlValue::Null),
        ];
        for (text, value) in cases {
            assert_eq!(
                parse(text)?,
                YamlValue::Map(vec![entry("k", value)]),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn yaml_value_empty_input_is_null() -> TestResult {
        for text in ["", "  \n\t\n", "# just a comment"] {
            assert_eq!(parse(text)?, YamlValue::Null, "{text:?}");
        }
        Ok(())
    }

    #[test]
    fn yaml_value_resolves_aliases() -> TestResult {
        let expected = YamlValue::Map(vec![
            entry("a", YamlValue::Int(1)),
            entry("b", YamlValue::Int(1)),
        ]);
        assert_eq!(parse("a: &x 1\nb: *x\n")?, expected);
        Ok(())
    }

    #[test]
    fn yaml_value_refuses_unsigned_beyond_i64() -> TestResult {
        let beyond = u64::try_from(i64::MAX)? + 1;
        let refused = YamlValue::deserialize(U64Deserializer::<ValueError>::new(beyond));
        assert_eq!(
            refused.err().map(|error| error.to_string()).as_deref(),
            Some("integer beyond i64")
        );
        let at_max =
            YamlValue::deserialize(U64Deserializer::<ValueError>::new(u64::try_from(i64::MAX)?))?;
        assert_eq!(at_max, YamlValue::Int(i64::MAX));
        Ok(())
    }

    #[test]
    fn yaml_value_kind_words() {
        let cases = [
            (YamlValue::Null, "null"),
            (YamlValue::Bool(true), "boolean"),
            (YamlValue::Int(1), "integer"),
            (YamlValue::Float(1.5), "float"),
            (YamlValue::Str(String::new()), "string"),
            (YamlValue::Seq(Vec::new()), "array"),
            (YamlValue::Map(Vec::new()), "table"),
        ];
        for (value, word) in cases {
            assert_eq!(value.kind_word().to_string(), word);
        }
    }
}
