//! The Lua function `agentstacks.json.edit(text, path, value)`.
//!
//! # Responsibilities
//!
//! - [`edit`] builds the function. It reads its three arguments as Lua values and hands the text,
//!   the key path and the value to [`types::json::edit`](crate::types::json::edit), which owns the
//!   rules and the order of the refusals.
//! - It refuses only what [`types::json::edit`](crate::types::json::edit) cannot see: a `text` or
//!   `path` of the wrong type, `text` that is not UTF-8, and a `path` table that is not a list.
//!   A path that is not a list is refused in the slot where an empty path is, so it never
//!   pre-empts an earlier refusal.
//! - The value is converted to JSON only when the rules reach it, and `nil` removes the key.
//! - A refusal is an ordinary Lua error located at the nearest Lua frame: `pcall` catches it,
//!   nothing is recorded, and uncaught it fails the run with its located message.
//!
//! # Non-responsibilities
//!
//! Parsing the text, walking the path and printing the result belong to `types::json`.

use airsl::mlua::{self, Function, Lua, Value};

use crate::host::caller::locate;
use crate::host::convert::{lua_to_json, lua_type, table_list};
use crate::host::failure::Raised;
use crate::host::state::SharedState;
use crate::types::json::{JsonEditError, PathElem, edit as edit_text};
use crate::types::op::LuaType;
use crate::types::report::Diagnostic;

/// The Lua function `agentstacks.json.edit`.
pub(super) fn edit(lua: &Lua, state: &SharedState) -> mlua::Result<Function> {
    let state = state.clone();
    lua.create_function(move |lua, (text, path, value): (Value, Value, Value)| {
        edited(&text, &path, &value).map_err(|error| {
            let tool = state.with(|run| run.tool().clone());
            Raised::ordinary(Diagnostic::new(locate(lua, &tool), error)).into_lua()
        })
    })
}

/// The edited text, or the first refusal.
fn edited(text: &Value, path: &Value, value: &Value) -> Result<String, JsonEditError> {
    let Value::String(text) = text else {
        return Err(JsonEditError::TextType {
            got: lua_type(text),
        });
    };
    let bytes = text.as_bytes();
    let text = std::str::from_utf8(&bytes).map_err(|_| JsonEditError::TextNotUtf8)?;
    let Value::Table(path) = path else {
        return Err(JsonEditError::PathType {
            got: lua_type(path),
        });
    };
    let list = table_list(path);
    let elements: Vec<PathElem> = list.iter().flatten().map(path_element).collect();
    let result = edit_text(text, &elements, || match value {
        Value::Nil => Ok(None),
        other => lua_to_json(other).map(Some),
    });
    match result {
        Err(JsonEditError::PathEmpty) if list.is_none() => Err(JsonEditError::PathNotList),
        other => other,
    }
}

