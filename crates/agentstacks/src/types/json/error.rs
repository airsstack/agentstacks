//! Every refusal of a `json.edit` call.
//!
//! The enum's `Display` is the text after the location, the extension file and line of the caller.
//! Every message starts with the function name, `json.edit: `.
//!
//! # Responsibilities
//!
//! - [`JsonEditError`] has one variant per refusal: the arguments, the document, the key path and
//!   the Lua value that is to be written.
//!
//! # Non-responsibilities
//!
//! Which refusal wins when several apply belongs to the code that edits the document.

use crate::types::op::LuaType;
use crate::types::report::{ParserMessage, Quoted};

/// A `json.edit` call refused.
///
/// # Examples
///
/// ```
/// use agentstacks::types::json::JsonEditError;
///
/// assert_eq!(
///     JsonEditError::PathEmpty.to_string(),
///     "json.edit: path: empty",
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JsonEditError {
    /// The `text` argument is not a string.
    #[error("json.edit: text: expected string, got {got}")]
    TextType {
        /// The type the argument has.
        got: LuaType,
    },
    /// The `text` argument is not UTF-8.
    #[error("json.edit: text: not valid UTF-8")]
    TextNotUtf8,
    /// The `path` argument is not a table.
    #[error("json.edit: path: expected table, got {got}")]
    PathType {
        /// The type the argument has.
        got: LuaType,
    },
    /// The `path` argument is a table but not a list.
    #[error("json.edit: path: expected a list")]
    PathNotList,
    /// The `text` does not parse as JSON, in the parser's own words.
    #[error("json.edit: {0}")]
    Parse(ParserMessage),
    /// The document's top level is not an object.
    #[error("json.edit: top level must be an object")]
    TopLevel,
    /// The `path` list is empty.
    #[error("json.edit: path: empty")]
    PathEmpty,
    /// An entry of the `path` list is not a string.
    #[error("json.edit: path[{index}]: expected string, got {got}")]
    PathItemType {
        /// The 1-based position of the entry, as Lua counts.
        index: usize,
        /// The type the entry has.
        got: LuaType,
    },
    /// A key on the way to the edit holds something other than an object.
    #[error("json.edit: {key} is not an object")]
    NotAnObject {
        /// The key, as written.
        key: Quoted,
    },
    /// The value is a table that mixes list and keyed entries.
    #[error("json.edit: value: table mixes list and keyed entries")]
    MixedTable,
    /// The value holds a NaN.
    #[error("json.edit: value: NaN cannot be JSON")]
    Nan,
    /// The value holds a positive infinity.
    #[error("json.edit: value: inf cannot be JSON")]
    Infinite,
    /// The value holds a negative infinity.
    #[error("json.edit: value: -inf cannot be JSON")]
    NegInfinite,
    /// The value holds a string that is not UTF-8.
    #[error("json.edit: value: string is not valid UTF-8")]
    StringNotUtf8,
    /// The value holds a table key that is not UTF-8.
    #[error("json.edit: value: key is not valid UTF-8")]
    KeyNotUtf8,
    /// The value is a table that contains itself.
    #[error("json.edit: value: table contains itself")]
    Cyclic,
    /// The value holds something that cannot be JSON: a function, userdata or a thread.
    #[error("json.edit: value: {got} cannot be JSON")]
    NotJson {
        /// The type the value has.
        got: LuaType,
    },
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::report::{Diagnostic, Location};

    fn shown(error: JsonEditError) -> String {
        let location = Location::at_line("claude/adapter.lua", NonZeroUsize::MIN.saturating_add(4));
        Diagnostic::new(location, error).to_string()
    }

    #[test]
    fn json_edit_error_text_type() {
        let error = JsonEditError::TextType { got: LuaType::Nil };
        assert_eq!(
            shown(error),
            "claude/adapter.lua:5: json.edit: text: expected string, got nil"
        );
    }

    #[test]
    fn json_edit_error_text_not_utf8() {
        assert_eq!(
            shown(JsonEditError::TextNotUtf8),
            "claude/adapter.lua:5: json.edit: text: not valid UTF-8"
        );
    }

    #[test]
    fn json_edit_error_path_type() {
        let error = JsonEditError::PathType {
            got: LuaType::String,
        };
        assert_eq!(
            shown(error),
            "claude/adapter.lua:5: json.edit: path: expected table, got string"
        );
    }

    #[test]
    fn json_edit_error_path_not_a_list() {
        assert_eq!(
            shown(JsonEditError::PathNotList),
            "claude/adapter.lua:5: json.edit: path: expected a list"
        );
    }

    #[test]
    fn json_edit_error_parse_keeps_serde_text() {
        let error = JsonEditError::Parse(ParserMessage::new("trailing comma at line 1 column 8"));
        assert_eq!(
            shown(error),
            "claude/adapter.lua:5: json.edit: trailing comma at line 1 column 8"
        );
    }

    #[test]
    fn json_edit_error_top_level() {
        assert_eq!(
            shown(JsonEditError::TopLevel),
            "claude/adapter.lua:5: json.edit: top level must be an object"
        );
    }

    #[test]
    fn json_edit_error_path_empty() {
        assert_eq!(
            shown(JsonEditError::PathEmpty),
            "claude/adapter.lua:5: json.edit: path: empty"
        );
    }

    #[test]
    fn json_edit_error_path_item_type() {
        let error = JsonEditError::PathItemType {
            index: 2,
            got: LuaType::Number,
        };
        assert_eq!(
            shown(error),
            "claude/adapter.lua:5: json.edit: path[2]: expected string, got number"
        );
    }

    #[test]
    fn json_edit_error_not_an_object() {
        let error = JsonEditError::NotAnObject {
            key: Quoted::new("hooks"),
        };
        assert_eq!(
            shown(error),
            "claude/adapter.lua:5: json.edit: \"hooks\" is not an object"
        );
    }

    #[test]
    fn json_edit_error_mixed_table() {
        assert_eq!(
            shown(JsonEditError::MixedTable),
            "claude/adapter.lua:5: json.edit: value: table mixes list and keyed entries"
        );
    }

    #[test]
    fn json_edit_error_nan() {
        assert_eq!(
            shown(JsonEditError::Nan),
            "claude/adapter.lua:5: json.edit: value: NaN cannot be JSON"
        );
    }

    #[test]
    fn json_edit_error_inf() {
        assert_eq!(
            shown(JsonEditError::Infinite),
            "claude/adapter.lua:5: json.edit: value: inf cannot be JSON"
        );
    }

    #[test]
    fn json_edit_error_negative_inf() {
        assert_eq!(
            shown(JsonEditError::NegInfinite),
            "claude/adapter.lua:5: json.edit: value: -inf cannot be JSON"
        );
    }

    #[test]
    fn json_edit_error_string_not_utf8() {
        assert_eq!(
            shown(JsonEditError::StringNotUtf8),
            "claude/adapter.lua:5: json.edit: value: string is not valid UTF-8"
        );
    }

    #[test]
    fn json_edit_error_key_not_utf8() {
        assert_eq!(
            shown(JsonEditError::KeyNotUtf8),
            "claude/adapter.lua:5: json.edit: value: key is not valid UTF-8"
        );
    }

    #[test]
    fn json_edit_error_table_contains_itself() {
        assert_eq!(
            shown(JsonEditError::Cyclic),
            "claude/adapter.lua:5: json.edit: value: table contains itself"
        );
    }

    #[test]
    fn json_edit_error_value_not_json() {
        for got in [LuaType::Function, LuaType::Userdata, LuaType::Thread] {
            assert_eq!(
                shown(JsonEditError::NotJson { got }),
                format!("claude/adapter.lua:5: json.edit: value: {got} cannot be JSON")
            );
        }
    }
}
