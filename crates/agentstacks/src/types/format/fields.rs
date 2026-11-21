//! Reads typed fields from one raw TOML table.
//!
//! A `TomlFields` reader sits over one table at a key-path prefix: empty at the top level, then
//! `pack`, `hooks[2]`, `hooks[2].grants`. Each read that finds a problem records a
//! [`Diagnostic`] at `<file>: <key.path>` and returns `None`, so reading goes on and every error
//! of a file is reported, not only the first.
//!
//! # Responsibilities
//!
//! - Typed reads (`required_string`, `required_text`, `optional_integer`, `optional_string_list`)
//!   refuse a missing required key, a wrong type, and for the text read an empty or
//!   whitespace-only string, with [`FieldError`]'s generic shapes.
//! - Child readers (`table_or_empty`, `optional_table`, `table_array`) carry the key path down:
//!   `hooks[2]`, `hooks[2].grants`; a list item's path carries its index (`requires.packs[1]`).
//! - `TomlFields::type_word` names a raw value's [`TypeWord`], so a datetime or a float where an
//!   integer is expected reads `expected integer, got datetime`.
//! - `refuse_unknown_keys` reports every key outside an allowed set, not only the first.
//! - `scan_field` scans one token-bearing string once and locates what it finds by key path.
//!
//! # Non-responsibilities
//!
//! Parsing the text into a table belongs to `parse_toml_document`, and
//! the rules of any one file's fields belong to the types that read that file.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::FieldError;
use crate::types::report::FormatError;
use crate::types::report::Location;
use crate::types::report::TypeWord;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TextOrigin;
use crate::types::tokens::scan;

/// A reader over one raw TOML table at a key-path prefix.
///
/// Crate-internal: it hands out `toml` types, which the public API does not expose.
#[derive(Debug)]
pub(crate) struct TomlFields {
    file: PathBuf,
    prefix: String,
    table: Table,
    diagnostics: Diagnostics,
}

impl TomlFields {
    /// A reader over `table`, which sits at `prefix` (empty at the top level) in `file`.
    #[must_use]
    pub(crate) fn new(file: &Path, prefix: &str, table: Table) -> Self {
        Self {
            file: file.to_path_buf(),
            prefix: prefix.to_owned(),
            table,
            diagnostics: Diagnostics::new(),
        }
    }

