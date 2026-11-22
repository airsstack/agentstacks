//! The fields of one call's argument table, read strictly.
//!
//! # Responsibilities
//!
//! - [`ArgTable::read`] reads the table a call was given against the fields the call takes, with
//!   no coercion, and returns the first failing check. The checks run in this order, each over
//!   every field in the order given before the next check starts:
//!   1. the argument is a table;
//!   2. each required field is present, where `nil` counts as missing;
//!   3. each field has exactly the type it takes, where an integer and a float are both
//!      `number`;
//!   4. a list field is a table whose keys are exactly `1..n`, and each of its entries is a
//!      string;
//!   5. every key is a string, and every key is one the call takes;
//!   6. each string field except raw bytes, and each entry of a list, is UTF-8.
//! - Raw bytes keep every byte, `\0` and invalid UTF-8 included, and may be empty. An empty list
//!   is the same as none.
//!
//! # Non-responsibilities
//!
//! What the values mean, and where a refusal is located, belong to the call that reads them.

use std::collections::BTreeMap;

use airsl::mlua::Value;

use crate::host::convert::{entries, lua_type, table_list};
use crate::types::op::{ApiFunction, LuaType, OpError};

/// What a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Want {
    /// A UTF-8 string.
    Text,
    /// A string whose bytes are kept as written.
    Bytes,
    /// A list of UTF-8 strings.
    List,
}

/// One field a call takes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Field {
    name: &'static str,
    want: Want,
    required: bool,
}

impl Field {
    /// A required string that must be UTF-8.
    pub(super) const fn text(name: &'static str) -> Self {
        Self {
            name,
            want: Want::Text,
            required: true,
        }
    }

    /// A required string whose bytes are kept as written.
    pub(super) const fn bytes(name: &'static str) -> Self {
        Self {
            name,
            want: Want::Bytes,
            required: true,
        }
    }

    /// A list of UTF-8 strings that may be left out.
    pub(super) const fn list(name: &'static str) -> Self {
        Self {
            name,
            want: Want::List,
            required: false,
        }
    }
}

/// What a field was given.
#[derive(Debug)]
enum Given {
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<String>),
}

/// The fields of a call, checked and read.
#[derive(Debug)]
pub(super) struct ArgTable {
    given: BTreeMap<&'static str, Given>,
}

impl ArgTable {
    /// Reads `argument` as the table of `function`, which takes `fields`.
    ///
    /// # Errors
    ///
    /// The refusal of the first failing check.
    pub(super) fn read(
        function: ApiFunction,
        argument: &Value,
        fields: &[Field],
    ) -> Result<Self, OpError> {
        let Value::Table(table) = argument else {
            return Err(OpError::ArgumentType {
                function,
                expected: LuaType::Table,
                got: lua_type(argument),
            });
        };
        let present: Vec<(&Field, Value)> = fields
            .iter()
            .map(|field| (field, table.raw_get(field.name).unwrap_or(Value::Nil)))
            .collect();
        for (field, value) in &present {
            if field.required && matches!(value, Value::Nil) {
                return Err(OpError::Required {
                    function,
                    field: field.name.to_owned(),
                });
            }
        }
        for (field, value) in &present {
            check_type(function, field, value)?;
        }
        let mut lists: BTreeMap<&'static str, Vec<Value>> = BTreeMap::new();
        for (field, value) in &present {
            if let (Want::List, Value::Table(list)) = (field.want, value) {
                lists.insert(field.name, list_entries(function, field, list)?);
            }
        }
        check_keys(function, table, fields)?;
        let mut given = BTreeMap::new();
        for (field, value) in &present {
            if let Some(read) = read_field(function, field, value, lists.get(field.name))? {
                given.insert(field.name, read);
            }
        }
        Ok(Self { given })
    }

    /// The UTF-8 string `name` holds, or `""` for a field that is not a string.
    pub(super) fn string(&self, name: &str) -> &str {
        match self.given.get(name) {
            Some(Given::Text(text)) => text,
            _ => "",
        }
    }

    /// The bytes `name` holds, or none for a field that is not raw bytes.
    pub(super) fn bytes(&self, name: &str) -> &[u8] {
        match self.given.get(name) {
            Some(Given::Bytes(bytes)) => bytes,
            _ => &[],
        }
    }

    /// The strings the list `name` holds, or none for a field that is not a list.
    pub(super) fn string_list(&self, name: &str) -> &[String] {
        match self.given.get(name) {
            Some(Given::List(items)) => items,
            _ => &[],
        }
    }
}

