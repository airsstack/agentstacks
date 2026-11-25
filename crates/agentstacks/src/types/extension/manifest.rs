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
//! - It reads `[extension]` (`name`, `version`, `api`, `description`, `entry`, and the optional
//!   `reads`) and the optional `[requires]` table, which holds only `agentstacks`, a version
//!   range; it refuses any other key at any of these levels.
//! - `reads` lists the project files the extension may read. Each entry is a project-relative path
//!   with no `..` segment and no `\`, kept as written; a path that repeats an earlier entry after
//!   normalising is refused. agentstacks' own files are allowed there.
//! - An `api` other than [`API`](crate::types::API) is refused here and nowhere else, as legacy
//!   when it is the `0` of the `0.x` releases and as unsupported otherwise: a refused manifest
//!   yields no extension, so no later check sees it.
//!
//! # Non-responsibilities
//!
//! Whether `entry` names an existing file needs the directory, so the extension loader decides it.

use std::path::Path;

use crate::types::api::{ApiCheck, check};
use crate::types::extension::EntryPath;
use crate::types::extension::error::ExtensionError;
use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::manifest::PackVersion;
use crate::types::manifest::VersionRange;
use crate::types::name::ToolName;
use crate::types::project::InvalidSubPath;
use crate::types::project::SubPath;
use crate::types::report::Diagnostics;
use crate::types::report::FieldError;
use crate::types::report::FormatError;
use crate::types::report::LegacyApi;
use crate::types::report::ListedTwice;
use crate::types::report::MustBeRelative;
use crate::types::report::OutsideProject;
use crate::types::report::UnsupportedApi;

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
///     api = 1\n\
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
    reads: Vec<SubPath>,
    requires: ExtensionRequires,
}

/// The `[requires]` table of an extension manifest, which holds only the agentstacks range.
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
///     version = \"1.0.0\"\n\
///     api = 1\n\
///     description = \"Renders packs into Claude Code project config\"\n\
///     entry = \"adapter.lua\"\n\
///     [requires]\n\
///     agentstacks = \">=1.0\"\n";
/// let manifest = ExtensionManifest::parse(Path::new("extension.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(
///     manifest.requires().agentstacks().map(|range| range.written()),
///     Some(">=1.0")
/// );
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionRequires {
    agentstacks: Option<VersionRange>,
}

impl ExtensionRequires {
    /// The agentstacks range the extension needs, when it declares one; absent means any version.
    #[must_use]
    pub const fn agentstacks(&self) -> Option<&VersionRange> {
        self.agentstacks.as_ref()
    }
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
        root.refuse_unknown_keys(&["extension", "requires"]);
        let read = root.table_or_empty("extension").map(|mut section| {
            let read = read_section(&mut section);
            root.absorb(section);
            read
        });
        let agentstacks = root.optional_table("requires").and_then(|mut table| {
            let range = VersionRange::read::<ExtensionError>(&mut table, "agentstacks");
            table.refuse_unknown_keys(&["agentstacks"]);
            root.absorb(table);
            range
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
                    reads,
                },
                true,
            ) => Ok(Self {
                name,
                version,
                description,
                entry,
                reads,
                requires: ExtensionRequires { agentstacks },
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

    /// What the extension requires; the same shape as a pack's
    /// [`Requires`](crate::types::Requires), holding only the agentstacks range.
    #[must_use]
    pub const fn requires(&self) -> &ExtensionRequires {
        &self.requires
    }

    /// The project files the extension may read, as written; empty when `reads` is absent.
    ///
    /// Compare entries by [`SubPath::normalized`], which makes `./a` and `a` one path.
    #[must_use]
    pub fn reads(&self) -> &[SubPath] {
        &self.reads
    }
}

/// The fields of `[extension]` that read cleanly; `api` is checked and not kept.
struct Section {
    name: Option<ToolName>,
    version: Option<PackVersion>,
    description: Option<String>,
    entry: Option<EntryPath>,
    reads: Vec<SubPath>,
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
    match section.required_integer("api").map(check) {
        Some(ApiCheck::Legacy) => section.refuse("api", ExtensionError::from(LegacyApi)),
        Some(ApiCheck::Unsupported(found)) => {
            section.refuse("api", ExtensionError::from(UnsupportedApi::new(found)));
        }
        Some(ApiCheck::Current) | None => {}
    }
    let description = section.required_text("description");
    let entry = section.required_text("entry").and_then(|text| {
        EntryPath::new(&text)
            .map_err(|error| section.refuse("entry", error))
            .ok()
    });
    let reads = read_reads(section);
    section.refuse_unknown_keys(&["name", "version", "api", "description", "entry", "reads"]);
    Section {
        name,
        version,
        description,
        entry,
        reads,
    }
}

/// Reads `reads`: the entries that pass [`check_read`] and do not repeat an earlier one after
/// normalising. Each refusal is located at its own index and the others are still reported.
fn read_reads(section: &mut TomlFields) -> Vec<SubPath> {
    let mut reads: Vec<SubPath> = Vec::new();
    for (index, text) in section.optional_string_list("reads").unwrap_or_default() {
        let at = section.item_path("reads", index);
        match check_read(&text) {
            Ok(path)
                if reads
                    .iter()
                    .any(|seen| seen.normalized() == path.normalized()) =>
            {
                section.refuse_path(&at, ExtensionError::from(ListedTwice::text(text)));
            }
            Ok(path) => reads.push(path),
            Err(error) => section.refuse_path(&at, error),
        }
    }
    reads
}

/// Checks one `reads` entry: relative, inside the project, and written with `/` only.
fn check_read(text: &str) -> Result<SubPath, FormatError> {
    if text.starts_with('/') {
        return Err(ExtensionError::from(MustBeRelative::new(text)).into());
    }
    let path = SubPath::new(text).map_err(|error| match error {
        InvalidSubPath::OutsideRepository => {
            FormatError::from(ExtensionError::from(OutsideProject::new(text)))
        }
        InvalidSubPath::Empty => FormatError::from(FieldError::Empty),
    })?;
    if text.contains('\\') {
        return Err(ExtensionError::Separator {
            text: text.to_owned(),
        }
        .into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::project::SubPath;
    use crate::types::report::Diagnostic;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "extension.toml";

    const EXAMPLE: &str = "[extension]\n\
        name        = \"claude\"\n\
        version     = \"0.1.0\"\n\
        api         = 1\n\
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

    /// The example with a `reads` line added to `[extension]`.
    fn with_reads(line: &str) -> String {
        with_line("entry", &format!("entry       = \"adapter.lua\"\n{line}"))
    }

    #[test]
    fn extension_manifest_reads_default_to_empty() -> TestResult {
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), EXAMPLE).map_err(|d| format!("{d:?}"))?;
        assert!(manifest.reads().is_empty());
        let manifest = ExtensionManifest::parse(Path::new(FILE), &with_reads("reads = []"))
            .map_err(|d| format!("{d:?}"))?;
        assert!(manifest.reads().is_empty());
        Ok(())
    }

    #[test]
    fn extension_manifest_parses_reads() -> TestResult {
        let text = with_reads("reads = [\".claude/settings.json\", \"./opencode.json\"]");
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), &text).map_err(|d| format!("{d:?}"))?;
        let written: Vec<&str> = manifest.reads().iter().map(SubPath::as_str).collect();
        assert_eq!(written, [".claude/settings.json", "./opencode.json"]);
        let normalized: Vec<String> = manifest.reads().iter().map(SubPath::normalized).collect();
        assert_eq!(normalized, [".claude/settings.json", "opencode.json"]);
        Ok(())
    }