/// One entry of the path table as a path element. An entry that is not a string, or is a string
/// that is not UTF-8, is kept as a non-string so the rules refuse it at its position.
fn path_element(entry: &Value) -> PathElem {
    if let Value::String(text) = entry
        && let Ok(key) = std::str::from_utf8(&text.as_bytes())
    {
        return PathElem::Key(key.to_owned());
    }
    PathElem::NotString(match entry {
        Value::String(_) => LuaType::String,
        other => lua_type(other),
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::*;
    use crate::host::events::Event;
    use crate::host::testing::Rig;

    type TestResult = Result<(), Box<dyn Error>>;

    /// What `agentstacks.json.edit(<arguments>)` on line 1 of the entry file gives.
    fn call(arguments: &str) -> Result<String, String> {
        let rig = Rig::new().map_err(|error| error.to_string())?;
        rig.text(&format!("return agentstacks.json.edit({arguments})"))
    }

    fn refusal(arguments: &str) -> String {
        call(arguments).err().unwrap_or_else(|| "edited".to_owned())
    }

    #[test]
    fn checked_example_is_byte_for_byte() -> TestResult {
        let edited = call(
            r#"'{\n    "permissions": {"allow": ["Bash(x:*)"]},\n    "hooks": {}\n}\n', {"hooks", "PreToolUse"}, { { matcher = "Read" } }"#,
        )?;
        let expected = "{\n    \"permissions\": {\n        \"allow\": [\n            \"Bash(x:*)\"\n        ]\n    },\n    \"hooks\": {\n        \"PreToolUse\": [\n            {\n                \"matcher\": \"Read\"\n            }\n        ]\n    }\n}\n";
        assert_eq!(edited, expected);
        Ok(())
    }

    #[test]
    fn text_must_be_a_string() {
        assert_eq!(
            refusal(r#"nil, {"a"}, 1"#),
            "claude/adapter.lua:1: json.edit: text: expected string, got nil"
        );
    }

    #[test]
    fn text_must_be_utf8() {
        assert_eq!(
            refusal(r#""\255", {"a"}, 1"#),
            "claude/adapter.lua:1: json.edit: text: not valid UTF-8"
        );
    }

    #[test]
    fn path_must_be_a_table() {
        assert_eq!(
            refusal(r#""{}", "a", 1"#),
            "claude/adapter.lua:1: json.edit: path: expected table, got string"
        );
    }

    #[test]
    fn path_must_be_a_list() {
        assert_eq!(
            refusal(r#""{}", {a = "x"}, 1"#),
            "claude/adapter.lua:1: json.edit: path: expected a list"
        );
    }

    #[test]
    fn path_must_not_be_empty() {
        assert_eq!(
            refusal(r#""{}", {}, 1"#),
            "claude/adapter.lua:1: json.edit: path: empty"
        );
    }

    #[test]
    fn path_elements_must_be_strings() {
        assert_eq!(
            refusal(r#""{}", {"a", 2}, 1"#),
            "claude/adapter.lua:1: json.edit: path[2]: expected string, got number"
        );
    }

    #[test]
    fn a_path_element_that_is_not_utf8_is_refused_at_its_position() {
        assert_eq!(
            refusal(r#""{}", {"a", "\255"}, 1"#),
            "claude/adapter.lua:1: json.edit: path[2]: expected string, got string"
        );
    }

    #[test]
    fn argument_row_wins() {
        assert_eq!(
            refusal("nil, {}, 1"),
            "claude/adapter.lua:1: json.edit: text: expected string, got nil"
        );
    }

    #[test]
    fn parse_refusal_precedes_path_rows() {
        let parse = edit_text("{,", &[], || Ok(None))
            .err()
            .map(|e| e.to_string());
        assert_eq!(
            refusal(r#""{,", {}, 1"#),
            format!("claude/adapter.lua:1: {}", parse.unwrap_or_default())
        );
        assert!(refusal(r#""{,", {}, 1"#).contains("json.edit: "));
        assert!(!refusal(r#""{,", {}, 1"#).contains("path: empty"));
        assert!(!refusal(r#""{,", {a = 1}, 1"#).contains("expected a list"));
    }

    #[test]
    fn value_is_not_converted_before_earlier_rows() {
        let parse = edit_text("{,", &[], || Ok(None))
            .err()
            .map(|e| e.to_string());
        assert_eq!(
            refusal(r#""{,", {"a"}, {1, a = 2}"#),
            format!("claude/adapter.lua:1: {}", parse.unwrap_or_default())
        );
    }

    #[test]
    fn value_refusal_is_located() {
        assert_eq!(
            refusal(r#""{}", {"a"}, {1, a = 2}"#),
            "claude/adapter.lua:1: json.edit: value: table mixes list and keyed entries"
        );
    }

    #[test]
    fn edit_refusal_passes_through_located() {
        assert_eq!(
            refusal(r#"'{"hooks": 1}', {"hooks", "x"}, 1"#),
            "claude/adapter.lua:1: json.edit: \"hooks\" is not an object"
        );
        assert_eq!(
            refusal(r#"'{"hooks": 1}', {"hooks", "x"}, nil"#),
            "claude/adapter.lua:1: json.edit: \"hooks\" is not an object"
        );
    }

    #[test]
    fn nil_value_removes_the_key() -> TestResult {
        assert_eq!(call(r#"'{"a":1,"b":2}', {"a"}, nil"#)?, "{\n  \"b\": 2\n}");
        assert_eq!(call(r#"'{"a":1}', {"z"}, nil"#)?, r#"{"a":1}"#);
        Ok(())
    }

    #[test]
    fn refusal_is_catchable_with_pcall() -> TestResult {
        let rig = Rig::new()?;
        rig.run("local ok = pcall(agentstacks.json.edit, nil, {'a'}, 1)\nassert(ok == false)")?;
        assert_eq!(rig.recorded(), 0);
        Ok(())
    }

    #[test]
    fn a_caught_refusal_reads_as_the_located_message() -> TestResult {
        let rig = Rig::new()?;
        let seen = rig.text(
            "local _, message = pcall(agentstacks.json.edit, nil, {'a'}, 1)\nreturn tostring(message):match('^[^\\n]*')",
        )?;
        assert_eq!(
            seen,
            "claude/adapter.lua:1: json.edit: text: expected string, got nil"
        );
        Ok(())
    }

    #[test]
    fn uncaught_refusal_shows_its_located_message() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('plan', function()\n  agentstacks.json.edit(nil, {'a'}, 1)\nend)")?;
        let refused = rig.dispatch(Event::Plan, &json!({})).err();
        assert_eq!(
            refused.as_deref(),
            Some("claude/adapter.lua:2: json.edit: text: expected string, got nil")
        );
        Ok(())
    }

    #[test]
    fn a_table_nested_ten_thousand_deep_is_refused() {
        let source = "local t = {} local c = t for i = 1, 10000 do local d = {} c[1] = d c = d end\n\
                      return agentstacks.json.edit('{}', {'a'}, t)";
        let refused = Rig::new()
            .map_err(|error| error.to_string())
            .and_then(|rig| rig.text(source))
            .err();
        assert_eq!(
            refused.as_deref(),
            Some("claude/adapter.lua:2: json.edit: value: nested more than 128 levels deep")
        );
    }
}
