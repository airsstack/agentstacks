//! Lua values read as JSON, and JSON values named as Lua names them.
//!
//! mlua's own serde conversion drops data, `{1, a = 2}` becomes `[1]`, so the conversion is
//! written by hand over raw table access, and no metamethod runs.
//!
//! # Responsibilities
//!
//! - [`lua_to_json`] converts a Lua value:
//!   - A table whose keys are exactly `1..n` is an array in index order, and a table of only
//!     string keys is an object with its keys sorted by bytes.
//!   - An empty table is `{}`, and there is no way to write `[]`.
//!   - Any other table, a table that contains itself, a function, a userdata, a thread, a NaN or
//!     an infinity, and a string or key that is not UTF-8 are refused with their
//!     [`JsonEditError`]. The same table reached twice by different keys is not a cycle.
//! - [`lua_type`] is the type word of a Lua value, and [`json_word`] is the word the Lua author
//!   wrote for a JSON value that came back from a handler: an object is a `table`, an array a
//!   `list`.
//! - [`table_list`] reads a table as a list of its values when its keys are exactly `1..n`, and
//!   [`entries`] reads every key and value of a table.
//!
//! # Non-responsibilities
//!
//! Which call reads a value, and how a refusal is located, belong to the function that calls.

use std::ffi::c_void;

use airsl::mlua::{Table, Value};
use serde_json::{Map, Number, Value as Json};

use crate::types::json::{JsonEditError, MAX_DEPTH};
use crate::types::op::LuaType;

/// The type of `value`, as a refusal names it.
pub(super) const fn lua_type(value: &Value) -> LuaType {
    match value {
        Value::Nil => LuaType::Nil,
        Value::Boolean(_) => LuaType::Boolean,
        Value::Integer(_) | Value::Number(_) => LuaType::Number,
        Value::String(_) => LuaType::String,
        Value::Table(_) => LuaType::Table,
        Value::Function(_) => LuaType::Function,
        Value::Thread(_) => LuaType::Thread,
        Value::LightUserData(_) | Value::UserData(_) | Value::Error(_) | Value::Other(..) => {
            LuaType::Userdata
        }
    }
}

/// The word a Lua author wrote for the JSON `value` a handler returned.
pub(super) const fn json_word(value: &Json) -> LuaType {
    match value {
        Json::Null => LuaType::Nil,
        Json::Bool(_) => LuaType::Boolean,
        Json::Number(_) => LuaType::Number,
        Json::String(_) => LuaType::String,
        Json::Array(_) => LuaType::List,
        Json::Object(_) => LuaType::Table,
    }
}

/// The values of `table` in index order when its keys are exactly the integers `1..n`, and
/// `None` for any other table. An empty table is the empty list.
pub(super) fn table_list(table: &Table) -> Option<Vec<Value>> {
    list_of(&entries(table))
}

/// `value` as JSON.
///
/// # Errors
///
/// The [`JsonEditError`] of the first thing in `value` that cannot be JSON.
pub(super) fn lua_to_json(value: &Value) -> Result<Json, JsonEditError> {
    convert(value, &mut Vec::new())
}

/// Converts `value`, where `path` holds the tables from the top down to the one being converted.
fn convert(value: &Value, path: &mut Vec<*const c_void>) -> Result<Json, JsonEditError> {
    match value {
        Value::Nil => Ok(Json::Null),
        Value::Boolean(flag) => Ok(Json::Bool(*flag)),
        Value::Integer(number) => Ok(Json::from(*number)),
        Value::Number(number) => float(*number),
        Value::String(text) => std::str::from_utf8(&text.as_bytes())
            .map(|text| Json::String(text.to_owned()))
            .map_err(|_| JsonEditError::StringNotUtf8),
        Value::Table(table) => convert_table(table, path),
        other => Err(JsonEditError::NotJson {
            got: lua_type(other),
        }),
    }
}

/// A float as a JSON number, or the refusal for a NaN or an infinity.
fn float(number: f64) -> Result<Json, JsonEditError> {
    if number.is_nan() {
        return Err(JsonEditError::Nan);
    }
    if number == f64::INFINITY {
        return Err(JsonEditError::Infinite);
    }
    if number == f64::NEG_INFINITY {
        return Err(JsonEditError::NegInfinite);
    }
    Number::from_f64(number)
        .map(Json::Number)
        .ok_or(JsonEditError::Nan)
}