    #[test]
    fn extension_manifest_refuses_reads_that_is_not_an_array() {
        assert_eq!(
            parse(&with_reads("reads = \"x\"")),
            ["extension.toml: extension.reads: expected array, got string"]
        );
    }

    #[test]
    fn extension_manifest_refuses_a_reads_item_that_is_not_a_string() {
        assert_eq!(
            parse(&with_reads("reads = [1]")),
            ["extension.toml: extension.reads[0]: expected string, got integer"]
        );
    }

    #[test]
    fn extension_manifest_refuses_an_absolute_reads_entry() {
        assert_eq!(
            parse(&with_reads("reads = [\"/x\"]")),
            ["extension.toml: extension.reads[0]: \"/x\" must be relative"]
        );
    }

    #[test]
    fn extension_manifest_refuses_a_reads_entry_outside_the_project() {
        assert_eq!(
            parse(&with_reads("reads = [\"../x\"]")),
            ["extension.toml: extension.reads[0]: \"../x\" must stay inside the project"]
        );
    }

    #[test]
    fn extension_manifest_refuses_a_blank_reads_entry() {
        for entry in ["", "  "] {
            assert_eq!(
                parse(&with_reads(&format!("reads = [\"{entry}\"]"))),
                ["extension.toml: extension.reads[0]: empty"],
                "{entry:?}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_a_reads_entry_with_a_backslash() {
        assert_eq!(
            parse(&with_reads("reads = ['a\\b']")),
            ["extension.toml: extension.reads[0]: \"a\\b\" must use / as the separator"]
        );
    }

    #[test]
    fn extension_manifest_allows_agentstacks_own_files_in_reads() -> TestResult {
        let text = with_reads("reads = [\"agentstacks.toml\", \"agentstacks.lock\"]");
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), &text).map_err(|d| format!("{d:?}"))?;
        let written: Vec<&str> = manifest.reads().iter().map(SubPath::as_str).collect();
        assert_eq!(written, ["agentstacks.toml", "agentstacks.lock"]);
        Ok(())
    }

    #[test]
    fn extension_manifest_refuses_a_reads_entry_listed_twice() {
        assert_eq!(
            parse(&with_reads(
                "reads = [\".claude/settings.json\", \".claude/settings.json\"]"
            )),
            ["extension.toml: extension.reads[1]: \".claude/settings.json\" listed twice"]
        );
    }

