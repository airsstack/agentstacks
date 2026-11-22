//! `agentstacks.lock`, parsed and checked.
//!
//! It is the one place the lock's whole-file rules live (repeats, owners, sorting), kept apart from
//! the per-entry readers in [`locked`](crate::types::lock::entry::locked) and
//! [`written`](crate::types::lock::entry::written) because those rules need every entry at
//! once.
//!
//! # Responsibilities
//!
//! - [`Lockfile::parse`] reads the whole file and returns the lock, or every refusal of the file
//!   at once: a bad entry never hides the others. A syntax error stops only this file and is its
//!   only diagnostic.
//! - [`Lockfile::new`] builds a lock from typed [`LockContents`] and applies the same whole-file
//!   rules. Every path in an entry is a [`SubPath`] or a
//!   [`LocalPath`](crate::types::LocalPath), which hold the climbing and emptiness rules from
//!   construction. `new` does not apply the rules the parse adds for a file path: a `\` and
//!   agentstacks' own files (`agentstacks.lock`, `agentstacks.toml`). The operation path rule keeps
//!   such a path out before a lock is built, so a lock built from accepted operations writes text
//!   that reads back.
//! - It reads `api`, `[airsl]`, `[[tool]]`, `[[pack]]` and `[[file]]`. Empty text is an empty
//!   table, so the fields it lacks are refused one by one; any other top-level key is an
//!   unknown key.
//! - It refuses a repeated tool, pack or file path, located at the repeated entry
//!   (`pack[2]`), and a `by` that names no locked pack or tool. File paths are compared lexically with empty and `.` segments removed, so `./a` and `a`
//!   are one path. An entry whose identifying field is valid still counts for these checks when another of
//!   its fields is refused, so a broken entry produces no follow-on refusals.
//! - It keeps every list sorted by byte order (tools and packs by name, files by path), so the text written back never needs sorting.
//!
//! # Non-responsibilities
//!
//! A missing lock is not an error: this reader takes text, and a caller with no file has none to
//! parse. Whether the lock agrees with `agentstacks.toml` and whether `[airsl]` is needed by the
//! locked packs belong to the code that holds those, not to this reader. Comparing a locked `tree`
//! or file hash with the hash of the content on disk belongs to the stage that plans and applies an
//! install, which has that content.

use std::collections::BTreeSet;
use std::path::Path;

use toml::Value;

use crate::types::api::API;
use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::lock::entry::key::EntryKey;
use crate::types::lock::entry::locked::LockedEntry;
use crate::types::lock::entry::written::FileEntry;
use crate::types::lock::error::LockError;
use crate::types::lock::owner::Owner;
use crate::types::manifest::PackVersion;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::project::SubPath;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::FieldError;
use crate::types::report::ListedTwice;
use crate::types::report::Location;
use crate::types::report::TypeWord;
use crate::types::report::UnsupportedApi;

/// The keys `agentstacks.lock` may hold at its top level.
const TOP_LEVEL_KEYS: [&str; 5] = ["api", "airsl", "tool", "pack", "file"];

/// The typed entries a lock is built from, in any order.
///
/// # Examples
///
/// ```
/// use agentstacks::types::LockContents;
///
/// // An empty set of contents records nothing: no airsl and no entries.
/// let contents = LockContents::default();
/// assert!(contents.airsl.is_none());
/// assert!(contents.tools.is_empty() && contents.packs.is_empty());
/// assert!(contents.files.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockContents {
    /// The airsl version the lock records, when it records one.
    pub airsl: Option<PackVersion>,
    /// The locked tools.
    pub tools: Vec<LockedEntry<ToolName>>,
    /// The locked packs.
    pub packs: Vec<LockedEntry<PackName>>,
    /// The files an install wrote.
    pub files: Vec<FileEntry>,
}

/// A parsed `agentstacks.lock`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{LockContents, Lockfile};
///
/// let lock = Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// assert!(lock.airsl().is_none());
/// assert!(lock.packs().is_empty());
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lockfile {
    airsl: Option<PackVersion>,
    tools: Vec<LockedEntry<ToolName>>,
    packs: Vec<LockedEntry<PackName>>,
    files: Vec<FileEntry>,
}

