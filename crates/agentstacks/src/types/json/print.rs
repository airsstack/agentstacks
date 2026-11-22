//! Printing a JSON value as text.
//!
//! # Responsibilities
//!
//! - [`Ending`]: whether the printed text ends with a newline.
//! - [`print()`]: pretty-prints a value with a given indent and line ending.
//!
//! # Non-responsibilities
//!
//! Reading the indent and the ending from a text belongs to [`Indent`] and [`Ending::of`]'s
//! callers, and choosing the value belongs to the code that edits the document.

use serde::Serialize;
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

use super::JsonEditError;
use super::indent::Indent;
use crate::types::report::ParserMessage;

/// Whether printed text ends with a newline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::types::json) enum Ending {
    /// The text ends with one `\n`.
    Newline,
    /// The text ends with the closing brace.
    Absent,
}

impl Ending {
    /// `Newline` exactly when `text` ends with `\n`, so CRLF text ending in `\r\n` is `Newline`.
    pub(in crate::types::json) fn of(text: &str) -> Self {
        if text.ends_with('\n') {
            Self::Newline
        } else {
            Self::Absent
        }
    }
}

/// Pretty-prints `value` with `indent`, one `\n` after every line break and, for
/// [`Ending::Newline`], one `\n` at the end.
///
/// Numbers come out in `serde_json`'s normal form, since a parsed value holds no original
/// spelling.
///
/// # Errors
///
/// Returns [`JsonEditError::Parse`] with `serde_json`'s message if the value cannot be
/// serialised. A parsed value always can, so no caller sees this.
pub(in crate::types::json) fn print(
    value: &Value,
    indent: &Indent,
    ending: Ending,
) -> Result<String, JsonEditError> {
    let mut bytes = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut serializer = Serializer::with_formatter(&mut bytes, formatter);
    value.serialize(&mut serializer).map_err(refused)?;
    let mut text = String::from_utf8(bytes).map_err(|error| refused(error.utf8_error()))?;
    if ending == Ending::Newline {
        text.push('\n');
    }
    Ok(text)
}

/// The refusal that carries `error`'s own words.
fn refused(error: impl std::fmt::Display) -> JsonEditError {
    JsonEditError::Parse(ParserMessage::new(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    type Test = Result<(), Box<dyn std::error::Error>>;

    /// A four-space layout whose number spellings a re-print changes.
    const SOURCE: &str = "{\n    \"permissions\": {\n        \"allow\": [\n            \"Bash(x:*)\"\n        ]\n    },\n    \"zeta\": 1,\n    \"alpha\": 12345678901234567890123,\n    \"d\": 0.10,\n    \"hooks\": {}\n}";

    /// The printed form of `SOURCE` for a one-level indent `unit`.
    fn printed(unit: &str) -> String {
        let (one, two, three) = (unit.to_owned(), unit.repeat(2), unit.repeat(3));
        format!(
            "{{\n{one}\"permissions\": {{\n{two}\"allow\": [\n{three}\"Bash(x:*)\"\n{two}]\n{one}}},\n{one}\"zeta\": 1,\n{one}\"alpha\": 1.2345678901234568e+22,\n{one}\"d\": 0.1,\n{one}\"hooks\": {{}}\n}}"
        )
    }

    fn indent_of(text: &str) -> Indent {
        Indent::detect(&format!("{{\n{text}\"a\": 1\n}}"))
    }

    #[test]
    fn ending_of_follows_the_last_byte() {
        assert_eq!(Ending::of("{}\n"), Ending::Newline);
        assert_eq!(Ending::of("{}\r\n"), Ending::Newline);
        assert_eq!(Ending::of("{}"), Ending::Absent);
        assert_eq!(Ending::of(""), Ending::Absent);
    }

    #[test]
    fn print_uses_the_given_indent() -> Test {
        let value: Value = serde_json::from_str(SOURCE)?;
        for unit in ["  ", "    ", "\t"] {
            let text = print(&value, &indent_of(unit), Ending::Absent)?;
            assert_eq!(text, printed(unit));
        }
        Ok(())
    }

    #[test]
    fn print_appends_one_newline_for_newline_ending() -> Test {
        let value: Value = serde_json::from_str("{\"a\":[1]}")?;
        let text = print(&value, &indent_of("\t"), Ending::Newline)?;
        assert_eq!(text, "{\n\t\"a\": [\n\t\t1\n\t]\n}\n");
        Ok(())
    }

    #[test]
    fn print_writes_no_carriage_return() -> Test {
        let value: Value = serde_json::from_str(&SOURCE.replace('\n', "\r\n"))?;
        for indent in ["  ", "    ", "\t"] {
            let text = print(&value, &indent_of(indent), Ending::Newline)?;
            assert!(!text.contains('\r'));
        }
        Ok(())
    }
}