/// A table as an array or an object. `path` is extended by `table` while its entries convert.
fn convert_table(table: &Table, path: &mut Vec<*const c_void>) -> Result<Json, JsonEditError> {
    let here = table.to_pointer();
    if path.contains(&here) {
        return Err(JsonEditError::Cyclic);
    }
    if path.len() >= MAX_DEPTH {
        return Err(JsonEditError::TooDeep);
    }
    let entries = entries(table);
    path.push(here);
    let converted = match list_of(&entries) {
        Some(items) if !items.is_empty() => items
            .iter()
            .map(|item| convert(item, path))
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => object(entries, path),
    };
    path.pop();
    converted
}

/// An object from `entries`, whose keys must all be strings. Its keys are sorted by bytes.
fn object(
    entries: Vec<(Value, Value)>,
    path: &mut Vec<*const c_void>,
) -> Result<Json, JsonEditError> {
    let mut keyed = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        let Value::String(key) = key else {
            return Err(JsonEditError::MixedTable);
        };
        keyed.push((key, value));
    }
    let mut named = Vec::with_capacity(keyed.len());
    for (key, value) in &keyed {
        let name = std::str::from_utf8(&key.as_bytes())
            .map(str::to_owned)
            .map_err(|_| JsonEditError::KeyNotUtf8)?;
        named.push((name, value));
    }
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let mut object = Map::new();
    for (name, value) in named {
        object.insert(name, convert(value, path)?);
    }
    Ok(Json::Object(object))
}

/// Every key and value of `table`, read without running a metamethod.
pub(super) fn entries(table: &Table) -> Vec<(Value, Value)> {
    table
        .pairs::<Value, Value>()
        .filter_map(Result::ok)
        .collect()
}

