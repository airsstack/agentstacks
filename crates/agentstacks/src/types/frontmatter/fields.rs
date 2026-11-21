//! The checks skill and agent frontmatter share.
//!
//! The mapping is checked by hand, never through a struct that refuses unknown keys, so every
//! field error of a file is collected instead of only the first.
//!
//! # Responsibilities
//!
//! - [`FrontmatterFields`] holds the mapping entries, the diagnostics found so far and the tokens
//!   scanned so far.
//! - It reads required and optional strings, string arrays, the literal `name` and the opaque
//!   `native` table, each refusing a wrong shape at the key's location.
//! - [`FrontmatterFields::scan_text`] scans one free-text value once; every token and refusal in it
//!   is located at the value's key.
//! - [`FrontmatterFields::finish`] refuses every key the caller does not allow, in source order, and
//!   hands back the diagnostics and tokens.
//!
//! # Non-responsibilities
//!
//! Which keys a skill or an agent has, and the vocabularies of their values, belong to
//! [`SkillMeta`](crate::types::SkillMeta) and [`AgentMeta`](crate::types::AgentMeta). Values under `native` are
//! opaque and never scanned.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::wording::StringWording;
use crate::types::format::YamlValue;
use crate::types::frontmatter::error::FrontmatterError;
use crate::types::name::ItemName;
use crate::types::name::ToolName;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::FieldError;
use crate::types::report::FormatError;
use crate::types::report::Location;
use crate::types::report::TypeWord;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TextOrigin;
use crate::types::tokens::scan;

/// The key under which tool-specific values live.
const NATIVE_KEY: &str = "native";

/// The entries of one frontmatter mapping with the checks run over them so far.
#[derive(Debug)]
pub(super) struct FrontmatterFields {
    file: PathBuf,
    entries: Vec<(String, YamlValue)>,
    diagnostics: Diagnostics,
    tokens: Vec<LocatedToken>,
}

impl FrontmatterFields {
    /// The checks over `entries`, the mapping of `file`.
    pub(super) fn new(file: &Path, entries: Vec<(String, YamlValue)>) -> Self {
        Self {
            file: file.to_path_buf(),
            entries,
            diagnostics: Diagnostics::new(),
            tokens: Vec::new(),
        }
    }

    /// Refuses `key_path` with `error`.
    pub(super) fn refuse_at(&mut self, key_path: &str, error: impl Into<FormatError>) {
        let location = Location::at_key(&self.file, key_path);
        self.diagnostics.push(Diagnostic::new(location, error));
    }

    /// Refuses the file as a whole with `error`.
    fn refuse_file(&mut self, error: impl Into<FormatError>) {
        let location = Location::file(&self.file);
        self.diagnostics.push(Diagnostic::new(location, error));
    }

