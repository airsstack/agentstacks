//! `agentstacks.toml`, parsed and checked.
//!
//! It is the only reader of the project file; the source values it builds from live beside it
//! because the lock reads them too.
//!
//! # Responsibilities
//!
//! - [`ProjectFile::parse`] reads the whole file and returns the project, or every refusal of the
//!   file at once: a bad entry never hides the others. A syntax error stops only this file and is
//!   its only diagnostic.
//! - It reads `api`, `[tools.*]` and `[packs.*]`. An `api` of `0` is read and reported as
//!   [`ApiCheck::Legacy`] for the caller to act on; an `api` that is neither `0` nor the current
//!   one is refused as unsupported. Entry names follow the name rule; each entry is read through
//!   the source reader under the pin rule "exactly one of `tag` and `rev`"; a pack entry may also
//!   hold an `options` table, kept raw.
//! - A file must list at least one tool and one pack. A section that is missing or empty is
//!   reported as `nothing to install`; a section of the wrong type is reported for its type only.
//! - Empty text is an empty table, so the fields it lacks are refused one by one.
//!
//! # Non-responsibilities
//!
//! Rules that need fetched content (an entry's key matching the name inside its source, a pack
//! requiring a pack the file does not list, cycles, typing the options) belong to the checks over
//! the loaded packs.

use std::collections::BTreeMap;
use std::path::Path;

use toml::{Table, Value};

use crate::types::api::{self, ApiCheck};
use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::name::InvalidName;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::project::PackEntry;
use crate::types::project::PinRule;
use crate::types::project::Source;
use crate::types::project::SourceFields;
use crate::types::project::error::ProjectError;
use crate::types::report::Diagnostics;
use crate::types::report::FieldError;
use crate::types::report::LegacyProject;
use crate::types::report::TypeWord;

/// The name of the project file, the one place the name is written.
///
/// It is the path of the file under the project root and the file every refusal of the project is
/// located in.
///
/// # Examples
///
/// ```
/// use agentstacks::types::project::PROJECT_FILE;
///
/// assert_eq!(PROJECT_FILE, "agentstacks.toml");
/// ```
pub const PROJECT_FILE: &str = "agentstacks.toml";

/// The keys `agentstacks.toml` may hold at its top level.
const TOP_LEVEL_KEYS: [&str; 3] = ["api", "tools", "packs"];
/// The keys a `[tools.*]` entry may hold.
const TOOL_KEYS: [&str; 5] = ["git", "path", "tag", "rev", "subdir"];
/// The keys a `[packs.*]` entry may hold.
const PACK_KEYS: [&str; 6] = ["git", "path", "tag", "rev", "subdir", "options"];

/// A parsed `agentstacks.toml`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::ProjectFile;
///
/// let text = "api = 1\n\
///     [tools.claude]\n\
///     path = \"extensions/claude\"\n\
///     [packs.team-lint]\n\
///     path = \"packs/team-lint\"\n";
/// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(project.tools().len(), 1);
/// assert_eq!(project.packs().len(), 1);
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFile {
    api: ApiCheck,
    tools: BTreeMap<ToolName, Source>,
    packs: BTreeMap<PackName, PackEntry>,
}

impl ProjectFile {
    /// Parses `text`, the content of the project file at `file`.
    ///
    /// # Errors
    ///
    /// Returns every refusal of the file when any part is refused; the [`Diagnostics`] hand them
    /// back sorted.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{ProjectFile, Source};
    ///
    /// let text = "api = 1\n\
    ///     [tools.claude]\n\
    ///     git = \"https://github.com/airsstack/agentstacks\"\n\
    ///     tag = \"packs-v0.1.0\"\n\
    ///     subdir = \"extensions/claude\"\n\
    ///     [packs.team-lint]\n\
    ///     path = \"packs/team-lint\"\n";
    /// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    /// assert_eq!(project.tools().len(), 1);
    /// assert!(matches!(project.tools().values().next(), Some(Source::Git { .. })));
    /// assert!(matches!(
    ///     project.packs().values().next().map(|entry| entry.source()),
    ///     Some(Source::Path(_))
    /// ));
    ///
    /// // A file that lists nothing to install is refused.
    /// assert!(ProjectFile::parse(Path::new("agentstacks.toml"), "api = 1\n").is_err());
    /// # Ok::<(), String>(())
    /// ```
    pub fn parse(file: &Path, text: &str) -> Result<Self, Diagnostics> {
        let table = match parse_toml_document(file, text) {
            Ok(table) => table,
            Err(syntax) => {
                let mut diagnostics = Diagnostics::new();
                diagnostics.push(syntax);
                return Err(diagnostics);
            }
        };
        let mut root = TomlFields::new(file, "", table);
        root.refuse_unknown_keys(&TOP_LEVEL_KEYS);
        // A refused `api` makes the whole parse fail, so the stand-in is never returned.
        let api = api::read(&mut root, ProjectError::from).unwrap_or(ApiCheck::Current);
        let tools = read_section(&mut root, "tools", ToolName::new, read_tool);
        let packs = read_section(&mut root, "packs", PackName::new, read_pack);
        if tools.supplies_nothing() || packs.supplies_nothing() {
            root.refuse_at_file(ProjectError::NothingToInstall);
        }
        let diagnostics = root.into_diagnostics();
        if diagnostics.is_empty() {
            Ok(Self {
                api,
                tools: tools.entries,
                packs: packs.entries,
            })
        } else {
            Err(diagnostics)
        }
    }