    #[test]
    fn extension_manifest_refuses_one_reads_path_written_two_ways() {
        let cases = [
            ("[\".claude/x\", \"./.claude/x\"]", "\"./.claude/x\""),
            ("[\"./a\", \"a\"]", "\"a\""),
            ("[\"a//b\", \"a/b\"]", "\"a/b\""),
        ];
        for (list, repeated) in cases {
            assert_eq!(
                parse(&with_reads(&format!("reads = {list}"))),
                [format!(
                    "extension.toml: extension.reads[1]: {repeated} listed twice"
                )],
                "{list}"
            );
        }
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
    fn extension_manifest_refuses_api_other_than_one() {
        for api in [2, -1] {
            assert_eq!(
                parse(&with_line("api", &format!("api = {api}"))),
                [format!(
                    "extension.toml: extension.api: {api} not supported (this agentstacks speaks 1)"
                )],
                "{api}"
            );
        }
    }

    #[test]
    fn extension_manifest_refuses_api_zero_as_legacy() {
        assert_eq!(
            parse(&with_line("api", "api = 0")),
            [
                "extension.toml: extension.api: 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)"
            ]
        );
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
            api = 2\n\
            description = \"d\"\n\
            entry = \"adapter.js\"\n";
        let mut shown = parse(text);
        shown.sort();
        let mut expected = vec![
            "extension.toml: foo: unknown key".to_owned(),
            "extension.toml: extension.name: \"Claude\" is not a valid name".to_owned(),
            "extension.toml: extension.version: \"1.2\" is not semver".to_owned(),
            "extension.toml: extension.api: 2 not supported (this agentstacks speaks 1)".to_owned(),
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

    const REQUIRES_FILE: &str = "ext/mytool/extension.toml";

    fn parse_requires(text: &str) -> Vec<String> {
        shown(ExtensionManifest::parse(Path::new(REQUIRES_FILE), text))
    }

    #[test]
    fn documented_examples_parse() -> TestResult {
        let text = "[extension]\n\
            name        = \"mytool\"\n\
            version     = \"1.0.0\"\n\
            api         = 1\n\
            description = \"Renders packs into mytool project config\"\n\
            entry       = \"adapter.lua\"\n\
            \n\
            [requires]\n\
            agentstacks = \">=1.0\"\n";
        let manifest = ExtensionManifest::parse(Path::new(REQUIRES_FILE), text)
            .map_err(|d| format!("{d:?}"))?;
        assert_eq!(
            manifest.requires().agentstacks().map(ToString::to_string),
            Some(">=1.0".to_owned())
        );
        Ok(())
    }

    #[test]
    fn extension_requires_absent_accepts_any() -> TestResult {
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), EXAMPLE).map_err(|d| format!("{d:?}"))?;
        assert!(manifest.requires().agentstacks().is_none());
        let with_empty = format!("{EXAMPLE}[requires]\n");
        let manifest =
            ExtensionManifest::parse(Path::new(FILE), &with_empty).map_err(|d| format!("{d:?}"))?;
        assert!(manifest.requires().agentstacks().is_none());
        Ok(())
    }

    #[test]
    fn extension_requires_unknown_key() {
        assert_eq!(
            parse_requires(&format!("{EXAMPLE}[requires]\nfoo = 1\n")),
            ["ext/mytool/extension.toml: requires.foo: unknown key"]
        );
    }

    #[test]
    fn extension_requires_not_a_table() {
        assert_eq!(
            parse_requires(&format!("requires = \"x\"\n{EXAMPLE}")),
            ["ext/mytool/extension.toml: requires: expected table, got string"]
        );
    }

    #[test]
    fn extension_requires_refuses_a_bad_agentstacks_range() {
        for text in ["", "   ", "nope"] {
            assert_eq!(
                parse_requires(&format!("{EXAMPLE}[requires]\nagentstacks = \"{text}\"\n")),
                [format!(
                    "ext/mytool/extension.toml: requires.agentstacks: \"{text}\" is not a version range"
                )],
                "{text:?}"
            );
        }
        assert_eq!(
            parse_requires(&format!("{EXAMPLE}[requires]\nagentstacks = 1\n")),
            ["ext/mytool/extension.toml: requires.agentstacks: expected string, got integer"]
        );
    }

    #[test]
    fn extension_requires_bad_range_is_an_extension_error() -> TestResult {
        let text = format!("{EXAMPLE}[requires]\nagentstacks = \"nope\"\n");
        let refused = ExtensionManifest::parse(Path::new(FILE), &text)
            .err()
            .ok_or("a bad range is refused")?
            .into_sorted();
        assert!(matches!(
            refused.first().map(Diagnostic::error),
            Some(FormatError::Extension(ExtensionError::Range(_)))
        ));
        Ok(())
    }

    #[test]
    fn extension_requires_range_is_one_refusal_among_others() {
        let text = format!(
            "{}\n[requires]\nagentstacks = \"nope\"\n",
            with_line("name", "name = \"Claude\"")
        );
        assert_eq!(
            parse_requires(&text),
            [
                "ext/mytool/extension.toml: extension.name: \"Claude\" is not a valid name",
                "ext/mytool/extension.toml: requires.agentstacks: \"nope\" is not a version range",
            ]
        );
    }
}
