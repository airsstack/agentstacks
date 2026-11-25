//! The entry that records a file an install wrote into the project.
//!
//! It is separate from [`LockedEntry`](crate::types::LockedEntry) because it records what was
//! written, with a hash of the bytes and an owner, not what was resolved.
//!
//! # Responsibilities
//!
//! - [`FileEntry`] is a `[[file]]`: a project-relative [`SubPath`], the hash
//!   of the written bytes and the [`Owner`] that wrote it.
//! - Reading refuses a path that is absolute, climbs out of the project, contains a `\`, or names
//!   `agentstacks.toml` or `agentstacks.lock`: agentstacks never records its own files.
//! - Reading checks every key of one raw entry table in a single pass, so one bad field does not
//!   hide another; the whole-file reader in [`Lockfile`](crate::types::Lockfile) drives it.
//!
//! # Non-responsibilities
//!
//! Whether an owner is a locked pack or tool, and whether a path repeats, belong to
//! [`Lockfile`](crate::types::Lockfile), which holds the whole list. A `by` that is not even a
//! name can never be locked, so it is refused here.

use crate::types::format::TomlFields;
use crate::types::lock::entry::locked::read_hash;
use crate::types::lock::error::LockError;
use crate::types::lock::owner::Owner;
use crate::types::project::InvalidSubPath;
use crate::types::project::PROJECT_FILE;
use crate::types::project::SourceError;
use crate::types::project::SubPath;
use crate::types::report::MustBeRelative;
use crate::types::report::OutsideProject;
use crate::types::tree::ContentHash;

/// The files agentstacks writes itself, which no entry may record.
const OWN_FILES: [&str; 2] = ["agentstacks.lock", PROJECT_FILE];
/// The keys a `[[file]]` entry may hold.
const FILE_KEYS: [&str; 3] = ["path", "hash", "by"];

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

/// Reads `path`: a relative path with no `..` segment, so it stays inside the project, written
/// with `/` only and naming neither of agentstacks' own files.
fn read_path(fields: &mut TomlFields) -> Option<SubPath> {
    let text = fields.required_string("path")?;
    if text.starts_with('/') {
        fields.refuse("path", LockError::from(MustBeRelative::new(text)));
        return None;
    }
    let path = match SubPath::new(&text) {
        Ok(path) => path,
        Err(InvalidSubPath::OutsideRepository) => {
            fields.refuse("path", LockError::from(OutsideProject::new(text)));
            return None;
        }
        Err(error) => {
            fields.refuse("path", SourceError::from(error));
            return None;
        }
    };
    if text.contains('\\') {
        fields.refuse("path", LockError::Separator { text });
        return None;
    }
    if OWN_FILES.contains(&path.normalized().as_str()) {
        fields.refuse("path", LockError::Managed { text });
        return None;
    }
    Some(path)
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

    fn file_text() -> String {
        format!(
            "path = \".claude/skills/discuss/SKILL.md\"\nhash = \"sha256:{HEX64}\"\nby = \"core\"\n"
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
                file_text().replace(".claude/skills/discuss/SKILL.md", "a\\\\b"),
                "agentstacks.lock: file[0].path: \"a\\b\" must use / as the separator".to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "agentstacks.lock"),
                "agentstacks.lock: file[0].path: \"agentstacks.lock\" is managed by agentstacks"
                    .to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "agentstacks.toml"),
                "agentstacks.lock: file[0].path: \"agentstacks.toml\" is managed by agentstacks"
                    .to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "./agentstacks.toml"),
                "agentstacks.lock: file[0].path: \"./agentstacks.toml\" is managed by agentstacks"
                    .to_owned(),
            ),
            (
                file_text().replace(".claude/skills/discuss/SKILL.md", "./agentstacks.lock"),
                "agentstacks.lock: file[0].path: \"./agentstacks.lock\" is managed by agentstacks"
                    .to_owned(),
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
}