    /// Which `api` the file declared: [`ApiCheck::Current`], or [`ApiCheck::Legacy`] for a file
    /// written for the `0.x` releases. Any other `api` is refused when the file is parsed, so it
    /// is never returned.
    #[must_use]
    pub const fn api(&self) -> ApiCheck {
        self.api
    }

    /// Refuses a file written for the `0.x` releases, which only `update` goes on with.
    ///
    /// # Errors
    ///
    /// [`ProjectError::LegacyProject`] when the file declares the legacy `api`.
    pub fn refuse_legacy(&self) -> Result<(), ProjectError> {
        match self.api {
            ApiCheck::Legacy => Err(ProjectError::from(LegacyProject)),
            ApiCheck::Current | ApiCheck::Unsupported(_) => Ok(()),
        }
    }

    /// The tools the project installs, by name.
    #[must_use]
    pub const fn tools(&self) -> &BTreeMap<ToolName, Source> {
        &self.tools
    }

    /// The packs the project installs, by name.
    #[must_use]
    pub const fn packs(&self) -> &BTreeMap<PackName, PackEntry> {
        &self.packs
    }
}

/// What one of `[tools]` and `[packs]` held.
struct Section<N, E> {
    /// The entries whose name and fields were all valid.
    entries: BTreeMap<N, E>,
    /// Whether the table has at least one entry.
    listed: bool,
    /// Whether the key is present but is not a table.
    mistyped: bool,
}

impl<N, E> Section<N, E> {
    /// Whether the file lists nothing here: the key is missing or its table is empty. A key that
    /// is present but not a table is refused for its type, so it is not also reported empty.
    const fn supplies_nothing(&self) -> bool {
        !self.listed && !self.mistyped
    }
}

/// Reads the table of entries at `key`, refusing every entry whose name `parse_name` refuses and
/// reading each entry's fields through `read_entry`.
fn read_section<N: Ord, E>(
    root: &mut TomlFields,
    key: &str,
    parse_name: fn(&str) -> Result<N, InvalidName>,
    read_entry: fn(&mut TomlFields, &str) -> Option<E>,
) -> Section<N, E> {
    let mut entries = BTreeMap::new();
    let Some(mut section) = root.optional_table(key) else {
        return Section {
            entries,
            listed: false,
            mistyped: root.raw(key).is_some(),
        };
    };
    let names = section.entries();
    for (name, _) in &names {
        let parsed = parse_name(name);
        if parsed.is_err() {
            section.refuse(name, ProjectError::InvalidName);
        }
        let path = section.key_path(name);
        let Some(mut entry) = section.optional_table(name) else {
            continue;
        };
        let read = read_entry(&mut entry, &path);
        section.absorb(entry);
        if let (Ok(name), Some(read)) = (parsed, read) {
            entries.insert(name, read);
        }
    }
    root.absorb(section);
    Section {
        entries,
        listed: !names.is_empty(),
        mistyped: false,
    }
}

/// Reads one `[tools.<name>]` entry at `path`.
fn read_tool(entry: &mut TomlFields, path: &str) -> Option<Source> {
    let source = SourceFields::read_into(entry, path, PinRule::ExactlyOneOfTagRev);
    entry.refuse_unknown_keys(&TOOL_KEYS);
    source.map(Source::from_fields)
}