    fn value(&self, key: &str) -> Option<YamlValue> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.clone())
    }

    /// A required string: refused when missing, not a string, or blank.
    pub(super) fn required_string(&mut self, key: &str) -> Option<String> {
        let Some(value) = self.value(key) else {
            self.refuse_at(key, FieldError::Required);
            return None;
        };
        self.string_value(key, &value, StringWording::Generic)
    }

    /// An optional string: absent is `None` with no refusal; present is refused when not a string
    /// (worded as `wording` says) or blank.
    pub(super) fn optional_string(&mut self, key: &str, wording: StringWording) -> Option<String> {
        let value = self.value(key)?;
        self.string_value(key, &value, wording)
    }

    fn string_value(
        &mut self,
        key_path: &str,
        value: &YamlValue,
        wording: StringWording,
    ) -> Option<String> {
        let Some(text) = value.as_str() else {
            let got = value.kind_word();
            match wording {
                StringWording::Generic => self.refuse_at(
                    key_path,
                    FieldError::WrongType {
                        expected: TypeWord::String,
                        got,
                    },
                ),
                StringWording::WithArticle => {
                    self.refuse_at(key_path, FrontmatterError::NotAString { got });
                }
            }
            return None;
        };
        if let Err(error) = FieldError::check_not_empty(text) {
            self.refuse_at(key_path, error);
            return None;
        }
        Some(text.to_owned())
    }

    /// An optional array of strings. A value that is not an array is refused; an item that is not
    /// a string is refused at `<key>[<index>]`. The string items are returned in order, each with
    /// its index in the array, so a caller can refuse one at `<key>[<index>]` too.
    pub(super) fn optional_array(&mut self, key: &str) -> Option<Vec<(usize, String)>> {
        let value = self.value(key)?;
        let YamlValue::Seq(items) = value else {
            self.refuse_at(
                key,
                FieldError::WrongType {
                    expected: TypeWord::Array,
                    got: value.kind_word(),
                },
            );
            return None;
        };
        let mut strings = Vec::new();
        for (index, item) in items.iter().enumerate() {
            match item.as_str() {
                Some(text) => strings.push((index, text.to_owned())),
                None => self.refuse_at(
                    &format!("{key}[{index}]"),
                    FieldError::WrongType {
                        expected: TypeWord::String,
                        got: item.kind_word(),
                    },
                ),
            }
        }
        Some(strings)
    }

    /// The required `name`: a literal string equal to `expected`.
    ///
    /// A `name` holding `{{` is refused as not literal and is not compared.
    pub(super) fn name(&mut self, expected: &ItemName) -> Option<ItemName> {
        let found = self.required_string("name")?;
        if found.contains("{{") {
            self.refuse_file(FrontmatterError::NameNotLiteral);
            return None;
        }
        if found != expected.as_str() {
            self.refuse_file(FrontmatterError::NameMismatch {
                found,
                expected: expected.as_str().to_owned(),
            });
            return None;
        }
        Some(expected.clone())
    }

    /// The optional `native` table: tool names mapped to opaque values.
    ///
    /// Absent is an empty table. A value that is not a table is refused at `native`, and so is each
    /// key that is not a tool name, at `native.<key>`.
    pub(super) fn native(&mut self) -> BTreeMap<ToolName, YamlValue> {
        let mut native = BTreeMap::new();
        let Some(value) = self.value(NATIVE_KEY) else {
            return native;
        };
        let YamlValue::Map(entries) = value else {
            self.refuse_at(
                NATIVE_KEY,
                FieldError::WrongType {
                    expected: TypeWord::Table,
                    got: value.kind_word(),
                },
            );
            return native;
        };
        for (key, entry) in entries {
            match ToolName::new(&key) {
                Ok(tool) => {
                    native.insert(tool, entry);
                }
                Err(invalid) => self.refuse_at(
                    &format!("{NATIVE_KEY}.{key}"),
                    FrontmatterError::InvalidToolName {
                        value: invalid.value().to_owned(),
                    },
                ),
            }
        }
        native
    }

    /// Scans the free-text `value` of `key` once, keeping its tokens and refusals, all located at
    /// the key.
    pub(super) fn scan_text(&mut self, key: &str, value: &str) {
        let origin = TextOrigin::Field {
            file: self.file.clone(),
            key_path: key.to_owned(),
        };
        let (tokens, diagnostics) = scan(value, &origin);
        self.tokens.extend(tokens);
        self.diagnostics.extend(diagnostics);
    }

    /// Refuses each key not in `allowed_keys`, in source order, and returns every diagnostic and
    /// token.
    pub(super) fn finish(mut self, allowed_keys: &[&str]) -> (Diagnostics, Vec<LocatedToken>) {
        let unknown: Vec<String> = self
            .entries
            .iter()
            .filter(|(key, _)| !allowed_keys.contains(&key.as_str()))
            .map(|(key, _)| key.clone())
            .collect();
        for key in unknown {
            self.refuse_file(FrontmatterError::UnknownKey { key });
        }
        (self.diagnostics, self.tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenKind;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "SKILL.md";

    fn fields(block: &str) -> Result<FrontmatterFields, Box<dyn std::error::Error>> {
        match serde_saphyr::from_str::<YamlValue>(block)? {
            YamlValue::Map(entries) => Ok(FrontmatterFields::new(Path::new(FILE), entries)),
            other => Err(format!("test block is not a mapping: {other:?}").into()),
        }
    }

    fn shown(fields: FrontmatterFields, allowed: &[&str]) -> Vec<String> {
        fields
            .finish(allowed)
            .0
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn expected(name: &str) -> Result<ItemName, Box<dyn std::error::Error>> {
        Ok(ItemName::new(name)?)
    }

    #[test]
    fn required_string_missing_is_required() -> TestResult {
        let mut f = fields("name: x\n")?;
        assert_eq!(f.required_string("description"), None);
        assert_eq!(shown(f, &["name"]), ["SKILL.md: description: required"]);
        let mut f = fields("description: d\n")?;
        assert_eq!(f.required_string("name"), None);
        assert_eq!(shown(f, &["description"]), ["SKILL.md: name: required"]);
        Ok(())
    }

    #[test]
    fn string_of_wrong_kind_is_wrong_type() -> TestResult {
        let mut f = fields("description: 3\n")?;
        assert_eq!(f.required_string("description"), None);
        assert_eq!(
            shown(f, &["description"]),
            ["SKILL.md: description: expected string, got integer"]
        );
        let cases = [
            ("argument_hint: yes\n", "boolean"),
            ("argument_hint: [a]\n", "array"),
            ("argument_hint: {k: v}\n", "table"),
            ("argument_hint: ~\n", "null"),
        ];
        for (block, got) in cases {
            let mut f = fields(block)?;
            assert_eq!(
                f.optional_string("argument_hint", StringWording::WithArticle),
                None
            );
            assert_eq!(
                shown(f, &["argument_hint"]),
                [format!(
                    "SKILL.md: argument_hint: expected a string, got {got}"
                )],
                "{block}"
            );
        }
        Ok(())
    }

    #[test]
    fn empty_or_whitespace_string_is_empty() -> TestResult {
        for block in ["description: \"\"\n", "description: \"   \"\n"] {
            let mut f = fields(block)?;
            assert_eq!(f.required_string("description"), None);
            assert_eq!(
                shown(f, &["description"]),
                ["SKILL.md: description: empty"],
                "{block}"
            );
        }
        let mut f = fields("name: \"\"\n")?;
        assert_eq!(f.name(&expected("x")?), None);
        assert_eq!(shown(f, &["name"]), ["SKILL.md: name: empty"]);
        Ok(())
    }

    #[test]
    fn unknown_keys_each_reported_in_order() -> TestResult {
        let f = fields("disable-model-invocation: true\nargument-hint: x\n")?;
        assert_eq!(
            shown(f, &["name"]),
            [
                "SKILL.md: unknown key \"disable-model-invocation\"",
                "SKILL.md: unknown key \"argument-hint\"",
            ]
        );
        Ok(())
    }

    #[test]
    fn name_must_be_a_literal() -> TestResult {
        let mut f = fields("name: \"{{skill x}}\"\n")?;
        assert_eq!(f.name(&expected("x")?), None);
        assert_eq!(shown(f, &["name"]), ["SKILL.md: name must be a literal"]);
        Ok(())
    }

    #[test]
    fn name_must_match_expected() -> TestResult {
        let mut f = fields("name: x\n")?;
        assert_eq!(f.name(&expected("y")?), None);
        assert_eq!(
            shown(f, &["name"]),
            ["SKILL.md: name \"x\" does not match \"y\""]
        );
        let mut f = fields("name: discuss\n")?;
        assert_eq!(f.name(&expected("discuss")?), Some(expected("discuss")?));
        assert!(shown(f, &["name"]).is_empty());
        Ok(())
    }

    #[test]
    fn native_keys_must_be_tool_names() -> TestResult {
        let mut f = fields("native: { Claude: { model: x } }\n")?;
        assert!(f.native().is_empty());
        assert_eq!(
            shown(f, &["native"]),
            ["SKILL.md: native.Claude: \"Claude\" is not a valid tool name"]
        );
        Ok(())
    }

    #[test]
    fn native_must_be_a_mapping() -> TestResult {
        let mut f = fields("native: [a]\n")?;
        assert!(f.native().is_empty());
        assert_eq!(
            shown(f, &["native"]),
            ["SKILL.md: native: expected table, got array"]
        );
        Ok(())
    }

    #[test]
    fn string_values_are_scanned_with_key_path() -> TestResult {
        let mut f = fields("description: \"run {{skil x}}\"\n")?;
        f.scan_text("description", "run {{skil x}}");
        assert_eq!(
            shown(f, &["description"]),
            ["SKILL.md: description: unknown token kind \"skil\""]
        );

        let mut f = fields("description: x\n")?;
        f.scan_text("description", "run {{skill sdlc/status}} now");
        let (diagnostics, tokens) = f.finish(&["description"]);
        assert!(diagnostics.is_empty());
        assert_eq!(
            tokens.iter().map(|t| t.token().kind()).collect::<Vec<_>>(),
            [TokenKind::Skill]
        );
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()).as_deref(),
            Some("SKILL.md: description")
        );

        let mut f = fields("argument_hint: x\n")?;
        f.scan_text("argument_hint", "{{var arguments}}");
        let (_, tokens) = f.finish(&["argument_hint"]);
        assert_eq!(
            tokens.iter().map(|t| t.token().kind()).collect::<Vec<_>>(),
            [TokenKind::Var]
        );
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()).as_deref(),
            Some("SKILL.md: argument_hint")
        );
        Ok(())
    }

    #[test]
    fn native_values_are_never_scanned() -> TestResult {
        let mut f = fields("native: { claude: { args: [\"{{skil x}}\", \"{{root}}/x\"] } }\n")?;
        assert_eq!(f.native().len(), 1);
        let (diagnostics, tokens) = f.finish(&["native"]);
        assert!(diagnostics.is_empty());
        assert!(tokens.is_empty());
        Ok(())
    }

    #[test]
    fn array_items_must_be_strings() -> TestResult {
        let mut f = fields("tools: [a, 1]\n")?;
        assert_eq!(f.optional_array("tools"), Some(vec![(0, "a".to_owned())]));
        assert_eq!(
            shown(f, &["tools"]),
            ["SKILL.md: tools[1]: expected string, got integer"]
        );
        Ok(())
    }
}