impl Lockfile {
    /// Parses `text`, the content of the lock at `file`.
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
    /// use agentstacks::types::{Lockfile, write_lock};
    ///
    /// let tree = "0123456789abcdef".repeat(4);
    /// let comment = "# Generated by agentstacks. Do not edit.";
    /// let text = format!(
    ///     "api = 0\n{comment}\n\
    ///      \n\
    ///      [[pack]]\n\
    ///      name    = \"team-lint\"\n\
    ///      version = \"0.1.0\"\n\
    ///      path    = \"packs/team-lint\"\n\
    ///      tree    = \"sha256:{tree}\"\n"
    /// );
    /// let lock = Lockfile::parse(Path::new("agentstacks.lock"), &text)
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    /// assert_eq!(lock.packs().len(), 1);
    /// assert!(lock.airsl().is_none());
    ///
    /// // Writing the lock back gives the same text.
    /// assert_eq!(write_lock(&lock), text);
    ///
    /// // A file without an `api` is refused.
    /// assert!(Lockfile::parse(Path::new("agentstacks.lock"), "").is_err());
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
        if let Some(api) = root.required_integer("api").filter(|api| *api != API) {
            root.refuse_at_file(LockError::from(UnsupportedApi::new(api)));
        }
        let airsl = read_airsl(&mut root);
        let tools = read_array(&mut root, file, "tool", |entry, path| {
            LockedEntry::read(entry, path, ToolName::new)
        });
        let packs = read_array(&mut root, file, "pack", |entry, path| {
            LockedEntry::read(entry, path, PackName::new)
        });
        let files = read_array(&mut root, file, "file", |entry, _| FileEntry::read(entry));