    /// The key path of `key` in this table: `prefix.key`, or just `key` at the top level.
    #[must_use]
    pub(crate) fn key_path(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.prefix)
        }
    }

    /// The key path of item `index` of the list at `key`: `prefix.key[index]`.
    #[must_use]
    pub(crate) fn item_path(&self, key: &str, index: usize) -> String {
        format!("{}[{index}]", self.key_path(key))
    }

    /// The raw value at `key`, for fields that word their own wrong-type refusal.
    #[must_use]
    pub(crate) fn raw(&self, key: &str) -> Option<&Value> {
        self.table.get(key)
    }

    /// The type word of a raw TOML value.
    #[must_use]
    pub(crate) const fn type_word(value: &Value) -> TypeWord {
        match value {
            Value::String(_) => TypeWord::String,
            Value::Integer(_) => TypeWord::Integer,
            Value::Float(_) => TypeWord::Float,
            Value::Boolean(_) => TypeWord::Boolean,
            Value::Datetime(_) => TypeWord::Datetime,
            Value::Array(_) => TypeWord::Array,
            Value::Table(_) => TypeWord::Table,
        }
    }

    /// A required string; refused when missing or not a string.
    pub(crate) fn required_string(&mut self, key: &str) -> Option<String> {
        if self.table.contains_key(key) {
            self.optional_string(key)
        } else {
            self.refuse(key, FieldError::Required);
            None
        }
    }

    /// An optional string; absent is `None` with no refusal, present is refused when not a
    /// string.
    pub(crate) fn optional_string(&mut self, key: &str) -> Option<String> {
        let value = self.table.get(key)?;
        if let Value::String(text) = value {
            return Some(text.clone());
        }
        let got = Self::type_word(value);
        self.refuse(
            key,
            FieldError::WrongType {
                expected: TypeWord::String,
                got,
            },
        );
        None
    }

    /// A required string that is not empty or whitespace-only.
    pub(crate) fn required_text(&mut self, key: &str) -> Option<String> {
        let text = self.required_string(key)?;
        self.checked_text(key, text)
    }

    fn checked_text(&mut self, key: &str, text: String) -> Option<String> {
        match FieldError::check_not_empty(&text) {
            Ok(()) => Some(text),
            Err(error) => {
                self.refuse(key, error);
                None
            }
        }
    }

    /// A required integer; refused when missing or not an integer.
    pub(crate) fn required_integer(&mut self, key: &str) -> Option<i64> {
        if self.table.contains_key(key) {
            self.optional_integer(key)
        } else {
            self.refuse(key, FieldError::Required);
            None
        }
    }

    /// An optional integer; absent is `None` with no refusal, present is refused when not an
    /// integer.
    pub(crate) fn optional_integer(&mut self, key: &str) -> Option<i64> {
        let value = self.table.get(key)?;
        if let Value::Integer(number) = value {
            return Some(*number);
        }
        let got = Self::type_word(value);
        self.refuse(
            key,
            FieldError::WrongType {
                expected: TypeWord::Integer,
                got,
            },
        );
        None
    }

    /// An optional array of strings, with each string item's index.
    ///
    /// A value that is not an array is refused at the key; an item that is not a string is refused
    /// at `key[index]` and left out.
    pub(crate) fn optional_string_list(&mut self, key: &str) -> Option<Vec<(usize, String)>> {
        let value = self.table.get(key)?;
        let Value::Array(items) = value else {
            let got = Self::type_word(value);
            self.refuse(
                key,
                FieldError::WrongType {
                    expected: TypeWord::Array,
                    got,
                },
            );
            return None;
        };
        let items = items.clone();
        let mut strings = Vec::new();
        for (index, item) in items.iter().enumerate() {
            if let Value::String(text) = item {
                strings.push((index, text.clone()));
            } else {
                let path = self.item_path(key, index);
                self.refuse_path(
                    &path,
                    FieldError::WrongType {
                        expected: TypeWord::String,
                        got: Self::type_word(item),
                    },
                );
            }
        }
        Some(strings)
    }

    /// A child reader over the table at `key`, or over an empty table when `key` is absent, so a
    /// missing table has its fields refused one by one.
    ///
    /// Returns `None`, after refusing the key, when the value is not a table.
    pub(crate) fn table_or_empty(&mut self, key: &str) -> Option<Self> {
        if self.table.contains_key(key) {
            self.optional_table(key)
        } else {
            Some(self.child(&self.key_path(key), Table::new()))
        }
    }

    /// A child reader over the table at `key`, or `None` when `key` is absent.
    ///
    /// Also `None`, after refusing the key, when the value is not a table.
    pub(crate) fn optional_table(&mut self, key: &str) -> Option<Self> {
        let value = self.table.get(key)?;
        if let Value::Table(table) = value {
            let table = table.clone();
            return Some(self.child(&self.key_path(key), table));
        }
        let got = Self::type_word(value);
        self.refuse(
            key,
            FieldError::WrongType {
                expected: TypeWord::Table,
                got,
            },
        );
        None
    }

    /// One child reader per item of the array of tables at `key`, at `key[index]`.
    ///
    /// A value that is not an array is refused at the key; an item that is not a table is refused
    /// at `key[index]` and left out. An absent key gives no readers.
    pub(crate) fn table_array(&mut self, key: &str) -> Vec<Self> {
        let Some(value) = self.table.get(key) else {
            return Vec::new();
        };
        let Value::Array(items) = value else {
            let got = Self::type_word(value);
            self.refuse(
                key,
                FieldError::WrongType {
                    expected: TypeWord::Array,
                    got,
                },
            );
            return Vec::new();
        };
        let items = items.clone();
        let mut readers = Vec::new();
        for (index, item) in items.into_iter().enumerate() {
            let path = self.item_path(key, index);
            if let Value::Table(table) = item {
                readers.push(self.child(&path, table));
            } else {
                self.refuse_path(
                    &path,
                    FieldError::WrongType {
                        expected: TypeWord::Table,
                        got: Self::type_word(&item),
                    },
                );
            }
        }
        readers
    }

    fn child(&self, prefix: &str, table: Table) -> Self {
        Self::new(&self.file, prefix, table)
    }

    /// The table's keys and raw values in key order, for tables whose keys are names.
    #[must_use]
    pub(crate) fn entries(&self) -> Vec<(String, Value)> {
        self.table
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// Refuses `key` with `error`.
    pub(crate) fn refuse(&mut self, key: &str, error: impl Into<FormatError>) {
        let path = self.key_path(key);
        self.refuse_path(&path, error);
    }

    /// Refuses the exact key path `key_path` with `error`, for list items and other paths this
    /// reader built itself.
    pub(crate) fn refuse_path(&mut self, key_path: &str, error: impl Into<FormatError>) {
        let location = Location::at_key(&self.file, key_path);
        self.diagnostics.push(Diagnostic::new(location, error));
    }

    /// Refuses the file as a whole with `error`.
    pub(crate) fn refuse_at_file(&mut self, error: impl Into<FormatError>) {
        let location = Location::file(&self.file);
        self.diagnostics.push(Diagnostic::new(location, error));
    }

    /// Refuses every key not in `allowed` as an unknown key, in key order.
    pub(crate) fn refuse_unknown_keys(&mut self, allowed: &[&str]) {
        let unknown: Vec<String> = self
            .table
            .keys()
            .filter(|key| !allowed.contains(&key.as_str()))
            .cloned()
            .collect();
        for key in unknown {
            self.refuse(&key, FieldError::UnknownKey);
        }
    }

    /// Scans `text`, the value of the field at `key_path`, once.
    ///
    /// Every token found is appended to `tokens`, located at `key_path`, and every token refusal is
    /// recorded. Returns whether the text held no refusal.
    pub(crate) fn scan_field(
        &mut self,
        key_path: &str,
        text: &str,
        tokens: &mut Vec<LocatedToken>,
    ) -> bool {
        let origin = TextOrigin::Field {
            file: self.file.clone(),
            key_path: key_path.to_owned(),
        };
        let (found, refusals) = scan(text, &origin);
        tokens.extend(found);
        let clean = refusals.is_empty();
        self.diagnostics.extend(refusals);
        clean
    }

    /// Moves the diagnostics of `child` into this reader.
    pub(crate) fn absorb(&mut self, child: Self) {
        self.diagnostics.extend(child.diagnostics);
    }

    /// How many diagnostics this reader holds, children absorbed so far included.
    ///
    /// A caller reads the count before and after a group of reads to learn whether the group was
    /// refused.
    #[must_use]
    pub(crate) const fn refusal_count(&self) -> usize {
        self.diagnostics.len()
    }

    /// Every diagnostic recorded so far.
    #[must_use]
    pub(crate) fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldError, parse_toml_document};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "packs/core/pack.toml";

    fn fields(prefix: &str, text: &str) -> Result<TomlFields, Box<dyn std::error::Error>> {
        let file = Path::new(FILE);
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        Ok(TomlFields::new(file, prefix, table))
    }

    fn shown(reader: TomlFields) -> Vec<String> {
        reader
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn toml_fields_reads_typed_values() -> TestResult {
        let mut reader = fields(
            "",
            "name = \"core\"\nn = 3\nflag = true\nlist = [\"a\", \"b\"]\n[child]\nx = 1\n",
        )?;
        assert_eq!(reader.required_string("name"), Some("core".to_owned()));
        assert_eq!(reader.required_integer("n"), Some(3));
        assert_eq!(
            reader.optional_string_list("list"),
            Some(vec![(0, "a".to_owned()), (1, "b".to_owned())])
        );
        let mut child = reader.optional_table("child").ok_or("child table")?;
        assert_eq!(child.optional_integer("x"), Some(1));
        reader.absorb(child);
        assert_eq!(reader.optional_string("missing"), None);
        assert!(reader.into_diagnostics().is_empty());
        Ok(())
    }

    #[test]
    fn type_word_names_each_toml_type() -> TestResult {
        let table = parse_toml_document(
            Path::new(FILE),
            "a = true\nb = 1\nc = 1.5\nd = \"a\"\ne = 1979-05-27\nf = [1]\ng = {a = 1}\n",
        )
        .map_err(|d| d.to_string())?;
        let words: Vec<String> = ["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|key| {
                table
                    .get(*key)
                    .map_or_else(String::new, |v| TomlFields::type_word(v).to_string())
            })
            .collect();
        assert_eq!(
            words,
            [
                "boolean", "integer", "float", "string", "datetime", "array", "table"
            ]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_reports_missing_and_wrong_types() -> TestResult {
        let mut empty = fields("pack", "")?;
        assert_eq!(empty.required_string("name"), None);
        assert_eq!(shown(empty), ["packs/core/pack.toml: pack.name: required"]);

        let cases = [
            ("api = \"0\"", "string"),
            ("api = 0.5", "float"),
            ("api = 1979-05-27", "datetime"),
        ];
        for (text, got) in cases {
            let mut reader = fields("pack", text)?;
            assert_eq!(reader.required_integer("api"), None, "{text}");
            assert_eq!(
                shown(reader),
                [format!(
                    "packs/core/pack.toml: pack.api: expected integer, got {got}"
                )],
                "{text}"
            );
        }
        let mut reader = fields("", "k = 1")?;
        assert_eq!(reader.optional_string("k"), None);
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: k: expected string, got integer"]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_reports_empty_and_whitespace_text() -> TestResult {
        for text in ["description = \"\"", "description = \"   \""] {
            let mut reader = fields("pack", text)?;
            assert_eq!(reader.required_text("description"), None, "{text}");
            assert_eq!(
                shown(reader),
                ["packs/core/pack.toml: pack.description: empty"],
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn toml_fields_locates_list_items_by_index() -> TestResult {
        let mut reader = fields("requires", "packs = [\"a\", 1]")?;
        assert_eq!(
            reader.optional_string_list("packs"),
            Some(vec![(0, "a".to_owned())])
        );
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: requires.packs[1]: expected string, got integer"]
        );
        let mut reader = fields("requires", "packs = \"a\"")?;
        assert_eq!(reader.optional_string_list("packs"), None);
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: requires.packs: expected array, got string"]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_builds_nested_key_paths() -> TestResult {
        let mut reader = fields(
            "",
            "[[hooks]]\n[[hooks]]\n[[hooks]]\n[hooks.grants]\nnet = []\n",
        )?;
        let mut items = reader.table_array("hooks");
        assert_eq!(items.len(), 3);
        let mut third = items.pop().ok_or("third hook")?;
        let mut grants = third.table_or_empty("grants").ok_or("grants table")?;
        grants.refuse_unknown_keys(&["policy"]);
        third.absorb(grants);
        reader.absorb(third);
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: hooks[2].grants.net: unknown key"]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_reports_every_unknown_key() -> TestResult {
        let mut reader = fields("pack", "name = \"x\"\nfoo = 1\nbar = 2\n")?;
        reader.refuse_unknown_keys(&["name"]);
        assert_eq!(
            shown(reader),
            [
                "packs/core/pack.toml: pack.bar: unknown key",
                "packs/core/pack.toml: pack.foo: unknown key"
            ]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_handles_absent_and_non_table_children() -> TestResult {
        let mut reader = fields("", "pack = 1")?;
        assert!(reader.optional_table("requires").is_none());
        let empty = reader.table_or_empty("requires").ok_or("empty child")?;
        assert!(empty.entries().is_empty());
        assert!(reader.table_or_empty("pack").is_none());
        assert!(reader.table_array("pack").is_empty());
        assert_eq!(
            shown(reader),
            [
                "packs/core/pack.toml: pack: expected table, got integer",
                "packs/core/pack.toml: pack: expected array, got integer"
            ]
        );
        let mut reader = fields("", "hooks = [1, {a = 1}]")?;
        assert_eq!(reader.table_array("hooks").len(), 1);
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: hooks[0]: expected table, got integer"]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_refuses_with_any_error_at_key_path_or_file() -> TestResult {
        let mut reader = fields("pack", "")?;
        reader.refuse("name", FieldError::Required);
        reader.refuse_path("pack.items[1]", FieldError::Empty);
        reader.refuse_at_file(FieldError::UnknownKey);
        assert_eq!(
            shown(reader),
            [
                "packs/core/pack.toml: pack.name: required",
                "packs/core/pack.toml: pack.items[1]: empty",
                "packs/core/pack.toml: unknown key"
            ]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_scans_a_field_once_by_key_path() -> TestResult {
        let mut reader = fields("hooks[0]", "")?;
        let mut tokens = Vec::new();
        assert!(reader.scan_field("hooks[0].text", "run {{skill x}}", &mut tokens));
        assert_eq!(
            tokens
                .iter()
                .map(|t| t.location().to_string())
                .collect::<Vec<_>>(),
            ["packs/core/pack.toml: hooks[0].text"]
        );
        assert!(!reader.scan_field("hooks[0].text", "{{skil x}}", &mut tokens));
        assert_eq!(
            shown(reader),
            ["packs/core/pack.toml: hooks[0].text: unknown token kind \"skil\""]
        );
        Ok(())
    }

    #[test]
    fn toml_fields_counts_its_refusals() -> TestResult {
        let mut reader = fields("", "a = 1")?;
        assert_eq!(reader.refusal_count(), 0);
        assert_eq!(reader.optional_string("a"), None);
        reader.refuse_unknown_keys(&[]);
        assert_eq!(reader.refusal_count(), 2);
        Ok(())
    }

    #[test]
    fn toml_fields_entries_list_keys_in_order() -> TestResult {
        let reader = fields("options", "b = 1\na = 2\n")?;
        let keys: Vec<String> = reader.entries().into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, ["a", "b"]);
        Ok(())
    }
}
