//! A tool extension's `extension.toml`, parsed and checked.
//!
//! It is separate from [`Extension`](crate::types::Extension) because it works on the text of one
//! file with no directory, so it can be parsed and tested alone; the loader adds the directory
//! checks around it.
//!
//! # Responsibilities
//!
//! - [`ExtensionManifest::parse`] reads the whole file and returns the manifest, or every refusal
//!   of the file at once: a field's refusal never hides the others.
//! - A syntax error stops only this file and is its only diagnostic; empty text is an empty table,
//!   so each missing field is refused by name.
//! - It reads `[extension]` (`name`, `version`, `api`, `description`, `entry`) and refuses any
//!   other key at either level.
//! - An `api` other than [`API`] is refused here and nowhere else: a refused manifest yields no
//!   extension, so no later check sees it.
//!
//! # Non-responsibilities
//!
//! Whether `entry` names an existing file needs the directory, so the extension loader decides it.

use std::path::Path;

use crate::types::api::API;
use crate::types::extension::EntryPath;
use crate::types::extension::error::ExtensionError;
use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::manifest::PackVersion;
use crate::types::name::ToolName;
use crate::types::report::Diagnostics;

/// A parsed `extension.toml`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::ExtensionManifest;
///
/// let text = "[extension]\n\
///     name = \"claude\"\n\
///     version = \"0.1.0\"\n\
///     api = 0\n\
///     description = \"Renders packs into Claude Code project config\"\n\
///     entry = \"adapter.lua\"\n";
/// let manifest = ExtensionManifest::parse(Path::new("extension.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(manifest.name().as_str(), "claude");
/// assert_eq!(manifest.version().to_string(), "0.1.0");
/// assert_eq!(manifest.description(), "Renders packs into Claude Code project config");
/// assert_eq!(manifest.entry().as_str(), "adapter.lua");
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone)]
pub struct ExtensionManifest {
    name: ToolName,
    version: PackVersion,
    description: String,
    entry: EntryPath,
}

impl ExtensionManifest {
    /// Parses `text`, the content of the `extension.toml` at `file`.
    ///
    /// # Errors
    ///
    /// Returns every refusal of the file when any field is refused; the [`Diagnostics`] hand them
    /// back sorted.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::ExtensionManifest;
    ///
    /// // A manifest that lacks its fields is refused, every field by name.
    /// let refused = ExtensionManifest::parse(Path::new("extension.toml"), "[extension]\n")
    ///     .err()
    ///     .ok_or("a manifest with no fields is refused")?;
    /// assert!(refused.into_sorted().len() > 1);
    /// # Ok::<(), &str>(())
    /// ```
    pub fn parse(file: &Path, text: &str) -> Result<Self, Diagnostics> {
        Self::parse_with_entry(file, text).0
    }

    /// Parses like [`ExtensionManifest::parse`] and also returns the `entry` whenever that field
    /// itself is valid, even if another field was refused, so the loader can check it against the
    /// directory in the same run.
    pub(crate) fn parse_with_entry(
        file: &Path,
        text: &str,
    ) -> (Result<Self, Diagnostics>, Option<EntryPath>) {
        let table = match parse_toml_document(file, text) {
            Ok(table) => table,
            Err(syntax) => {
                let mut diagnostics = Diagnostics::new();
                diagnostics.push(syntax);
                return (Err(diagnostics), None);
            }
        };
        let mut root = TomlFields::new(file, "", table);
        root.refuse_unknown_keys(&["extension"]);
        let read = root.table_or_empty("extension").map(|mut section| {
            let read = read_section(&mut section);
            root.absorb(section);
            read
        });
        let diagnostics = root.into_diagnostics();
        let Some(read) = read else {
            return (Err(diagnostics), None);
        };
        let valid_entry = read.entry.clone();
        let manifest = match (read, diagnostics.is_empty()) {
            (
                Section {
                    name: Some(name),
                    version: Some(version),
                    description: Some(description),
                    entry: Some(entry),
                },
                true,
            ) => Ok(Self {
                name,
                version,
                description,
                entry,
            }),
            _ => Err(diagnostics),
        };
        (manifest, valid_entry)
    }

    /// The extension's name.
    #[must_use]
    pub const fn name(&self) -> &ToolName {
        &self.name
    }