        let mut diagnostics = root.into_diagnostics();
        Identities {
            tools: tools
                .iter()
                .filter_map(|(path, read)| Some((path.clone(), read.name.as_ref()?)))
                .collect(),
            packs: packs
                .iter()
                .filter_map(|(path, read)| Some((path.clone(), read.name.as_ref()?)))
                .collect(),
            files: files
                .iter()
                .filter_map(|(path, read)| Some((path.clone(), read.path.as_ref()?)))
                .collect(),
            owners: files
                .iter()
                .filter_map(|(path, read)| Some((by_path(path), read.by.as_ref()?)))
                .collect(),
        }
        .check(file, &mut diagnostics);
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        // Every entry that failed a field recorded a refusal, so with none recorded each read
        // holds its entry.
        let mut lock = Self {
            airsl,
            tools: tools
                .into_iter()
                .filter_map(|(_, read)| read.entry)
                .collect(),
            packs: packs
                .into_iter()
                .filter_map(|(_, read)| read.entry)
                .collect(),
            files: files
                .into_iter()
                .filter_map(|(_, read)| read.entry)
                .collect(),
        };
        lock.sort();
        Ok(lock)
    }

    /// Builds the lock `file` holds from typed `contents`, in any order.
    ///
    /// # Errors
    ///
    /// Returns every whole-file refusal, located in `file`: a repeated tool, pack or file path
    /// (located at the repeated entry, `pack[i]`), and a `by` that names no locked pack or tool
    /// (located at `file[i].by`). Each index is the entry's position in the
    /// sorted list the lock keeps and writes.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{
    ///     ContentHash, FileEntry, LocalPath, LockContents, LockedEntry, LockedSource, Lockfile,
    ///     Owner, PackName, PackVersion, SubPath,
    /// };
    ///
    /// let tree = ContentHash::new(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
    /// let core = LockedEntry::new(
    ///     PackName::new("core")?,
    ///     PackVersion::new("0.3.0")?,
    ///     LockedSource::Path(LocalPath::new("packs/core")?),
    ///     tree.clone(),
    /// );
    /// let file = FileEntry::new(
    ///     SubPath::new(".claude/skills/discuss/SKILL.md")?,
    ///     tree,
    ///     Owner::new("core")?,
    /// );
    /// let contents = LockContents {
    ///     packs: vec![core],
    ///     files: vec![file.clone()],
    ///     ..LockContents::default()
    /// };
    /// let lock = Lockfile::new(Path::new("agentstacks.lock"), contents)
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    /// assert_eq!(lock.files(), &[file.clone()]);
    ///
    /// // A file owned by a pack the lock does not hold is refused.
    /// let orphan = LockContents { files: vec![file], ..LockContents::default() };
    /// assert!(Lockfile::new(Path::new("agentstacks.lock"), orphan).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(file: &Path, contents: LockContents) -> Result<Self, Diagnostics> {
        let LockContents {
            airsl,
            tools,
            packs,
            files,
        } = contents;
        let mut lock = Self {
            airsl,
            tools,
            packs,
            files,
        };
        lock.sort();
        let mut diagnostics = Diagnostics::new();
        Identities {
            tools: indexed("tool", lock.tools.iter().map(LockedEntry::name)),
            packs: indexed("pack", lock.packs.iter().map(LockedEntry::name)),
            files: indexed("file", lock.files.iter().map(FileEntry::path)),
            owners: lock
                .files
                .iter()
                .enumerate()
                .map(|(index, entry)| (format!("file[{index}].by"), entry.by()))
                .collect(),
        }
        .check(file, &mut diagnostics);
        if diagnostics.is_empty() {
            Ok(lock)
        } else {
            Err(diagnostics)
        }
    }

    /// The airsl version the lock records, when it records one.
    #[must_use]
    pub const fn airsl(&self) -> Option<&PackVersion> {
        self.airsl.as_ref()
    }

    /// The locked tools, sorted by name.
    #[must_use]
    pub fn tools(&self) -> &[LockedEntry<ToolName>] {
        &self.tools
    }

    /// The locked packs, sorted by name.
    #[must_use]
    pub fn packs(&self) -> &[LockedEntry<PackName>] {
        &self.packs
    }

    /// The files an install wrote, sorted by path.
    #[must_use]
    pub fn files(&self) -> &[FileEntry] {
        &self.files
    }

    fn sort(&mut self) {
        self.tools.sort_by(|a, b| a.name().cmp(b.name()));
        self.packs.sort_by(|a, b| a.name().cmp(b.name()));
        self.files.sort_by(|a, b| a.path().cmp(b.path()));
    }
}