/// Reads one `[packs.<name>]` entry at `path`, keeping `options` raw.
fn read_pack(entry: &mut TomlFields, path: &str) -> Option<PackEntry> {
    let source = SourceFields::read_into(entry, path, PinRule::ExactlyOneOfTagRev);
    let options = match entry.raw("options").cloned() {
        Some(Value::Table(options)) => Some(options),
        Some(other) => {
            let got = TomlFields::type_word(&other);
            entry.refuse(
                "options",
                FieldError::WrongType {
                    expected: TypeWord::Table,
                    got,
                },
            );
            None
        }
        None => Some(Table::new()),
    };
    entry.refuse_unknown_keys(&PACK_KEYS);
    Some(PackEntry::new(Source::from_fields(source?), options?))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::types::project::GitTag;
    use crate::types::project::GitUrl;
    use crate::types::project::LocalPath;
    use crate::types::project::Pin;
    use crate::types::project::SubPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const EXAMPLE: &str = "api = 1\n\
        \n\
        [tools.claude]\n\
        git    = \"https://github.com/airsstack/agentstacks\"\n\
        tag    = \"packs-v0.1.0\"\n\
        subdir = \"extensions/claude\"\n\
        \n\
        [packs.core]\n\
        git     = \"https://github.com/airsstack/agentstacks\"\n\
        tag     = \"packs-v0.1.0\"\n\
        subdir  = \"packs/core\"\n\
        options = { archive_keep = 50 }\n\
        \n\
        [packs.team-lint]\n\
        path = \"packs/team-lint\"\n";

    /// A file that is valid apart from what a test adds to it.
    const BASE: &str = "api = 1\n\
        [tools.claude]\n\
        git = \"https://example.org/r\"\n\
        tag = \"v1\"\n\
        [packs.core]\n\
        git = \"https://example.org/r\"\n\
        tag = \"v1\"\n";

    fn parse(text: &str) -> Result<ProjectFile, Vec<String>> {
        ProjectFile::parse(Path::new("agentstacks.toml"), text).map_err(|diagnostics| {
            diagnostics
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect()
        })
    }

    fn refusals(text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match parse(text) {
            Ok(project) => Err(format!("expected a refusal, parsed {project:?}").into()),
            Err(shown) => Ok(shown),
        }
    }

    fn refused_once(text: &str, message: &str) -> TestResult {
        assert_eq!(refusals(text)?, [message], "{text}");
        Ok(())
    }

    #[test]
    fn project_file_parses_the_documented_example() -> TestResult {
        let project = parse(EXAMPLE).map_err(|shown| shown.join("; "))?;
        let url = GitUrl::new("https://github.com/airsstack/agentstacks")?;
        let tag = Pin::Tag(GitTag::new("packs-v0.1.0")?);

        assert_eq!(
            project.tools().get(&ToolName::new("claude")?),
            Some(&Source::Git {
                url: url.clone(),
                pin: tag.clone(),
                subdir: Some(SubPath::new("extensions/claude")?),
            })
        );
        let core = project
            .packs()
            .get(&PackName::new("core")?)
            .ok_or("pack core")?;
        assert_eq!(
            core.source(),
            &Source::Git {
                url,
                pin: tag,
                subdir: Some(SubPath::new("packs/core")?),
            }
        );
        assert_eq!(
            core.options().get("archive_keep"),
            Some(&Value::Integer(50))
        );
        let team_lint = project
            .packs()
            .get(&PackName::new("team-lint")?)
            .ok_or("pack team-lint")?;
        assert_eq!(
            team_lint.source(),
            &Source::Path(LocalPath::new("packs/team-lint")?)
        );
        assert!(team_lint.options().is_empty());
        assert_eq!(project.tools().len(), 1);
        assert_eq!(project.packs().len(), 2);
        Ok(())
    }

    #[test]
    fn project_file_refuses_a_syntax_error_with_its_line() -> TestResult {
        refused_once(
            "api = 1\n\nx = [1,",
            "agentstacks.toml:3: unclosed array, expected `]`",
        )
    }

    #[test]
    fn project_file_refuses_api_rules() -> TestResult {
        let rest = BASE.trim_start_matches("api = 1\n");
        let cases = [
            (
                format!("api = 2\n{rest}"),
                "agentstacks.toml: api 2 not supported (this agentstacks speaks 1)",
            ),
            (rest.to_owned(), "agentstacks.toml: api: required"),
            (
                format!("api = \"0\"\n{rest}"),
                "agentstacks.toml: api: expected integer, got string",
            ),
            (
                format!("api = 1979-05-27\n{rest}"),
                "agentstacks.toml: api: expected integer, got datetime",
            ),
        ];
        for (text, message) in cases {
            refused_once(&text, message)?;
        }
        Ok(())
    }

    #[test]
    fn project_refuses_api_two() -> TestResult {
        let rest = BASE.trim_start_matches("api = 1\n");
        refused_once(
            &format!("api = 2\n{rest}"),
            "agentstacks.toml: api 2 not supported (this agentstacks speaks 1)",
        )?;
        refused_once(
            &format!("api = -1\n{rest}"),
            "agentstacks.toml: api -1 not supported (this agentstacks speaks 1)",
        )
    }

    #[test]
    fn project_reads_api_zero_as_legacy() -> TestResult {
        let rest = BASE.trim_start_matches("api = 1\n");
        let legacy = parse(&format!("api = 0\n{rest}")).map_err(|shown| shown.join("; "))?;
        assert_eq!(legacy.api(), ApiCheck::Legacy);
        let current = parse(&format!("api = 1\n{rest}")).map_err(|shown| shown.join("; "))?;
        assert_eq!(current.api(), ApiCheck::Current);
        assert_eq!(
            legacy.refuse_legacy().map_err(|error| error.to_string()),
            Err("api 0 is from agentstacks 0.x (run agentstacks update)".to_owned())
        );
        assert_eq!(current.refuse_legacy(), Ok(()));
        Ok(())
    }

    #[test]
    fn project_file_refuses_nothing_to_install() -> TestResult {
        let tool = "[tools.claude]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n";
        let pack = "[packs.core]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n";
        let cases = [
            format!("api = 1\n{tool}"),
            format!("api = 1\n{pack}"),
            "api = 1\n[tools]\n[packs]\n".to_owned(),
            "api = 1\n".to_owned(),
        ];
        for text in cases {
            refused_once(&text, "agentstacks.toml: nothing to install")?;
        }
        let shown: BTreeSet<String> = refusals("")?.into_iter().collect();
        let expected: BTreeSet<String> = [
            "agentstacks.toml: api: required",
            "agentstacks.toml: nothing to install",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn project_file_gives_only_the_wrong_type_for_a_mistyped_table_beside_a_listed_one()
    -> TestResult {
        refused_once(
            "api = 1\ntools = 1\n[packs.core]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n",
            "agentstacks.toml: tools: expected table, got integer",
        )?;
        refused_once(
            "api = 1\npacks = \"x\"\n[tools.claude]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n",
            "agentstacks.toml: packs: expected table, got string",
        )
    }

    #[test]
    fn project_file_reports_nothing_to_install_beside_a_mistyped_table() -> TestResult {
        let expected = |wrong_type: &str| -> BTreeSet<String> {
            [wrong_type, "agentstacks.toml: nothing to install"]
                .map(str::to_owned)
                .into()
        };
        let packs_only: BTreeSet<String> =
            refusals("api = 1\npacks = \"x\"\n")?.into_iter().collect();
        assert_eq!(
            packs_only,
            expected("agentstacks.toml: packs: expected table, got string")
        );
        let tools_only: BTreeSet<String> = refusals("api = 1\ntools = 1\n")?.into_iter().collect();
        assert_eq!(
            tools_only,
            expected("agentstacks.toml: tools: expected table, got integer")
        );
        let both_empty: BTreeSet<String> = refusals("api = 1\ntools = 1\n[packs]\n")?
            .into_iter()
            .collect();
        assert_eq!(
            both_empty,
            expected("agentstacks.toml: tools: expected table, got integer")
        );
        let both_mistyped: BTreeSet<String> = refusals("api = 1\ntools = 1\npacks = \"x\"\n")?
            .into_iter()
            .collect();
        let both_wrong: BTreeSet<String> = [
            "agentstacks.toml: tools: expected table, got integer",
            "agentstacks.toml: packs: expected table, got string",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(both_mistyped, both_wrong);
        Ok(())
    }

    #[test]
    fn project_file_accepts_the_project_root_as_a_pack_path() -> TestResult {
        let text = BASE.replace(
            "[packs.core]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n",
            "[packs.root]\npath = \".\"\n",
        );
        let project = parse(&text).map_err(|shown| shown.join("; "))?;
        let root = project
            .packs()
            .get(&PackName::new("root")?)
            .ok_or("pack root")?;
        assert_eq!(root.source(), &Source::Path(LocalPath::new(".")?));
        Ok(())
    }

    #[test]
    fn project_file_refuses_invalid_table_keys() -> TestResult {
        let pack = "git = \"https://example.org/r\"\ntag = \"v1\"\n";
        let tool = "[tools.claude]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n";
        refused_once(
            &format!("api = 1\n{tool}[packs.Core]\n{pack}"),
            "agentstacks.toml: packs.Core: not a valid name",
        )?;
        refused_once(
            &format!("api = 1\n[tools.Claude]\n{pack}[packs.core]\n{pack}"),
            "agentstacks.toml: tools.Claude: not a valid name",
        )
    }

    #[test]
    fn project_file_refuses_unknown_keys() -> TestResult {
        refused_once(
            &format!("{BASE}branch = \"main\"\n"),
            "agentstacks.toml: packs.core.branch: unknown key",
        )?;
        refused_once(
            &BASE.replace(
                "tag = \"v1\"\n[packs",
                "tag = \"v1\"\noptions = { x = 1 }\n[packs",
            ),
            "agentstacks.toml: tools.claude.options: unknown key",
        )?;
        refused_once(
            &format!("foo = 1\n{BASE}"),
            "agentstacks.toml: foo: unknown key",
        )
    }

    #[test]
    fn project_file_refuses_wrong_typed_tables() -> TestResult {
        refused_once(
            "api = 1\n[packs.core]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n\
             [tools]\nclaude = \"x\"\n",
            "agentstacks.toml: tools.claude: expected table, got string",
        )?;
        refused_once(
            &format!("{BASE}options = 5\n"),
            "agentstacks.toml: packs.core.options: expected table, got integer",
        )
    }

    #[test]
    fn project_file_applies_source_rules_to_every_entry() -> TestResult {
        let tool = "[tools.claude]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n";
        let pack = "[packs.core]\ngit = \"https://example.org/r\"\ntag = \"v1\"\n";
        let hex = "0123456789abcdef0123456789abcdef01234567";
        let cases = [
            (
                format!("api = 1\n{tool}{pack}path = \"packs/x\"\n"),
                "agentstacks.toml: packs.core: needs exactly one of git, path",
            ),
            (
                format!("api = 1\n{tool}{pack}rev = \"{hex}\"\n"),
                "agentstacks.toml: packs.core: needs exactly one of tag, rev",
            ),
            (
                format!(
                    "api = 1\n{tool}[packs.core]\ngit = \"https://example.org/r\"\nrev = \"main\"\n"
                ),
                "agentstacks.toml: packs.core.rev: not a commit id",
            ),
            (
                format!("api = 1\n{tool}[packs.team-lint]\npath = \"packs/x\"\ntag = \"v1\"\n"),
                "agentstacks.toml: packs.team-lint: tag, rev and subdir apply only to git sources",
            ),
            (
                format!("api = 1\n{tool}[packs.team-lint]\npath = \"/abs\"\n"),
                "agentstacks.toml: packs.team-lint.path: absolute paths are not portable",
            ),
            (
                format!("api = 1\n[tools.claude]\ngit = \"-x\"\ntag = \"v1\"\n{pack}"),
                "agentstacks.toml: tools.claude.git: URL must not start with \"-\"",
            ),
            (
                format!("api = 1\n{tool}subdir = \"/x\"\n{pack}"),
                "agentstacks.toml: tools.claude.subdir: must stay inside the repository",
            ),
        ];
        for (text, message) in cases {
            refused_once(&text, message)?;
        }
        Ok(())
    }

    #[test]
    fn project_file_reports_every_error() -> TestResult {
        let text = "api = 2\n\
            [tools.claude]\n\
            git = \"https://example.org/r\"\n\
            tag = \"v1\"\n\
            options = { x = 1 }\n\
            [packs.Core]\n\
            git = \"https://example.org/r\"\n\
            tag = \"v1\"\n\
            [packs.core]\n\
            git = \"https://example.org/r\"\n\
            tag = \"v1\"\n\
            branch = \"main\"\n";
        let shown: BTreeSet<String> = refusals(text)?.into_iter().collect();
        let expected: BTreeSet<String> = [
            "agentstacks.toml: api 2 not supported (this agentstacks speaks 1)",
            "agentstacks.toml: packs.Core: not a valid name",
            "agentstacks.toml: packs.core.branch: unknown key",
            "agentstacks.toml: tools.claude.options: unknown key",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn project_file_keeps_option_values_raw() -> TestResult {
        let text = format!("{BASE}options = {{ archive_keep = \"many\", ratio = 1.5 }}\n");
        let project = parse(&text).map_err(|shown| shown.join("; "))?;
        let core = project
            .packs()
            .get(&PackName::new("core")?)
            .ok_or("pack core")?;
        assert_eq!(
            core.options().get("archive_keep"),
            Some(&Value::String("many".to_owned()))
        );
        assert!(matches!(core.options().get("ratio"), Some(Value::Float(_))));
        Ok(())
    }
}
