//! The entries that record what an install wrote into the project: files and merges.
//!
//! They are separate from [`LockedEntry`](crate::types::LockedEntry) because they record what was
//! written, with a hash of the bytes and an owner, not what was resolved.
//!
//! # Responsibilities
//!
//! - [`FileEntry`] is a `[[file]]`: a project-relative [`SubPath`], the hash
//!   of the written bytes and the [`Owner`] that wrote it.
//! - [`MergeEntry`] is a `[[merge]]`: the file and key a merge touched, the hash of what it
//!   wrote there and its [`Owner`].
//! - Reading checks every key of one raw entry table in a single pass, so one bad field does not
//!   hide another; the whole-file reader in [`Lockfile`](crate::types::Lockfile) drives it.
//!
//! # Non-responsibilities
//!
//! Whether an owner is a locked pack or tool, and whether a path or merge repeats, belong to
//! [`Lockfile`](crate::types::Lockfile), which holds the whole list. A `by` that is not even a
//! name can never be locked, so it is refused here.

use crate::types::format::TomlFields;
use crate::types::lock::entry::locked::read_hash;
use crate::types::lock::error::LockError;
use crate::types::lock::merge::{MergeFile, MergeKey};
use crate::types::lock::owner::Owner;
use crate::types::project::InvalidSubPath;
use crate::types::project::SourceError;
use crate::types::project::SubPath;
use crate::types::report::FieldError;
use crate::types::report::MustBeRelative;
use crate::types::report::OutsideProject;
use crate::types::tree::ContentHash;

/// The keys a `[[file]]` entry may hold.
const FILE_KEYS: [&str; 3] = ["path", "hash", "by"];
/// The keys a `[[merge]]` entry may hold.
const MERGE_KEYS: [&str; 4] = ["file", "key", "hash", "by"];

/// One file an install wrote.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ContentHash, FileEntry, Owner, SubPath};
///
/// let hash = ContentHash::new(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
/// let entry = FileEntry::new(
///     SubPath::new(".claude/skills/discuss/SKILL.md")?,
///     hash.clone(),
///     Owner::new("core")?,
/// );
/// assert_eq!(entry.path().as_str(), ".claude/skills/discuss/SKILL.md");
/// assert_eq!(entry.hash(), &hash);
/// assert_eq!(entry.by().as_str(), "core");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    path: SubPath,
    hash: ContentHash,
    by: Owner,
}

impl FileEntry {
    /// An entry for the file at `path` with the hash of its bytes, written by `by`. The path is a
    /// [`SubPath`], so the entry is one that [`Lockfile::parse`](crate::types::Lockfile::parse)
    /// accepts when it is written out.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{ContentHash, FileEntry, Owner, SubPath};
    ///
    /// let hash = ContentHash::new(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
    /// let entry = FileEntry::new(SubPath::new("a/b.md")?, hash, Owner::new("core")?);
    /// assert_eq!(entry.path().normalized(), "a/b.md");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn new(path: SubPath, hash: ContentHash, by: Owner) -> Self {
        Self { path, hash, by }
    }

    /// The path, relative to the project.
    #[must_use]
    pub const fn path(&self) -> &SubPath {
        &self.path
    }

    /// The hash of the bytes written.
    #[must_use]
    pub const fn hash(&self) -> &ContentHash {
        &self.hash
    }

    /// The pack or tool that wrote the file.
    #[must_use]
    pub const fn by(&self) -> &Owner {
        &self.by
    }
}

/// What reading one raw `[[file]]` found.
pub(in crate::types::lock) struct ReadFile {
    /// The path, when it was valid, even if another field was refused.
    pub(in crate::types::lock) path: Option<SubPath>,
    /// The owner, when it was a valid name, even if another field was refused.
    pub(in crate::types::lock) by: Option<Owner>,
    /// The whole entry, when no field was refused.
    pub(in crate::types::lock) entry: Option<FileEntry>,
}