/// The values of `entries` in index order when their keys are exactly the integers `1..n`.
fn list_of(entries: &[(Value, Value)]) -> Option<Vec<Value>> {
    let mut slots: Vec<Option<Value>> = vec![None; entries.len()];
    for (key, value) in entries {
        let Value::Integer(index) = key else {
            return None;
        };
        let slot = usize::try_from(*index)
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| slots.get_mut(index))?;
        *slot = Some(value.clone());
    }
    slots.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use airsl::mlua::Lua;

    use super::*;

    /// `source` evaluated in a fresh Lua state and converted, as compact JSON text or the
    /// refusal with the `json.edit: ` prefix the caller gives it.
    fn converted(source: &str) -> Result<String, String> {
        let lua = Lua::new();
        let value: Value = lua.load(source).eval().map_err(|error| error.to_string())?;
        lua_to_json(&value)
            .map(|json| json.to_string())
            .map_err(|error| error.to_string())
    }

    fn refused(source: &str) -> String {
        converted(source)
            .err()
            .unwrap_or_else(|| "converted".to_owned())
    }

    #[test]
    fn sequence_becomes_array() {
        assert_eq!(
            converted("return {1, 'a', true}").as_deref(),
            Ok(r#"[1,"a",true]"#)
        );
    }

    #[test]
    fn string_keys_become_sorted_object() {
        assert_eq!(
            converted("return {zeta = 1, alpha = 2}").as_deref(),
            Ok(r#"{"alpha":2,"zeta":1}"#)
        );
    }

    #[test]
    fn empty_table_becomes_empty_object() {
        assert_eq!(converted("return {}").as_deref(), Ok("{}"));
    }

    #[test]
    fn mixed_table_is_refused() {
        assert_eq!(
            refused("return {1, a = 2}"),
            "json.edit: value: table mixes list and keyed entries"
        );
    }

    #[test]
    fn sparse_integer_keys_are_refused() {
        assert_eq!(
            refused("return {[1] = 1, [3] = 3}"),
            "json.edit: value: table mixes list and keyed entries"
        );
    }

    #[test]
    fn self_containing_table_is_refused() {
        assert_eq!(
            refused("local t = {} t.me = t return t"),
            "json.edit: value: table contains itself"
        );
    }

    #[test]
    fn a_cycle_through_a_list_is_refused() {
        assert_eq!(
            refused("local t = {} t[1] = {t} return t"),
            "json.edit: value: table contains itself"
        );
    }

    #[test]
    fn shared_subtable_is_not_a_cycle() {
        assert_eq!(
            converted("local s = {1} return {a = s, b = s}").as_deref(),
            Ok(r#"{"a":[1],"b":[1]}"#)
        );
    }

    #[test]
    fn integer_and_float_become_numbers() {
        assert_eq!(converted("return 3").as_deref(), Ok("3"));
        assert_eq!(converted("return 0.5").as_deref(), Ok("0.5"));
    }

    #[test]
    fn nan_is_refused() {
        assert_eq!(
            refused("return 0/0"),
            "json.edit: value: NaN cannot be JSON"
        );
    }

    #[test]
    fn inf_is_refused() {
        assert_eq!(
            refused("return math.huge"),
            "json.edit: value: inf cannot be JSON"
        );
    }

    #[test]
    fn negative_inf_is_refused() {
        assert_eq!(
            refused("return -math.huge"),
            "json.edit: value: -inf cannot be JSON"
        );
    }

    #[test]
    fn non_utf8_string_is_refused() {
        assert_eq!(
            refused(r#"return "\255""#),
            "json.edit: value: string is not valid UTF-8"
        );
    }

    #[test]
    fn non_utf8_key_is_refused() {
        assert_eq!(
            refused(r#"return {["\255"] = 1}"#),
            "json.edit: value: key is not valid UTF-8"
        );
    }

    #[test]
    fn function_is_refused() {
        assert_eq!(
            refused("return function() end"),
            "json.edit: value: function cannot be JSON"
        );
    }

    #[test]
    fn thread_and_userdata_are_refused() {
        assert_eq!(
            refused("return coroutine.create(function() end)"),
            "json.edit: value: thread cannot be JSON"
        );
        assert_eq!(
            refused("return io.stdout"),
            "json.edit: value: userdata cannot be JSON"
        );
    }

    #[test]
    fn nested_values_convert_recursively() {
        assert_eq!(
            converted("return { { matcher = 'Read' } }").as_deref(),
            Ok(r#"[{"matcher":"Read"}]"#)
        );
    }

    #[test]
    fn a_refusal_deep_inside_is_found() {
        assert_eq!(
            refused("return { a = { b = { 0/0 } } }"),
            "json.edit: value: NaN cannot be JSON"
        );
    }

    #[test]
    fn nil_is_null() {
        assert_eq!(converted("return nil").as_deref(), Ok("null"));
    }

    #[test]
    fn json_words_match_lua_words() {
        let words: Vec<LuaType> = [
            serde_json::json!({}),
            serde_json::json!([]),
            serde_json::json!(null),
            serde_json::json!(1),
            serde_json::json!("s"),
            serde_json::json!(true),
        ]
        .iter()
        .map(json_word)
        .collect();
        assert_eq!(
            words,
            [
                LuaType::Table,
                LuaType::List,
                LuaType::Nil,
                LuaType::Number,
                LuaType::String,
                LuaType::Boolean
            ]
        );
    }

    #[test]
    fn a_table_is_a_list_only_when_its_keys_are_one_to_n() {
        let lua = Lua::new();
        let list = |source: &str| -> Option<usize> {
            let Ok(Value::Table(table)) = lua.load(source).eval::<Value>() else {
                return None;
            };
            table_list(&table).map(|items| items.len())
        };
        assert_eq!(list("return {}"), Some(0));
        assert_eq!(list("return {'a', 'b'}"), Some(2));
        assert_eq!(list("return {a = 1}"), None);
        assert_eq!(list("return {1, a = 1}"), None);
        assert_eq!(list("return {[2] = 1}"), None);
        assert_eq!(list("return {[0] = 1, [1] = 2}"), None);
    }

    /// A list nested `depth` tables deep, the innermost holding `1`.
    fn nested(depth: usize) -> String {
        format!("local t = {{1}} for i = 2, {depth} do t = {{t}} end return t")
    }

    #[test]
    fn nesting_up_to_the_limit_converts_and_one_more_is_refused() {
        assert!(converted(&nested(128)).is_ok());
        assert_eq!(
            refused(&nested(129)),
            "json.edit: value: nested more than 128 levels deep"
        );
    }
}
