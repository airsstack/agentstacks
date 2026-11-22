//! Editing the text of a JSON document at one key path.
//!
//! # Responsibilities
//!
//! - [`edit()`]: parses the text, walks the key path, sets or removes one key, and prints the
//!   document back with the layout the text had.
//! - The order of the refusals: the first that applies is the one returned.
//!
//! # Non-responsibilities
//!
//! Reading the call's arguments and converting the Lua value to JSON belong to the host. `edit`
//! takes the converted value as a closure and calls it only after every refusal that does not
//! depend on the value has passed.

use serde_json::{Map, Value};

use super::JsonEditError;
use super::elem::PathElem;
use super::indent::Indent;
use super::print::{Ending, print};
use crate::types::report::{ParserMessage, Quoted};

/// Sets the value at `path` in the JSON `text`, or removes the key when `value` gives `None`.
///
/// Text that is empty or only whitespace is the document `{}`. The other rules:
///
/// - The refusals are checked in this order, and the first that applies is returned: the text
///   does not parse, its top level is not an object, the path is empty, a path element is not a
///   string, a key on the way holds something other than an object. Then `value` is called, once,
///   and its refusal is returned unchanged.
/// - Setting inserts every missing key on the way as an object. An existing key keeps its
///   position and takes the new value, and a new key goes last.
/// - Removing a key that is missing, or whose parent is missing, returns `text` unchanged. Every
///   other removal keeps the order of the remaining keys, and prunes no parent.
/// - The document is printed with the indent of the first indented line of `text` (two spaces
///   when there is none), with `\n` line endings, and with a trailing newline exactly when `text`
///   had one. Numbers come out in their normal form, and the last of a repeated key wins.
///
/// # Errors
///
/// Returns the [`JsonEditError`] of the first refusal above.
///
/// # Examples
///
/// ```
/// use agentstacks::types::json::{PathElem, edit};
/// use serde_json::json;
///
/// let path = [PathElem::Key("hooks".to_owned())];
/// let edited = edit("{\n    \"a\": 1\n}\n", &path, || Ok(Some(json!([]))))?;
/// assert_eq!(edited, "{\n    \"a\": 1,\n    \"hooks\": []\n}\n");
/// # Ok::<(), agentstacks::types::json::JsonEditError>(())
/// ```
pub fn edit<F>(text: &str, path: &[PathElem], value: F) -> Result<String, JsonEditError>
where
    F: FnOnce() -> Result<Option<Value>, JsonEditError>,
{
    let mut document = parse(text)?;
    let keys = keys(path)?;
    let Some((last, parents)) = keys.split_last() else {
        return Err(JsonEditError::PathEmpty);
    };
    check_parents(&document, parents)?;
    match value()? {
        Some(value) => set(&mut document, parents, last, value)?,
        None => {
            if !remove(&mut document, parents, last) {
                return Ok(text.to_owned());
            }
        }
    }
    print(
        &Value::Object(document),
        &Indent::detect(text),
        Ending::of(text),
    )
}

/// The document of `text`, which must be an object. Empty or whitespace-only text is `{}`.
fn parse(text: &str) -> Result<Map<String, Value>, JsonEditError> {
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(document)) => Ok(document),
        Ok(_) => Err(JsonEditError::TopLevel),
        Err(error) => Err(JsonEditError::Parse(ParserMessage::new(&error.to_string()))),
    }
}

/// The keys of `path`, or the refusal of its first element that is not a string.
fn keys(path: &[PathElem]) -> Result<Vec<&str>, JsonEditError> {
    path.iter()
        .enumerate()
        .map(|(position, elem)| match elem {
            PathElem::Key(key) => Ok(key.as_str()),
            PathElem::NotString(got) => Err(JsonEditError::PathItemType {
                index: position + 1,
                got: *got,
            }),
        })
        .collect()
}