impl FileEntry {
    /// Reads the entry behind `fields`, refusing what is wrong in it.
    pub(in crate::types::lock) fn read(fields: &mut TomlFields) -> ReadFile {
        let path = read_path(fields);
        let hash = read_hash(fields, "hash");
        let by = read_owner(fields);
        fields.refuse_unknown_keys(&FILE_KEYS);

        let entry = match (path.clone(), hash, by.clone()) {
            (Some(path), Some(hash), Some(by)) => Some(Self { path, hash, by }),
            _ => None,
        };
        ReadFile { path, by, entry }
    }
}

/// One merge an install made into a file it does not own.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ContentHash, MergeEntry, MergeFile, MergeKey, Owner};
///
/// let hash = ContentHash::new(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
/// let entry = MergeEntry::new(
///     MergeFile::new(".claude/settings.json")?,
///     MergeKey::new("hooks")?,
///     hash.clone(),
///     Owner::new("core")?,
/// );
/// assert_eq!(entry.file().as_str(), ".claude/settings.json");
/// assert_eq!(entry.key().as_str(), "hooks");
/// assert_eq!(entry.hash(), &hash);
/// assert_eq!(entry.by().as_str(), "core");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeEntry {
    file: MergeFile,
    key: MergeKey,
    hash: ContentHash,
    by: Owner,
}

impl MergeEntry {
    /// An entry for the merge at `key` of `file` with the hash of what was written, made by `by`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{ContentHash, MergeEntry, MergeFile, MergeKey, Owner};
    ///
    /// let hash = ContentHash::new(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
    /// let entry = MergeEntry::new(
    ///     MergeFile::new("opencode.json")?,
    ///     MergeKey::new("plugin")?,
    ///     hash,
    ///     Owner::new("claude")?,
    /// );
    /// assert_eq!(entry.by().to_string(), "claude");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn new(file: MergeFile, key: MergeKey, hash: ContentHash, by: Owner) -> Self {
        Self {
            file,
            key,
            hash,
            by,
        }
    }

    /// The file the merge touched.
    #[must_use]
    pub const fn file(&self) -> &MergeFile {
        &self.file
    }

    /// The key the merge touched.
    #[must_use]
    pub const fn key(&self) -> &MergeKey {
        &self.key
    }

    /// The hash of what the merge wrote.
    #[must_use]
    pub const fn hash(&self) -> &ContentHash {
        &self.hash
    }

    /// The pack or tool that made the merge.
    #[must_use]
    pub const fn by(&self) -> &Owner {
        &self.by
    }
}

/// What reading one raw `[[merge]]` found.
pub(in crate::types::lock) struct ReadMerge {
    /// The file and key, when both were valid, even if another field was refused.
    pub(in crate::types::lock) target: Option<(MergeFile, MergeKey)>,
    /// The owner, when it was a valid name, even if another field was refused.
    pub(in crate::types::lock) by: Option<Owner>,
    /// The whole entry, when no field was refused.
    pub(in crate::types::lock) entry: Option<MergeEntry>,
}

impl MergeEntry {
    /// Reads the entry behind `fields`, refusing what is wrong in it.
    pub(in crate::types::lock) fn read(fields: &mut TomlFields) -> ReadMerge {
        let file = read_text(fields, "file", MergeFile::new);
        let key = read_text(fields, "key", MergeKey::new);
        let hash = read_hash(fields, "hash");
        let by = read_owner(fields);
        fields.refuse_unknown_keys(&MERGE_KEYS);

        let target = file.zip(key);
        let entry = match (target.clone(), hash, by.clone()) {
            (Some((file, key)), Some(hash), Some(by)) => Some(Self {
                file,
                key,
                hash,
                by,
            }),
            _ => None,
        };
        ReadMerge { target, by, entry }
    }
}

/// Reads `path`: a relative path with no `..` segment, so it stays inside the project.
fn read_path(fields: &mut TomlFields) -> Option<SubPath> {
    let text = fields.required_string("path")?;
    if text.starts_with('/') {
        fields.refuse("path", LockError::from(MustBeRelative::new(text)));
        return None;
    }
    match SubPath::new(&text) {
        Ok(path) => Some(path),
        Err(InvalidSubPath::OutsideRepository) => {
            fields.refuse("path", LockError::from(OutsideProject::new(text)));
            None
        }
        Err(error) => {
            fields.refuse("path", SourceError::from(error));
            None
        }
    }
}