/// Refuses `value` when it is not the type `field` takes. A missing optional field passes.
fn check_type(function: ApiFunction, field: &Field, value: &Value) -> Result<(), OpError> {
    let expected = match (field.want, value) {
        (_, Value::Nil)
        | (Want::Text | Want::Bytes, Value::String(_))
        | (Want::List, Value::Table(_)) => return Ok(()),
        (Want::Text | Want::Bytes, _) => LuaType::String,
        (Want::List, _) => LuaType::Table,
    };
    Err(OpError::FieldType {
        function,
        field: field.name.to_owned(),
        expected,
        got: lua_type(value),
    })
}

/// The entries of the list `field` holds, which must be a list of strings.
fn list_entries(
    function: ApiFunction,
    field: &Field,
    list: &airsl::mlua::Table,
) -> Result<Vec<Value>, OpError> {
    let items = table_list(list).ok_or_else(|| OpError::NotList {
        function,
        field: field.name.to_owned(),
    })?;
    for (index, item) in items.iter().enumerate() {
        if !matches!(item, Value::String(_)) {
            return Err(OpError::FieldType {
                function,
                field: format!("{}[{}]", field.name, index + 1),
                expected: LuaType::String,
                got: lua_type(item),
            });
        }
    }
    Ok(items)
}

/// Refuses a key that is not a string, then the first string key, by bytes, that is not one of
/// `fields`.
fn check_keys(
    function: ApiFunction,
    table: &airsl::mlua::Table,
    fields: &[Field],
) -> Result<(), OpError> {
    let keys: Vec<Value> = entries(table).into_iter().map(|(key, _)| key).collect();
    if let Some(key) = keys.iter().find(|key| !matches!(key, Value::String(_))) {
        return Err(OpError::KeyType {
            function,
            got: lua_type(key),
        });
    }
    let mut unknown: Vec<(Vec<u8>, String)> = keys
        .iter()
        .filter_map(|key| match key {
            Value::String(text) => Some((text.as_bytes().to_vec(), text.to_string_lossy())),
            _ => None,
        })
        .filter(|(bytes, _)| !fields.iter().any(|field| field.name.as_bytes() == bytes))
        .collect();
    unknown.sort();
    match unknown.into_iter().next() {
        Some((_, key)) => Err(OpError::UnknownKey { function, key }),
        None => Ok(()),
    }
}