    /// The extension's version.
    #[must_use]
    pub const fn version(&self) -> &PackVersion {
        &self.version
    }

    /// The extension's description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The Lua file the extension is run from, relative to its directory.
    #[must_use]
    pub const fn entry(&self) -> &EntryPath {
        &self.entry
    }
}

/// The fields of `[extension]` that read cleanly; `api` is checked and not kept.
struct Section {
    name: Option<ToolName>,
    version: Option<PackVersion>,
    description: Option<String>,
    entry: Option<EntryPath>,
}

/// Reads and checks every field of `[extension]`, refusing each failure and going on.
fn read_section(section: &mut TomlFields) -> Section {
    let name = section.required_string("name").and_then(|text| {
        ToolName::new(&text)
            .map_err(|error| section.refuse("name", ExtensionError::from(error)))
            .ok()
    });
    let version = section.required_string("version").and_then(|text| {
        PackVersion::new(&text)
            .map_err(|error| section.refuse("version", ExtensionError::from(error)))
            .ok()
    });
    if let Some(api) = section.required_integer("api").filter(|api| *api != API) {
        section.refuse("api", ExtensionError::ApiUnsupported { api });
    }
    let description = section.required_text("description");
    let entry = section.required_text("entry").and_then(|text| {
        EntryPath::new(&text)
            .map_err(|error| section.refuse("entry", error))
            .ok()
    });
    section.refuse_unknown_keys(&["name", "version", "api", "description", "entry"]);
    Section {
        name,
        version,
        description,
        entry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "extension.toml";

    const EXAMPLE: &str = "[extension]\n\
        name        = \"claude\"\n\
        version     = \"0.1.0\"\n\
        api         = 0\n\
        description = \"Renders packs into Claude Code project config\"\n\
        entry       = \"adapter.lua\"\n";

    /// The example with the line that sets `key` replaced by `line`.
    fn with_line(key: &str, line: &str) -> String {
        EXAMPLE
            .lines()
            .map(|existing| {
                if existing.trim_start().starts_with(key) {
                    line
                } else {
                    existing
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The example without the line that sets `key`.
    fn without_line(key: &str) -> String {
        EXAMPLE
            .lines()
            .filter(|existing| !existing.trim_start().starts_with(key))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn shown(result: Result<ExtensionManifest, Diagnostics>) -> Vec<String> {
        result
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.to_string())
            .collect()
    }

    fn parse(text: &str) -> Vec<String> {
        shown(ExtensionManifest::parse(Path::new(FILE), text))
    }

    #[test]
    fn extension_manifest_parses_the_documented_example() -> TestResult {
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), EXAMPLE).map_err(|d| format!("{d:?}"))?;
        assert_eq!(manifest.name().as_str(), "claude");
        assert_eq!(manifest.version().to_string(), "0.1.0");
        assert_eq!(
            manifest.description(),
            "Renders packs into Claude Code project config"
        );
        assert_eq!(manifest.entry().as_str(), "adapter.lua");
        Ok(())
    }

    #[test]
    fn extension_manifest_reports_a_syntax_error_with_its_line() {
        assert_eq!(
            parse("[extension]\nname = \"claude\"\na = [1,"),
            ["extension.toml:3: unclosed array, expected `]`"]
        );
    }

    #[test]
    fn extension_manifest_refuses_an_empty_file_field_by_field() {
        assert_eq!(
            parse(""),
            [
                "extension.toml: extension.name: required",
                "extension.toml: extension.version: required",
                "extension.toml: extension.api: required",
                "extension.toml: extension.description: required",
                "extension.toml: extension.entry: required",
            ]
        );
    }

    #[test]
    fn extension_manifest_refuses_a_wrong_typed_extension_table() {
        assert_eq!(
            parse("extension = \"x\""),
            ["extension.toml: extension: expected table, got string"]
        );
    }

    #[test]
    fn extension_manifest_requires_every_field() {
        for field in ["name", "version", "api", "description", "entry"] {
            assert_eq!(
                parse(&without_line(field)),
                [format!("extension.toml: extension.{field}: required")],
                "{field}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_wrong_types() {
        let cases = [
            (
                "api",
                "api = \"0\"",
                "extension.api: expected integer, got string",
            ),
            (
                "api",
                "api = 0.5",
                "extension.api: expected integer, got float",
            ),
            (
                "name",
                "name = 1",
                "extension.name: expected string, got integer",
            ),
            (
                "version",
                "version = 1979-05-27",
                "extension.version: expected string, got datetime",
            ),
        ];
        for (key, line, message) in cases {
            assert_eq!(
                parse(&with_line(key, line)),
                [format!("extension.toml: {message}")],
                "{line}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_an_invalid_name() {
        assert_eq!(
            parse(&with_line("name", "name = \"Claude\"")),
            ["extension.toml: extension.name: \"Claude\" is not a valid name"]
        );
    }

    #[test]
    fn extension_manifest_refuses_a_non_semver_version() {
        assert_eq!(
            parse(&with_line("version", "version = \"1.2\"")),
            ["extension.toml: extension.version: \"1.2\" is not semver"]
        );
    }

    #[test]
    fn extension_manifest_refuses_an_empty_description() {
        for value in ["", "   "] {
            assert_eq!(
                parse(&with_line(
                    "description",
                    &format!("description = \"{value}\"")
                )),
                ["extension.toml: extension.description: empty"],
                "{value:?}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_api_other_than_zero() {
        for api in [1, -1] {
            assert_eq!(
                parse(&with_line("api", &format!("api = {api}"))),
                [format!(
                    "extension.toml: extension.api: {api} not supported (this agentstacks speaks 0)"
                )],
                "{api}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_bad_entries() {
        let cases = [
            (
                "../a.lua",
                "extension.entry: \"../a.lua\" must stay inside the extension",
            ),
            (
                "/a.lua",
                "extension.entry: \"/a.lua\" must stay inside the extension",
            ),
            (
                "adapter.js",
                "extension.entry: \"adapter.js\" is not a .lua file",
            ),
            ("", "extension.entry: empty"),
        ];
        for (value, message) in cases {
            assert_eq!(
                parse(&with_line("entry", &format!("entry = \"{value}\""))),
                [format!("extension.toml: {message}")],
                "{value:?}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_unknown_keys() {
        assert_eq!(
            parse(&with_line("entry", "entry = \"adapter.lua\"\nfoo = 1")),
            ["extension.toml: extension.foo: unknown key"]
        );
        assert_eq!(
            parse(&format!("foo = 1\n{EXAMPLE}")),
            ["extension.toml: foo: unknown key"]
        );
    }

    #[test]
    fn extension_manifest_reports_all_errors_at_once() {
        let text = "foo = 1\n\
            [extension]\n\
            name = \"Claude\"\n\
            version = \"1.2\"\n\
            api = 1\n\
            description = \"d\"\n\
            entry = \"adapter.js\"\n";
        let mut shown = parse(text);
        shown.sort();
        let mut expected = vec![
            "extension.toml: foo: unknown key".to_owned(),
            "extension.toml: extension.name: \"Claude\" is not a valid name".to_owned(),
            "extension.toml: extension.version: \"1.2\" is not semver".to_owned(),
            "extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)".to_owned(),
            "extension.toml: extension.entry: \"adapter.js\" is not a .lua file".to_owned(),
        ];
        expected.sort();
        assert_eq!(shown, expected);
    }

    #[test]
    fn parse_with_entry_keeps_a_valid_entry_when_another_field_fails() -> TestResult {
        let (result, entry) = ExtensionManifest::parse_with_entry(Path::new(FILE), EXAMPLE);
        assert!(result.is_ok());
        assert_eq!(entry.ok_or("entry kept")?.as_str(), "adapter.lua");

        let (result, entry) = ExtensionManifest::parse_with_entry(
            Path::new(FILE),
            &with_line("name", "name = \"Claude\""),
        );
        assert_eq!(
            shown(result),
            ["extension.toml: extension.name: \"Claude\" is not a valid name"]
        );
        assert_eq!(entry.ok_or("entry kept")?.as_str(), "adapter.lua");

        let (result, entry) = ExtensionManifest::parse_with_entry(
            Path::new(FILE),
            &with_line("entry", "entry = \"adapter.js\""),
        );
        assert_eq!(shown(result).len(), 1);
        assert!(entry.is_none());
        Ok(())
    }
}