/// Reads the string at `key` and builds its value with `build`, refusing what it refuses.
fn read_text<T>(
    fields: &mut TomlFields,
    key: &str,
    build: impl FnOnce(&str) -> Result<T, FieldError>,
) -> Option<T> {
    let text = fields.required_string(key)?;
    match build(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            fields.refuse(key, error);
            None
        }
    }
}

/// Reads `by`: a name, refused as an unknown owner when it cannot be one.
fn read_owner(fields: &mut TomlFields) -> Option<Owner> {
    let text = fields.required_string("by")?;
    Owner::new(&text)
        .map_err(|_| fields.refuse("by", LockError::UnknownOwner { text }))
        .ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const HASH_MESSAGE: &str = "bad hash (expected sha256:<64 lowercase hex>)";

    fn reader(key_path: &str, text: &str) -> Result<TomlFields, Box<dyn std::error::Error>> {
        let table: toml::Table = text.parse()?;
        Ok(TomlFields::new(
            Path::new("agentstacks.lock"),
            key_path,
            table,
        ))
    }

    fn shown(fields: TomlFields) -> Vec<String> {
        fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Reads a `[[file]]` at `file[0]`: the entry, or every refusal as displayed text.
    fn read_file(text: &str) -> Result<Result<FileEntry, Vec<String>>, Box<dyn std::error::Error>> {
        let mut fields = reader("file[0]", text)?;
        let read = FileEntry::read(&mut fields);
        let shown = shown(fields);
        Ok(match read.entry {
            Some(entry) if shown.is_empty() => Ok(entry),
            _ => Err(shown),
        })
    }

    /// Reads a `[[merge]]` at `merge[0]`.
    fn read_merge(
        text: &str,
    ) -> Result<Result<MergeEntry, Vec<String>>, Box<dyn std::error::Error>> {
        let mut fields = reader("merge[0]", text)?;
        let read = MergeEntry::read(&mut fields);
        let shown = shown(fields);
        Ok(match read.entry {
            Some(entry) if shown.is_empty() => Ok(entry),
            _ => Err(shown),
        })
    }

    fn file_text() -> String {
        format!(
            "path = \".claude/skills/discuss/SKILL.md\"\nhash = \"sha256:{HEX64}\"\nby = \"core\"\n"
        )
    }

    fn merge_text() -> String {
        format!(
            "file = \".claude/settings.json\"\nkey = \"<locator>\"\nhash = \"sha256:{HEX64}\"\nby = \"core\"\n"
        )
    }

    #[test]
    fn file_entry_parses_a_file_entry() -> TestResult {
        let entry = read_file(&file_text())?.map_err(|shown| shown.join("; "))?;
        assert_eq!(entry.path().as_str(), ".claude/skills/discuss/SKILL.md");
        assert_eq!(entry.hash().hex(), HEX64);
        assert_eq!(entry.by(), &Owner::new("core")?);
        Ok(())
    }

    #[test]
    fn file_entry_refuses_each_rule() -> TestResult {
        let cases: Vec<(String, String)> = vec![
            (
                file_text().replace("path = \".claude/skills/discuss/SKILL.md\"\n", ""),
                "agentstacks.lock: file[0].path: required".to_owned(),
            ),
            (
                file_text().replace(&format!("hash = \"sha256:{HEX64}\"\n"), ""),
                "agentstacks.lock: file[0].hash: required".to_owned(),
            ),
            (
                file_text().replace("by = \"core\"\n", ""),
                "agentstacks.lock: file[0].by: required".to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "/x"),
                "agentstacks.lock: file[0].path: \"/x\" must be relative".to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "../x"),
                "agentstacks.lock: file[0].path: \"../x\" must stay inside the project".to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "a/../b"),
                "agentstacks.lock: file[0].path: \"a/../b\" must stay inside the project"
                    .to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", ""),
                "agentstacks.lock: file[0].path: empty".to_owned(),
            ),
            (
                file_text().replace(HEX64, "XYZ"),
                format!("agentstacks.lock: file[0].hash: {HASH_MESSAGE}"),
            ),
            (
                file_text().replace("by = \"core\"", "by = \"Kore\""),
                "agentstacks.lock: file[0].by: \"Kore\" is not a locked pack or tool".to_owned(),
            ),
            (
                file_text().replace("path = \".claude/skills/discuss/SKILL.md\"", "path = 7"),
                "agentstacks.lock: file[0].path: expected string, got integer".to_owned(),
            ),
            (
                format!("{}mode = \"0644\"\n", file_text()),
                "agentstacks.lock: file[0].mode: unknown key".to_owned(),
            ),
        ];
        for (text, expected) in cases {
            let shown = read_file(&text)?.err().ok_or("expected a refusal")?;
            assert_eq!(shown, [expected], "{text}");
        }
        Ok(())
    }

    #[test]
    fn file_entry_new_keeps_its_parts() -> TestResult {
        let entry = FileEntry::new(
            SubPath::new(".claude/x")?,
            ContentHash::new(&format!("sha256:{HEX64}"))?,
            Owner::new("core")?,
        );
        assert_eq!(entry.path().as_str(), ".claude/x");
        assert_eq!(entry.by().as_str(), "core");
        Ok(())
    }

    #[test]
    fn merge_entry_parses_a_merge_entry() -> TestResult {
        let entry = read_merge(&merge_text())?.map_err(|shown| shown.join("; "))?;
        assert_eq!(entry.file().as_str(), ".claude/settings.json");
        assert_eq!(entry.key().as_str(), "<locator>");
        assert_eq!(entry.hash().hex(), HEX64);
        assert_eq!(entry.by(), &Owner::new("core")?);
        Ok(())
    }

    #[test]
    fn merge_entry_refuses_each_rule() -> TestResult {
        let mut cases: Vec<(String, String)> = Vec::new();
        for (line, field) in [
            ("file = \".claude/settings.json\"\n".to_owned(), "file"),
            ("key = \"<locator>\"\n".to_owned(), "key"),
            (format!("hash = \"sha256:{HEX64}\"\n"), "hash"),
            ("by = \"core\"\n".to_owned(), "by"),
        ] {
            cases.push((
                merge_text().replace(&line, ""),
                format!("agentstacks.lock: merge[0].{field}: required"),
            ));
        }
        for blank in ["", "  "] {
            cases.push((
                merge_text().replace("<locator>", blank),
                "agentstacks.lock: merge[0].key: empty".to_owned(),
            ));
            cases.push((
                merge_text().replace(".claude/settings.json", blank),
                "agentstacks.lock: merge[0].file: empty".to_owned(),
            ));
        }
        cases.push((
            merge_text().replace("file = \".claude/settings.json\"", "file = 3"),
            "agentstacks.lock: merge[0].file: expected string, got integer".to_owned(),
        ));
        cases.push((
            merge_text().replace(HEX64, "XYZ"),
            format!("agentstacks.lock: merge[0].hash: {HASH_MESSAGE}"),
        ));
        cases.push((
            merge_text().replace("by = \"core\"", "by = \"Core\""),
            "agentstacks.lock: merge[0].by: \"Core\" is not a locked pack or tool".to_owned(),
        ));
        cases.push((
            format!("{}path = \"x\"\n", merge_text()),
            "agentstacks.lock: merge[0].path: unknown key".to_owned(),
        ));
        for (text, expected) in cases {
            let shown = read_merge(&text)?.err().ok_or("expected a refusal")?;
            assert_eq!(shown, [expected], "{text}");
        }
        Ok(())
    }

    #[test]
    fn merge_entry_new_keeps_its_parts() -> TestResult {
        let entry = MergeEntry::new(
            MergeFile::new(".claude/settings.json")?,
            MergeKey::new("hooks")?,
            ContentHash::new(&format!("sha256:{HEX64}"))?,
            Owner::new("core")?,
        );
        assert_eq!(entry.file().as_str(), ".claude/settings.json");
        assert_eq!(entry.key().as_str(), "hooks");
        assert_eq!(entry.by().as_str(), "core");
        Ok(())
    }
}