/// What `field` holds, with its UTF-8 checked, or `None` for an optional field left out.
fn read_field(
    function: ApiFunction,
    field: &Field,
    value: &Value,
    list: Option<&Vec<Value>>,
) -> Result<Option<Given>, OpError> {
    let not_utf8 = |name: String| OpError::NotUtf8 {
        function,
        field: name,
    };
    match (field.want, value) {
        (Want::Text, Value::String(text)) => std::str::from_utf8(&text.as_bytes())
            .map(|text| Some(Given::Text(text.to_owned())))
            .map_err(|_| not_utf8(field.name.to_owned())),
        (Want::Bytes, Value::String(text)) => Ok(Some(Given::Bytes(text.as_bytes().to_vec()))),
        (Want::List, Value::Table(_)) => {
            let mut items = Vec::new();
            for (index, item) in list.into_iter().flatten().enumerate() {
                if let Value::String(text) = item {
                    let text = std::str::from_utf8(&text.as_bytes())
                        .map(str::to_owned)
                        .map_err(|_| not_utf8(format!("{}[{}]", field.name, index + 1)))?;
                    items.push(text);
                }
            }
            Ok(Some(Given::List(items)))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use airsl::mlua::Lua;

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    const WRITE: [Field; 3] = [
        Field::text("path"),
        Field::bytes("content"),
        Field::list("covers"),
    ];

    fn read(chunk: &str, fields: &[Field]) -> Result<Result<ArgTable, String>, Box<dyn Error>> {
        let lua = Lua::new();
        let value: Value = lua.load(chunk).eval()?;
        Ok(ArgTable::read(ApiFunction::Write, &value, fields).map_err(|error| error.to_string()))
    }

    fn refusal(chunk: &str, fields: &[Field]) -> Result<String, Box<dyn Error>> {
        read(chunk, fields)?
            .err()
            .ok_or_else(|| "the argument should be refused".into())
    }

    #[test]
    fn argument_must_be_a_table() -> TestResult {
        assert_eq!(
            refusal("return 'x'", &WRITE)?,
            "write: expected table, got string"
        );
        Ok(())
    }

    #[test]
    fn nil_field_counts_as_missing() -> TestResult {
        assert_eq!(
            refusal("return {content = 'x'}", &WRITE)?,
            "write: path: required"
        );
        assert_eq!(
            refusal("return {path = 'a', content = nil}", &WRITE)?,
            "write: content: required"
        );
        Ok(())
    }

    #[test]
    fn no_coercion_of_numbers() -> TestResult {
        assert_eq!(
            refusal("return {path = 5, content = ''}", &WRITE)?,
            "write: path: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn float_is_also_number() -> TestResult {
        assert_eq!(
            refusal("return {path = 5.5, content = ''}", &WRITE)?,
            "write: path: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn covers_must_be_a_table() -> TestResult {
        assert_eq!(
            refusal("return {path = 'a', content = '', covers = 'x'}", &WRITE)?,
            "write: covers: expected table, got string"
        );
        Ok(())
    }

    #[test]
    fn covers_must_be_a_list() -> TestResult {
        assert_eq!(
            refusal(
                "return {path = 'a', content = '', covers = {a = 'x'}}",
                &WRITE
            )?,
            "write: covers: expected a list"
        );
        Ok(())
    }

    #[test]
    fn covers_elements_must_be_strings() -> TestResult {
        assert_eq!(
            refusal(
                "return {path = 'a', content = '', covers = {'a', true}}",
                &WRITE
            )?,
            "write: covers[2]: expected string, got boolean"
        );
        Ok(())
    }

    #[test]
    fn empty_covers_is_none() -> TestResult {
        let table = read("return {path = 'a', content = '', covers = {}}", &WRITE)??;
        assert!(table.string_list("covers").is_empty());
        let table = read("return {path = 'a', content = ''}", &WRITE)??;
        assert!(table.string_list("covers").is_empty());
        Ok(())
    }

    #[test]
    fn covers_entries_are_read_in_order() -> TestResult {
        let table = read(
            "return {path = 'a', content = '', covers = {'x', 'y'}}",
            &WRITE,
        )??;
        assert_eq!(table.string_list("covers"), ["x", "y"]);
        assert_eq!(table.string("path"), "a");
        Ok(())
    }

    #[test]
    fn unknown_key_is_refused() -> TestResult {
        assert_eq!(
            refusal("return {path = 'a', content = '', foo = 1}", &WRITE)?,
            "write: foo: unknown key"
        );
        Ok(())
    }

    #[test]
    fn non_string_key_is_refused() -> TestResult {
        assert_eq!(
            refusal("return {path = 'a', content = '', [1] = 'x'}", &WRITE)?,
            "write: keys must be strings, got number"
        );
        Ok(())
    }

    #[test]
    fn path_must_be_utf8() -> TestResult {
        assert_eq!(
            refusal("return {path = 'a\\255', content = ''}", &WRITE)?,
            "write: path: not valid UTF-8"
        );
        Ok(())
    }

    #[test]
    fn other_string_fields_must_be_utf8() -> TestResult {
        let skip = [Field::text("item"), Field::text("reason")];
        assert_eq!(
            refusal("return {item = 'a\\255', reason = 'r'}", &skip)?,
            "write: item: not valid UTF-8"
        );
        Ok(())
    }

    #[test]
    fn list_entries_must_be_utf8() -> TestResult {
        assert_eq!(
            refusal(
                "return {path = 'a', content = '', covers = {'x', 'y\\255'}}",
                &WRITE
            )?,
            "write: covers[2]: not valid UTF-8"
        );
        Ok(())
    }

    #[test]
    fn content_keeps_raw_bytes() -> TestResult {
        let table = read("return {path = 'a', content = 'a\\255b\\0c'}", &WRITE)??;
        assert_eq!(table.bytes("content"), [97, 255, 98, 0, 99]);
        Ok(())
    }

    #[test]
    fn empty_content_is_accepted() -> TestResult {
        let table = read("return {path = 'a', content = ''}", &WRITE)??;
        assert!(table.bytes("content").is_empty());
        Ok(())
    }

    #[test]
    fn first_failing_check_wins() -> TestResult {
        let both = [Field::text("path"), Field::bytes("content")];
        assert_eq!(refusal("return {foo = 1}", &both)?, "write: path: required");
        Ok(())
    }

    #[test]
    fn required_row_precedes_type_row() -> TestResult {
        let both = [Field::text("path"), Field::bytes("content")];
        assert_eq!(
            refusal("return {path = 5}", &both)?,
            "write: content: required"
        );
        Ok(())
    }

    #[test]
    fn type_row_precedes_the_key_rows() -> TestResult {
        assert_eq!(
            refusal("return {path = 5, content = '', foo = 1}", &WRITE)?,
            "write: path: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn key_rows_precede_the_utf8_row() -> TestResult {
        assert_eq!(
            refusal("return {path = 'a\\255', content = '', foo = 1}", &WRITE)?,
            "write: foo: unknown key"
        );
        Ok(())
    }
}