/// Walks `parents` without changing anything, stopping at the first missing key. A key that
/// holds something other than an object is refused.
fn check_parents(document: &Map<String, Value>, parents: &[&str]) -> Result<(), JsonEditError> {
    let mut current = document;
    for key in parents {
        match current.get(*key) {
            None => return Ok(()),
            Some(Value::Object(next)) => current = next,
            Some(_) => return Err(not_an_object(key)),
        }
    }
    Ok(())
}

/// Inserts `value` at `last` under `parents`, creating each missing parent as an object.
fn set(
    document: &mut Map<String, Value>,
    parents: &[&str],
    last: &str,
    value: Value,
) -> Result<(), JsonEditError> {
    let mut current = document;
    for key in parents {
        let next = current
            .entry((*key).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        current = next.as_object_mut().ok_or_else(|| not_an_object(key))?;
    }
    current.insert(last.to_owned(), value);
    Ok(())
}

/// Removes `last` under `parents`, keeping the order of the other keys. Gives `false` when a
/// parent or the key is missing, which leaves the document as it was.
fn remove(document: &mut Map<String, Value>, parents: &[&str], last: &str) -> bool {
    let mut current = document;
    for key in parents {
        match current.get_mut(*key) {
            Some(Value::Object(next)) => current = next,
            _ => return false,
        }
    }
    current.shift_remove(last).is_some()
}

/// The refusal for a key that holds something other than an object.
fn not_an_object(key: &str) -> JsonEditError {
    JsonEditError::NotAnObject {
        key: Quoted::new(key),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use serde_json::json;

    use super::*;
    use crate::types::op::LuaType;

    fn path(keys: &[&str]) -> Vec<PathElem> {
        keys.iter()
            .map(|key| PathElem::Key((*key).to_owned()))
            .collect()
    }

    fn set_to(text: &str, keys: &[&str], value: Value) -> Result<String, JsonEditError> {
        edit(text, &path(keys), || Ok(Some(value)))
    }

    fn remove_at(text: &str, keys: &[&str]) -> Result<String, JsonEditError> {
        edit(text, &path(keys), || Ok(None))
    }

    /// The layout of a probe document with `unit` as the indent and the given number spellings,
    /// ending in a newline.
    fn layout(unit: &str, alpha: &str, d: &str) -> String {
        let (one, two, three) = (unit.to_owned(), unit.repeat(2), unit.repeat(3));
        format!(
            "{{\n{one}\"permissions\": {{\n{two}\"allow\": [\n{three}\"Bash(x:*)\"\n{two}]\n{one}}},\n{one}\"zeta\": 1,\n{one}\"alpha\": {alpha},\n{one}\"d\": {d},\n{one}\"hooks\": {{}}\n}}\n"
        )
    }

    fn source(unit: &str) -> String {
        layout(unit, "12345678901234567890123", "0.10")
    }

    fn printed(unit: &str) -> String {
        layout(unit, "1.2345678901234568e+22", "0.1")
    }

    fn parse_error(message: &str) -> JsonEditError {
        JsonEditError::Parse(ParserMessage::new(message))
    }

    #[test]
    fn edit_treats_empty_text_as_an_empty_object() {
        assert_eq!(
            set_to("", &["a"], json!(1)),
            Ok("{\n  \"a\": 1\n}".to_owned())
        );
    }

    #[test]
    fn edit_treats_whitespace_only_text_as_an_empty_object() {
        assert_eq!(
            set_to("  ", &["a"], json!(1)),
            Ok("{\n  \"a\": 1\n}".to_owned())
        );
    }

    #[test]
    fn edit_takes_whitespace_only_formatting_from_the_original_text() {
        assert_eq!(
            set_to("\n", &["a"], json!(1)),
            Ok("{\n  \"a\": 1\n}\n".to_owned())
        );
        assert_eq!(
            set_to("\n\t\n", &["a"], json!(1)),
            Ok("{\n\t\"a\": 1\n}\n".to_owned())
        );
    }

    #[test]
    fn edit_first_failing_row_wins() {
        let not_string = [PathElem::NotString(LuaType::Number)];
        assert!(matches!(
            set_to("{,", &[], json!(1)),
            Err(JsonEditError::Parse(_))
        ));
        assert_eq!(set_to("[1]", &[], json!(1)), Err(JsonEditError::TopLevel));
        assert!(matches!(
            edit("{,", &not_string, || Ok(Some(json!(1)))),
            Err(JsonEditError::Parse(_))
        ));
    }

    #[test]
    fn edit_refuses_a_path_element_that_is_not_a_string() {
        let elems = [
            PathElem::Key("a".to_owned()),
            PathElem::NotString(LuaType::Number),
        ];
        let refused = edit("{}", &elems, || Ok(Some(json!(1))));
        assert_eq!(
            refused.map_err(|error| error.to_string()),
            Err("json.edit: path[2]: expected string, got number".to_owned())
        );
    }

    #[test]
    fn edit_calls_value_last() {
        let calls = Cell::new(0);
        let call = |text: &str, elems: &[PathElem]| {
            edit(text, elems, || {
                calls.set(calls.get() + 1);
                Ok(Some(json!(1)))
            })
        };
        assert!(call("{,", &path(&["a"])).is_err());
        assert!(call("[1]", &path(&["a"])).is_err());
        assert!(call("{}", &[]).is_err());
        assert!(call("{}", &[PathElem::NotString(LuaType::Table)]).is_err());
        assert!(call("{\"a\": []}", &path(&["a", "b"])).is_err());
        assert_eq!(calls.get(), 0);
        assert!(call("{}", &path(&["a"])).is_ok());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn edit_returns_the_value_refusal_unchanged() {
        let refused = edit("{}", &path(&["a"]), || Err(JsonEditError::MixedTable));
        assert_eq!(refused, Err(JsonEditError::MixedTable));
    }

    #[test]
    fn edit_refuses_text_that_is_not_json() {
        assert_eq!(
            set_to("{\"a\":1,}", &["b"], json!(1)),
            Err(parse_error("trailing comma at line 1 column 8"))
        );
    }

    #[test]
    fn edit_refuses_a_top_level_that_is_not_an_object() {
        for text in ["[1]", "\"x\"", "null", "1"] {
            assert_eq!(set_to(text, &["a"], json!(1)), Err(JsonEditError::TopLevel));
        }
    }

    #[test]
    fn edit_refuses_an_empty_path() {
        assert_eq!(set_to("{}", &[], json!(1)), Err(JsonEditError::PathEmpty));
    }

    #[test]
    fn edit_creates_missing_parents_as_objects() {
        assert_eq!(
            set_to("{}", &["a", "b"], json!(1)),
            Ok("{\n  \"a\": {\n    \"b\": 1\n  }\n}".to_owned())
        );
    }

    #[test]
    fn edit_refuses_a_parent_that_is_not_an_object() {
        let hooks = set_to("{\"hooks\": []}", &["hooks", "PreToolUse"], json!([]));
        assert_eq!(hooks, Err(not_an_object("hooks")));
        let deep = set_to("{\"a\": {\"b\": 1}}", &["a", "b", "c"], json!(1));
        assert_eq!(deep, Err(not_an_object("b")));
    }

    #[test]
    fn edit_replaces_the_value_at_the_last_key() {
        assert_eq!(
            set_to("{\"hooks\": [1]}", &["hooks"], json!({"x": 1})),
            Ok("{\n  \"hooks\": {\n    \"x\": 1\n  }\n}".to_owned())
        );
    }

    #[test]
    fn edit_returns_the_text_unchanged_when_removing_a_missing_key() {
        assert_eq!(
            remove_at("{\"a\": 0.10}", &["b"]),
            Ok("{\"a\": 0.10}".to_owned())
        );
    }

    #[test]
    fn edit_returns_the_text_unchanged_when_removing_under_a_missing_parent() {
        assert_eq!(
            remove_at("{\"a\": 0.10}", &["x", "y"]),
            Ok("{\"a\": 0.10}".to_owned())
        );
    }

    #[test]
    fn edit_refuses_removing_through_a_parent_that_is_not_an_object() {
        let refused = remove_at("{\"hooks\": []}", &["hooks", "x"]);
        assert_eq!(refused, Err(not_an_object("hooks")));
        assert_eq!(
            refused.map_err(|error| error.to_string()),
            Err("json.edit: \"hooks\" is not an object".to_owned())
        );
    }

    #[test]
    fn edit_removes_an_existing_key_and_keeps_the_others_in_order() {
        assert_eq!(
            remove_at("{\"a\":1,\"b\":2,\"c\":3,\"d\":4}", &["b"]),
            Ok("{\n  \"a\": 1,\n  \"c\": 3,\n  \"d\": 4\n}".to_owned())
        );
    }

    #[test]
    fn edit_keeps_existing_keys_in_place_and_appends_new_ones() -> Result<(), JsonEditError> {
        let once = set_to(&source("    "), &["zeta"], json!(2))?;
        let twice = set_to(&once, &["new"], json!([]))?;
        let document: Value =
            serde_json::from_str(&twice).map_err(|error| parse_error(&error.to_string()))?;
        let order: Vec<&str> = document
            .as_object()
            .map(|map| map.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(order, ["permissions", "zeta", "alpha", "d", "hooks", "new"]);
        Ok(())
    }

    #[test]
    fn edit_keeps_the_indent_of_the_input() {
        for unit in ["    ", "  ", "\t"] {
            assert_eq!(
                set_to(&source(unit), &["zeta"], json!(1)),
                Ok(printed(unit))
            );
        }
    }

    #[test]
    fn edit_falls_back_to_two_spaces() {
        assert_eq!(
            set_to("{\"a\":1}", &["b"], json!(2)),
            Ok("{\n  \"a\": 1,\n  \"b\": 2\n}".to_owned())
        );
    }

    #[test]
    fn edit_turns_crlf_into_lf() {
        let crlf = source("    ").replace('\n', "\r\n");
        let edited = set_to(&crlf, &["zeta"], json!(1));
        assert_eq!(edited, Ok(printed("    ")));
        assert!(edited.is_ok_and(|text| !text.contains('\r')));
    }

    #[test]
    fn edit_keeps_a_trailing_newline() {
        let edited = set_to("{\n  \"a\": 1\n}\n", &["b"], json!(2));
        assert_eq!(edited, Ok("{\n  \"a\": 1,\n  \"b\": 2\n}\n".to_owned()));
    }

    #[test]
    fn edit_adds_no_trailing_newline_when_the_input_had_none() {
        assert!(set_to("{}", &["a"], json!(1)).is_ok_and(|text| text.ends_with('}')));
    }

    #[test]
    fn edit_reprints_numbers_in_normal_form() {
        let text = "{\"alpha\": 12345678901234567890123, \"d\": 0.10}";
        assert_eq!(
            set_to(text, &["x"], json!(1)),
            Ok([
                "{",
                "  \"alpha\": 1.2345678901234568e+22,",
                "  \"d\": 0.1,",
                "  \"x\": 1",
                "}",
            ]
            .join("\n"))
        );
    }

    #[test]
    fn edit_keeps_the_last_of_duplicate_keys() {
        assert_eq!(
            set_to("{\"a\":1,\"a\":2}", &["b"], json!(true)),
            Ok("{\n  \"a\": 2,\n  \"b\": true\n}".to_owned())
        );
    }

    #[test]
    fn edit_matches_the_checked_example() {
        let old = "{\n    \"permissions\": {\"allow\": [\"Bash(x:*)\"]},\n    \"hooks\": {}\n}\n";
        let expected = "{\n    \"permissions\": {\n        \"allow\": [\n            \"Bash(x:*)\"\n        ]\n    },\n    \"hooks\": {\n        \"PreToolUse\": [\n            {\n                \"matcher\": \"Read\"\n            }\n        ]\n    }\n}\n";
        assert_eq!(
            set_to(old, &["hooks", "PreToolUse"], json!([{"matcher": "Read"}])),
            Ok(expected.to_owned())
        );
    }
}