/// The identifying values of every entry, borrowed, for the checks that look across entries.
struct Identities<'a> {
    /// Each tool name with the key path of its entry (`tool[1]`).
    tools: Vec<(String, &'a ToolName)>,
    /// Each pack name with the key path of its entry.
    packs: Vec<(String, &'a PackName)>,
    /// Each file path with the key path of its entry.
    files: Vec<(String, &'a SubPath)>,
    /// Each `by` with the key path it was read at.
    owners: Vec<(String, &'a Owner)>,
}

impl Identities<'_> {
    /// Records in `diagnostics` every entry that repeats an earlier one, located at the repeated
    /// entry, and every `by` that names nothing locked.
    fn check(&self, file: &Path, diagnostics: &mut Diagnostics) {
        let at_entry = |path: &str, key: EntryKey| {
            Diagnostic::new(
                Location::at_key(file, path),
                LockError::from(ListedTwice::new(key)),
            )
        };
        for (path, name) in repeated(&self.tools, |name| name) {
            diagnostics.push(at_entry(path, EntryKey::Tool(name.clone())));
        }
        for (path, name) in repeated(&self.packs, |name| name) {
            diagnostics.push(at_entry(path, EntryKey::Pack(name.clone())));
        }
        for (path, written) in repeated(&self.files, SubPath::normalized) {
            diagnostics.push(at_entry(path, EntryKey::File(written.clone())));
        }

        let locked: BTreeSet<&str> = self
            .tools
            .iter()
            .map(|(_, name)| name.as_str())
            .chain(self.packs.iter().map(|(_, name)| name.as_str()))
            .collect();
        for (path, owner) in &self.owners {
            if !locked.contains(owner.as_str()) {
                diagnostics.push(Diagnostic::new(
                    Location::at_key(file, path.as_str()),
                    LockError::UnknownOwner {
                        text: owner.to_string(),
                    },
                ));
            }
        }
    }
}

/// The items whose `key` occurred before, each with its key path, in list order. Every later
/// occurrence is returned, so a value written three times is returned twice.
fn repeated<T: Copy, K: Ord>(items: &[(String, T)], key: impl Fn(T) -> K) -> Vec<(&str, T)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (path, item) in items {
        if !seen.insert(key(*item)) {
            out.push((path.as_str(), *item));
        }
    }
    out
}

/// Pairs each of `items` with its key path in the sorted list, `<key>[<index>]`.
fn indexed<T>(key: &str, items: impl Iterator<Item = T>) -> Vec<(String, T)> {
    items
        .enumerate()
        .map(|(index, item)| (format!("{key}[{index}]"), item))
        .collect()
}

/// The key path of the `by` of the entry at `entry_path`.
fn by_path(entry_path: &str) -> String {
    format!("{entry_path}.by")
}

/// Reads the optional `[airsl]` table: an exact version and nothing else.
fn read_airsl(root: &mut TomlFields) -> Option<PackVersion> {
    let mut airsl = root.optional_table("airsl")?;
    let version = airsl.required_string("version").and_then(|text| {
        PackVersion::new(&text)
            .map_err(|_| airsl.refuse("version", LockError::NotAVersion { text }))
            .ok()
    });
    airsl.refuse_unknown_keys(&["version"]);
    root.absorb(airsl);
    version
}

/// Reads every table of the array at `key` through `read`, with the key path of each entry
/// (`pack[1]`). A value that is not an array, or an item that is not a table, is refused and left
/// out, so an entry keeps the index it has in the text.
fn read_array<R>(
    root: &mut TomlFields,
    file: &Path,
    key: &str,
    read: impl Fn(&mut TomlFields, &str) -> R,
) -> Vec<(String, R)> {
    let items = match root.raw(key).cloned() {
        None => return Vec::new(),
        Some(Value::Array(items)) => items,
        Some(other) => {
            let got = TomlFields::type_word(&other);
            root.refuse(
                key,
                FieldError::WrongType {
                    expected: TypeWord::Array,
                    got,
                },
            );
            return Vec::new();
        }
    };
    let mut reads = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        let path = root.item_path(key, index);
        if let Value::Table(table) = item {
            let mut entry = TomlFields::new(file, &path, table);
            let result = read(&mut entry, &path);
            root.absorb(entry);
            reads.push((path, result));
        } else {
            root.refuse_path(
                &path,
                FieldError::WrongType {
                    expected: TypeWord::Table,
                    got: TomlFields::type_word(&item),
                },
            );
        }
    }
    reads
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::types::lock::testing::{HEX40, HEX64, example_lock};
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const LOCK_FILE: &str = "agentstacks.lock";

    fn parse(text: &str) -> Result<Lockfile, Vec<String>> {
        Lockfile::parse(Path::new(LOCK_FILE), text).map_err(|diagnostics| {
            diagnostics
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect()
        })
    }

    fn refusals(text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match parse(text) {
            Ok(lock) => Err(format!("expected a refusal, parsed {lock:?}").into()),
            Err(shown) => Ok(shown),
        }
    }

    fn refused_once(text: &str, message: &str) -> TestResult {
        assert_eq!(refusals(text)?, [message], "{text}");
        Ok(())
    }

    fn parsed(text: &str) -> Result<Lockfile, Box<dyn std::error::Error>> {
        parse(text).map_err(|shown| shown.join("; ").into())
    }

    fn tool(name: &str) -> String {
        format!(
            "[[tool]]\nname = \"{name}\"\nversion = \"0.1.0\"\ngit = \"https://example.org/r\"\n\
             rev = \"{HEX40}\"\ntree = \"sha256:{HEX64}\"\n"
        )
    }

    fn pack(name: &str) -> String {
        format!(
            "[[pack]]\nname = \"{name}\"\nversion = \"0.1.0\"\ngit = \"https://example.org/r\"\n\
             rev = \"{HEX40}\"\ntree = \"sha256:{HEX64}\"\n"
        )
    }

    fn file_entry(path: &str, by: &str) -> String {
        format!("[[file]]\npath = \"{path}\"\nhash = \"sha256:{HEX64}\"\nby = \"{by}\"\n")
    }

    #[test]
    fn lockfile_parses_the_documented_example() -> TestResult {
        let lock = parsed(&example_lock())?;
        assert_eq!(lock.airsl(), Some(&PackVersion::new("0.1.4")?));
        let tools: Vec<&str> = lock.tools().iter().map(|t| t.name().as_str()).collect();
        assert_eq!(tools, ["claude"]);
        let packs: Vec<&str> = lock.packs().iter().map(|p| p.name().as_str()).collect();
        assert_eq!(packs, ["core", "team-lint"]);
        let files: Vec<&str> = lock.files().iter().map(|f| f.path().as_str()).collect();
        assert_eq!(files, [".claude/skills/discuss/SKILL.md"]);
        assert_eq!(lock.files().first().map(|f| f.by().as_str()), Some("core"));
        Ok(())
    }

    #[test]
    fn lockfile_refuses_a_merge_table_as_an_unknown_key() -> TestResult {
        refused_once(
            &format!(
                "api = 0\n[[merge]]\nfile = \".claude/settings.json\"\nkey = \"hooks\"\n\
                 hash = \"sha256:{HEX64}\"\nby = \"core\"\n"
            ),
            "agentstacks.lock: merge: unknown key",
        )
    }

    #[test]
    fn lockfile_refuses_empty_text_field_by_field() -> TestResult {
        refused_once("", "agentstacks.lock: api: required")
    }

    #[test]
    fn lockfile_reports_a_syntax_error_alone() -> TestResult {
        refused_once(
            "api = 0\nx = [1,",
            "agentstacks.lock:2: unclosed array, expected `]`",
        )
    }

    #[test]
    fn lockfile_refuses_top_level_rules() -> TestResult {
        let cases = [
            (
                "api = 1\n",
                "agentstacks.lock: api 1 not supported (this agentstacks speaks 0)",
            ),
            (
                "api = \"0\"\n",
                "agentstacks.lock: api: expected integer, got string",
            ),
            (
                "api = 0.0\n",
                "agentstacks.lock: api: expected integer, got float",
            ),
            (
                "api = 0\n[airsl]\nversion = \"0.1\"\n",
                "agentstacks.lock: airsl.version: \"0.1\" is not a version",
            ),
            (
                "api = 0\n[airsl]\n",
                "agentstacks.lock: airsl.version: required",
            ),
            (
                "api = 0\n[airsl]\nversion = 1979-05-27\n",
                "agentstacks.lock: airsl.version: expected string, got datetime",
            ),
            (
                "api = 0\nairsl = \"0.1.4\"\n",
                "agentstacks.lock: airsl: expected table, got string",
            ),
            (
                "api = 0\n[airsl]\nversion = \"0.1.4\"\nchannel = \"x\"\n",
                "agentstacks.lock: airsl.channel: unknown key",
            ),
            (
                "api = 0\npack = \"core\"\n",
                "agentstacks.lock: pack: expected array, got string",
            ),
            (
                "api = 0\npack = [1]\n",
                "agentstacks.lock: pack[0]: expected table, got integer",
            ),
            ("api = 0\nfoo = 1\n", "agentstacks.lock: foo: unknown key"),
        ];
        for (text, message) in cases {
            refused_once(text, message)?;
        }
        Ok(())
    }

    #[test]
    fn lockfile_refuses_duplicates() -> TestResult {
        let head = "api = 0\n";
        let cases = [
            (
                format!("{head}{}{}", tool("claude"), tool("claude")),
                "agentstacks.lock: tool[1]: tool \"claude\" listed twice",
            ),
            (
                format!("{head}{}{}", pack("core"), pack("core")),
                "agentstacks.lock: pack[1]: pack \"core\" listed twice",
            ),
            (
                format!(
                    "{head}{}{}{}",
                    pack("core"),
                    file_entry(".claude/x", "core"),
                    file_entry(".claude/x", "core")
                ),
                "agentstacks.lock: file[1]: file \".claude/x\" listed twice",
            ),
        ];
        for (text, message) in &cases {
            refused_once(text, message)?;
        }

        let thrice = format!("{head}{}{}{}", pack("core"), pack("core"), pack("core"));
        assert_eq!(
            refusals(&thrice)?,
            [
                "agentstacks.lock: pack[1]: pack \"core\" listed twice",
                "agentstacks.lock: pack[2]: pack \"core\" listed twice",
            ]
        );

        let broken = pack("core").replace(HEX64, "XYZ");
        let text = format!("{head}{}{broken}", pack("core"));
        let shown: BTreeSet<String> = refusals(&text)?.into_iter().collect();
        let expected: BTreeSet<String> = [
            "agentstacks.lock: pack[1]: pack \"core\" listed twice",
            "agentstacks.lock: pack[1].tree: bad hash (expected sha256:<64 lowercase hex>)",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn lockfile_accepts_distinct_namespaces() -> TestResult {
        let head = "api = 0\n";
        let texts = [format!("{head}{}{}", tool("claude"), pack("claude"))];
        for text in &texts {
            parsed(text)?;
        }
        Ok(())
    }

    #[test]
    fn lockfile_checks_by_against_tools_and_packs() -> TestResult {
        let head = "api = 0\n";
        parsed(&format!(
            "{head}{}{}",
            tool("claude"),
            file_entry(".claude/x", "claude")
        ))?;

        let broken = pack("core").replace(HEX64, "XYZ");
        refused_once(
            &format!("{head}{broken}{}", file_entry(".claude/x", "core")),
            "agentstacks.lock: pack[0].tree: bad hash (expected sha256:<64 lowercase hex>)",
        )?;

        refused_once(
            &format!("{head}{}{}", pack("core"), file_entry(".claude/x", "kore")),
            "agentstacks.lock: file[0].by: \"kore\" is not a locked pack or tool",
        )
    }

    #[test]
    fn lockfile_reports_every_error_in_one_run() -> TestResult {
        let text = format!(
            "api = 1\n{}{}",
            pack("core").replace(&format!("rev = \"{HEX40}\"\n"), ""),
            file_entry("../x", "core")
        );
        let shown: BTreeSet<String> = refusals(&text)?.into_iter().collect();
        let expected: BTreeSet<String> = [
            "agentstacks.lock: api 1 not supported (this agentstacks speaks 0)",
            "agentstacks.lock: pack[0].rev: required",
            "agentstacks.lock: file[0].path: \"../x\" must stay inside the project",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn lockfile_sorts_parsed_entries() -> TestResult {
        let text = format!(
            "api = 0\n{}{}{}{}{}{}",
            pack("team-lint"),
            pack("core"),
            file_entry("b", "core"),
            file_entry("a", "core"),
            tool("codex"),
            tool("claude"),
        );
        let lock = parsed(&text)?;
        let packs: Vec<&str> = lock.packs().iter().map(|p| p.name().as_str()).collect();
        assert_eq!(packs, ["core", "team-lint"]);
        let tools: Vec<&str> = lock.tools().iter().map(|t| t.name().as_str()).collect();
        assert_eq!(tools, ["claude", "codex"]);
        let files: Vec<&str> = lock.files().iter().map(|f| f.path().as_str()).collect();
        assert_eq!(files, ["a", "b"]);
        Ok(())
    }

    #[test]
    fn lockfile_keeps_text_indices_in_diagnostics_after_a_non_table_item() -> TestResult {
        let text = "api = 0\npack = [1, { name = \"core\" }]\n";
        let shown: BTreeSet<String> = refusals(text)?.into_iter().collect();
        assert!(
            shown.contains("agentstacks.lock: pack[0]: expected table, got integer"),
            "{shown:?}"
        );
        assert!(
            shown.contains("agentstacks.lock: pack[1].version: required"),
            "{shown:?}"
        );
        Ok(())
    }

    fn new_lock(contents: LockContents) -> Result<Lockfile, Vec<String>> {
        Lockfile::new(Path::new(LOCK_FILE), contents).map_err(|diagnostics| {
            diagnostics
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect()
        })
    }

    fn owned_file(path: &str, by: &str) -> Result<FileEntry, Box<dyn std::error::Error>> {
        Ok(FileEntry::new(
            SubPath::new(path)?,
            ContentHash::new(&format!("sha256:{HEX64}"))?,
            Owner::new(by)?,
        ))
    }

    #[test]
    fn lockfile_new_applies_the_whole_file_rules() -> TestResult {
        let base = parsed(&format!(
            "api = 0\n{}{}{}{}",
            pack("team-lint"),
            pack("core"),
            file_entry("b", "core"),
            file_entry("a", "core"),
        ))?;
        let lock = new_lock(LockContents {
            packs: base.packs().iter().rev().cloned().collect(),
            files: base.files().iter().rev().cloned().collect(),
            ..LockContents::default()
        })
        .map_err(|shown| shown.join("; "))?;
        assert_eq!(lock, base);

        let core = base.packs().first().cloned().ok_or("core pack")?;
        let twice = new_lock(LockContents {
            packs: vec![core.clone(), core],
            ..LockContents::default()
        });
        assert_eq!(
            twice.err(),
            Some(vec![
                "agentstacks.lock: pack[1]: pack \"core\" listed twice".to_owned()
            ])
        );

        let unowned = new_lock(LockContents {
            files: vec![owned_file("a", "kore")?],
            ..LockContents::default()
        });
        assert_eq!(
            unowned.err(),
            Some(vec![
                "agentstacks.lock: file[0].by: \"kore\" is not a locked pack or tool".to_owned()
            ])
        );
        Ok(())
    }

    #[test]
    fn lockfile_new_locates_a_bad_by_at_its_sorted_position() -> TestResult {
        let base = parsed(&format!("api = 0\n{}", pack("core")))?;
        let unowned = new_lock(LockContents {
            packs: base.packs().to_vec(),
            files: vec![
                owned_file("b", "kore")?,
                owned_file("a", "core")?,
                owned_file("c", "core")?,
            ],
            ..LockContents::default()
        });
        assert_eq!(
            unowned.err(),
            Some(vec![
                "agentstacks.lock: file[1].by: \"kore\" is not a locked pack or tool".to_owned()
            ])
        );
        Ok(())
    }

    #[test]
    fn lockfile_new_locates_every_refusal_in_the_given_file() -> TestResult {
        let diagnostics = Lockfile::new(
            Path::new("/abs/agentstacks.lock"),
            LockContents {
                files: vec![owned_file("a", "kore")?],
                ..LockContents::default()
            },
        )
        .err()
        .ok_or("expected a refusal")?;
        let shown: Vec<String> = diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            shown,
            ["/abs/agentstacks.lock: file[0].by: \"kore\" is not a locked pack or tool"]
        );
        Ok(())
    }

    #[test]
    fn lockfile_refuses_one_path_written_two_ways() -> TestResult {
        let head = format!("api = 0\n{}", pack("core"));
        for (first, second) in [(".claude/x", "./.claude/x"), ("./a", "a"), ("a//b", "a/b")] {
            let text = format!(
                "{head}{}{}",
                file_entry(first, "core"),
                file_entry(second, "core")
            );
            let shown = refusals(&text)?;
            assert_eq!(shown.len(), 1, "{first} {second}: {shown:?}");
            assert!(
                shown
                    .iter()
                    .all(|message| message.ends_with("listed twice")),
                "{shown:?}"
            );
        }

        let base = parsed(&format!("{head}{}", file_entry("a", "core")))?;
        let by_new = new_lock(LockContents {
            packs: base.packs().to_vec(),
            files: vec![owned_file("a", "core")?, owned_file("./a", "core")?],
            ..LockContents::default()
        });
        assert_eq!(
            by_new.err(),
            Some(vec![
                "agentstacks.lock: file[1]: file \"a\" listed twice".to_owned()
            ])
        );
        Ok(())
    }
}
