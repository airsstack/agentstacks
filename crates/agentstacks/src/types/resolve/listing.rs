//! Reading the records `git ls-tree -z` prints, and deciding from them.
//!
//! It is separate from the tag reader because a tree listing keeps raw path bytes, where tag
//! lines are text, and because the subdir decisions are made over these records.
//!
//! # Responsibilities
//!
//! - [`TreeRecord`] is one `<mode> <type> <oid> TAB <path>` record. [`TreeRecord::kind`] maps its
//!   mode to an [`EntryKind`], and [`TreeRecord::relative`] gives its path below the `subdir`.
//! - [`parse_ls_tree`] reads NUL-separated output into records in git's byte order; the path stays
//!   raw bytes, because `-z` prints it unquoted and the UTF-8 refusal belongs to the tree walk.
//!   [`parse_ls_tree_at`] locates its refusal at an entry's key.
//! - [`check_subdir`] refuses a `subdir` that listed nothing or named a file; [`check_subdir_at`]
//!   locates the refusal at the entry's `subdir`.
//! - [`root_label`] is the label a git source's tree reports as its root.
//!
//! # Non-responsibilities
//!
//! Running git and loading the listed blobs belong to the git adapter. Whether a path is valid
//! UTF-8, or a symlink is allowed, is decided by the tree walk.

use std::path::PathBuf;

use crate::port::EntryKind;
use crate::types::project::Pin;
use crate::types::project::SubPath;
use crate::types::report::Diagnostic;
use crate::types::report::Quoted;
use crate::types::resolve::FetchError;
use crate::types::resolve::GitStep;
use crate::types::resolve::SourceKey;
use crate::types::resolve::unreadable;

/// One record of a `git ls-tree -z` listing.
///
/// # Examples
///
/// ```
/// use agentstacks::port::EntryKind;
/// use agentstacks::types::resolve::parse_ls_tree;
///
/// let oid = "136c05e0d0290e2dc7eb89bceaa27b5f011e780a";
/// let stdout = format!("100755 blob {oid}\tpacks/core/run.sh\0");
/// let records = parse_ls_tree(stdout.as_bytes())?;
/// assert_eq!(records[0].kind(), EntryKind::File);
/// assert_eq!(records[0].path, b"packs/core/run.sh");
/// # Ok::<(), agentstacks::types::resolve::FetchError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRecord {
    /// The file mode, such as `100644`.
    pub mode: String,
    /// The object type: `blob`, `tree` or `commit`.
    pub object: String,
    /// The object id.
    pub oid: String,
    /// The path, as the raw bytes git printed.
    pub path: Vec<u8>,
}

impl TreeRecord {
    /// What the record's mode stands for: `100644` and `100755` are files, `120000` is a symlink,
    /// and any other mode (a gitlink, a tree) is [`EntryKind::Other`].
    #[must_use]
    pub fn kind(&self) -> EntryKind {
        match self.mode.as_str() {
            "100644" | "100755" => EntryKind::File,
            "120000" => EntryKind::Symlink,
            _ => EntryKind::Other,
        }
    }

    /// The record's path with `<normalised subdir>/` removed.
    ///
    /// Without a `subdir` the listing covers the whole tree and the whole path is returned. `None`
    /// means the path is not below the `subdir`.
    #[must_use]
    pub fn relative(&self, subdir: Option<&SubPath>) -> Option<Vec<u8>> {
        let normalized = subdir.map(SubPath::normalized).unwrap_or_default();
        if normalized.is_empty() {
            return Some(self.path.clone());
        }
        let prefix = format!("{normalized}/");
        self.path
            .strip_prefix(prefix.as_bytes())
            .map(<[u8]>::to_vec)
    }
}

/// One record, or `None` when it has another shape than `<mode> SP <type> SP <oid> TAB <path>`.
fn parse_record(record: &[u8]) -> Option<TreeRecord> {
    let tab = record.iter().position(|byte| *byte == b'\t')?;
    let (header, path) = (&record[..tab], &record[tab + 1..]);
    if path.is_empty() {
        return None;
    }
    let header = std::str::from_utf8(header).ok()?;
    let mut fields = header.split(' ');
    let (mode, object, oid) = (fields.next()?, fields.next()?, fields.next()?);
    if fields.next().is_some() || mode.is_empty() || object.is_empty() || oid.is_empty() {
        return None;
    }
    Some(TreeRecord {
        mode: mode.to_owned(),
        object: object.to_owned(),
        oid: oid.to_owned(),
        path: path.to_vec(),
    })
}

/// Reads `stdout`, the output of `git ls-tree -z`, into records in output order. Empty output is
/// an empty list.
///
/// # Errors
///
/// Returns the unlocated [`FetchError::GitUnreadable`] for `ls-tree` when a record has another
/// shape.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::parse_ls_tree;
///
/// assert!(parse_ls_tree(b"")?.is_empty());
/// assert!(parse_ls_tree(b"no tab here\0").is_err());
/// # Ok::<(), agentstacks::types::resolve::FetchError>(())
/// ```
pub fn parse_ls_tree(stdout: &[u8]) -> Result<Vec<TreeRecord>, FetchError> {
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let body = stdout.strip_suffix(&[0]).unwrap_or(stdout);
    body.split(|byte| *byte == 0)
        .map(|record| parse_record(record).ok_or_else(|| unreadable(GitStep::LsTree, stdout)))
        .collect()
}

/// [`parse_ls_tree`], with its refusal located at the entry `key`.
///
/// # Errors
///
/// The same refusal as [`parse_ls_tree`], at [`SourceKey::location`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
/// use agentstacks::types::resolve::{SourceKey, parse_ls_tree_at};
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// let refusal = parse_ls_tree_at(&key, b"junk\0").err().ok_or("expected a refusal")?;
/// assert_eq!(
///     refusal.to_string(),
///     "agentstacks.toml: packs.core: git ls-tree printed output agentstacks cannot read: \"junk\"",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_ls_tree_at(key: &SourceKey, stdout: &[u8]) -> Result<Vec<TreeRecord>, Diagnostic> {
    parse_ls_tree(stdout).map_err(|error| Diagnostic::new(key.location(), error))
}

/// The label a git source's tree reports as its root, and its load messages start with: the
/// `subdir` as written, or `.` without one.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::SubPath;
/// use agentstacks::types::resolve::root_label;
///
/// assert_eq!(root_label(Some(&SubPath::new("./packs//core")?)), Path::new("./packs//core"));
/// assert_eq!(root_label(None), Path::new("."));
/// # Ok::<(), agentstacks::types::InvalidSubPath>(())
/// ```
#[must_use]
pub fn root_label(subdir: Option<&SubPath>) -> PathBuf {
    subdir.map_or_else(|| PathBuf::from("."), |path| PathBuf::from(path.as_str()))
}

/// The pin as the tag or revision text a refusal names.
fn pin_text(pin: &Pin) -> &str {
    match pin {
        Pin::Tag(tag) => tag.as_str(),
        Pin::Rev(rev) => rev.as_str(),
    }
}

/// Refuses a `subdir` whose listing holds no record, or only the `subdir` itself as a blob.
///
/// `at` is the pin the listing was taken at. The refusal carries no location; the flow that holds
/// the entry's key locates it with [`SourceKey::subdir_location`]. Call it only for a source that
/// has a `subdir`.
///
/// # Errors
///
/// Returns [`FetchError::SubdirNotFound`] for an empty listing and [`FetchError::SubdirIsFile`]
/// for a single blob record at the `subdir`'s path.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, Pin, SubPath};
/// use agentstacks::types::resolve::check_subdir;
///
/// let pin = Pin::Tag(GitTag::new("packs-v0.1.0")?);
/// let refusal = check_subdir(&[], &SubPath::new("packs/nope")?, &pin)
///     .err()
///     .ok_or("expected a refusal")?;
/// assert_eq!(refusal.to_string(), "\"packs/nope\" not found at packs-v0.1.0");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn check_subdir(records: &[TreeRecord], subdir: &SubPath, at: &Pin) -> Result<(), FetchError> {
    match records {
        [] => Err(FetchError::SubdirNotFound {
            subdir: Quoted::new(subdir.as_str()),
            at: pin_text(at).to_owned(),
        }),
        [only] if only.object == "blob" && only.path == subdir.normalized().as_bytes() => {
            Err(FetchError::SubdirIsFile {
                subdir: Quoted::new(subdir.as_str()),
            })
        }
        _ => Ok(()),
    }
}

/// [`check_subdir`], with its refusal located at the entry's `subdir`.
///
/// # Errors
///
/// The same refusals as [`check_subdir`], at [`SourceKey::subdir_location`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, PackName, Pin, SubPath};
/// use agentstacks::types::resolve::{SourceKey, check_subdir_at};
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// let pin = Pin::Tag(GitTag::new("packs-v0.1.0")?);
/// let refusal = check_subdir_at(&key, &[], &SubPath::new("packs/nope")?, &pin)
///     .err()
///     .ok_or("expected a refusal")?;
/// assert_eq!(
///     refusal.to_string(),
///     "agentstacks.toml: packs.core.subdir: \"packs/nope\" not found at packs-v0.1.0",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn check_subdir_at(
    key: &SourceKey,
    records: &[TreeRecord],
    subdir: &SubPath,
    at: &Pin,
) -> Result<(), Diagnostic> {
    check_subdir(records, subdir, at).map_err(|error| Diagnostic::new(key.subdir_location(), error))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::name::PackName;
    use crate::types::project::CommitId;
    use crate::types::project::GitTag;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The records of G4's listing of `packs/core`, as `ls-tree -r -z` printed them.
    const G4: [(&str, &[u8]); 15] = [
        (
            "100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a",
            b"packs/core/.hidden",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/Z.md",
        ),
        (
            "100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c",
            b"packs/core/crlf-added.md",
        ),
        (
            "100644 blob c30dea8a3641ea99b125d04d599d843712292759",
            b"packs/core/crlf.md",
        ),
        (
            "100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c",
            b"packs/core/lf.md",
        ),
        (
            "120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9",
            b"packs/core/link",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/nested-x.md",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/nested.md",
        ),
        (
            "100644 blob 4cdb2265d30204be5463b38174b2e8e717982405",
            b"packs/core/nested/deeper/file.md",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/nested0.md",
        ),
        (
            "100644 blob d0a85717332d884c11c371725ba542b28ef47106",
            b"packs/core/pack.toml",
        ),
        (
            "100755 blob 4163036efa65bd4a469e752267498f01ea36a55c",
            b"packs/core/run.sh",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/sp ace.md",
        ),
        (
            "160000 commit 0123456789abcdef0123456789abcdef01234567",
            b"packs/core/sub",
        ),
        (
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/\xc3\xbc.md",
        ),
    ];

    /// `rows` as `ls-tree -z` prints them: `<header> TAB <path> NUL` each.
    fn stdout(rows: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (header, path) in rows {
            out.extend_from_slice(header.as_bytes());
            out.push(b'\t');
            out.extend_from_slice(path);
            out.push(0);
        }
        out
    }

    fn core() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("core")?))
    }

    fn tag_pin() -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Tag(GitTag::new("packs-v0.1.0")?))
    }

    #[test]
    fn ls_tree_records_parse() -> TestResult {
        let records = parse_ls_tree(&stdout(&G4))?;
        assert_eq!(records.len(), 15);
        for (record, (header, path)) in records.iter().zip(G4) {
            let fields: Vec<&str> = header.split(' ').collect();
            assert_eq!(
                (
                    record.mode.as_str(),
                    record.object.as_str(),
                    record.oid.as_str(),
                    record.path.as_slice()
                ),
                (fields[0], fields[1], fields[2], path)
            );
        }
        Ok(())
    }

    #[test]
    fn ls_tree_mode_mapping() -> TestResult {
        let rows: [(&str, &[u8]); 5] = [
            (
                "100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a",
                b"packs/core/a",
            ),
            (
                "100755 blob 4163036efa65bd4a469e752267498f01ea36a55c",
                b"packs/core/run.sh",
            ),
            (
                "120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9",
                b"packs/core/link",
            ),
            (
                "160000 commit 0123456789abcdef0123456789abcdef01234567",
                b"packs/core/sub",
            ),
            (
                "040000 tree 3519f5a9cbcb8b44e70ffa08716fa5711a7c9ec9",
                b"packs/core",
            ),
        ];
        let kinds: Vec<EntryKind> = parse_ls_tree(&stdout(&rows))?
            .iter()
            .map(TreeRecord::kind)
            .collect();
        assert_eq!(
            kinds,
            [
                EntryKind::File,
                EntryKind::File,
                EntryKind::Symlink,
                EntryKind::Other,
                EntryKind::Other,
            ]
        );
        Ok(())
    }

    #[test]
    fn ls_tree_path_keeps_raw_bytes() -> TestResult {
        let records = parse_ls_tree(&stdout(&G4))?;
        let umlaut = records.last().ok_or("no records")?;
        assert_eq!(
            umlaut.path,
            [b"packs/core/".as_slice(), &[0xC3, 0xBC], b".md"].concat()
        );
        let spaced = records
            .iter()
            .find(|record| record.path.ends_with(b"sp ace.md"))
            .ok_or("no spaced record")?;
        assert_eq!(spaced.path, b"packs/core/sp ace.md");

        let odd = stdout(&[(
            "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb",
            b"packs/core/\xff.md",
        )]);
        assert_eq!(parse_ls_tree(&odd)?[0].path, b"packs/core/\xff.md");
        Ok(())
    }

    #[test]
    fn ls_tree_malformed_record_is_unreadable() -> TestResult {
        let output = b"100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb packs/core/a\0";
        let error = parse_ls_tree(output).err().ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::GitUnreadable { .. }));
        let record = "100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb packs/core/a";
        assert_eq!(
            error.to_string(),
            format!("git ls-tree printed output agentstacks cannot read: \"{record}\"")
        );
        let located = parse_ls_tree_at(&core()?, output)
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            located.to_string(),
            format!(
                "agentstacks.toml: packs.core: git ls-tree printed output agentstacks cannot read: \"{record}\""
            )
        );
        for bad in [
            b"100644 blob\tpath\0".as_slice(),
            b"100644 blob abc extra\tpath\0",
            b"100644 blob abc\t\0",
            b"\0",
        ] {
            assert!(parse_ls_tree(bad).is_err(), "{bad:?}");
        }
        Ok(())
    }

    #[test]
    fn ls_tree_empty_output_is_empty() -> TestResult {
        assert!(parse_ls_tree(b"")?.is_empty());
        Ok(())
    }

    #[test]
    fn record_relative_strips_the_subdir() -> TestResult {
        let rows: [(&str, &[u8]); 1] = [(
            "100644 blob 4cdb2265d30204be5463b38174b2e8e717982405",
            b"packs/core/nested/deeper/file.md",
        )];
        let records = parse_ls_tree(&stdout(&rows))?;
        let record = &records[0];
        let subdir = SubPath::new("./packs//core")?;
        assert_eq!(
            record.relative(Some(&subdir)),
            Some(b"nested/deeper/file.md".to_vec())
        );
        assert_eq!(
            record.relative(None),
            Some(b"packs/core/nested/deeper/file.md".to_vec())
        );
        assert_eq!(record.relative(Some(&SubPath::new("other")?)), None);
        Ok(())
    }

    #[test]
    fn root_label_is_the_subdir_as_written_or_dot() -> TestResult {
        assert_eq!(
            root_label(Some(&SubPath::new("./packs//core")?)),
            Path::new("./packs//core")
        );
        assert_eq!(root_label(None), Path::new("."));
        Ok(())
    }

    #[test]
    fn subdir_listing_nothing_is_refused() -> TestResult {
        let subdir = SubPath::new("packs/nope")?;
        let error = check_subdir(&[], &subdir, &tag_pin()?)
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::SubdirNotFound { .. }));
        assert_eq!(
            error.to_string(),
            "\"packs/nope\" not found at packs-v0.1.0"
        );
        let located = check_subdir_at(&core()?, &[], &subdir, &tag_pin()?)
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            located.to_string(),
            "agentstacks.toml: packs.core.subdir: \"packs/nope\" not found at packs-v0.1.0"
        );

        let rev = "0123456789abcdef0123456789abcdef01234567";
        let by_rev = check_subdir(&[], &subdir, &Pin::Rev(CommitId::new(rev)?))
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            by_rev.to_string(),
            format!("\"packs/nope\" not found at {rev}")
        );
        Ok(())
    }

    #[test]
    fn subdir_naming_a_file_is_refused() -> TestResult {
        let rows: [(&str, &[u8]); 1] = [(
            "100644 blob d0a85717332d884c11c371725ba542b28ef47106",
            b"packs/core/pack.toml",
        )];
        let records = parse_ls_tree(&stdout(&rows))?;
        let subdir = SubPath::new("packs/core/pack.toml")?;
        let error = check_subdir(&records, &subdir, &tag_pin()?)
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(error, FetchError::SubdirIsFile { .. }));
        assert_eq!(
            error.to_string(),
            "\"packs/core/pack.toml\" is a file, not a directory"
        );
        let located = check_subdir_at(&core()?, &records, &subdir, &tag_pin()?)
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(
            located.to_string(),
            "agentstacks.toml: packs.core.subdir: \"packs/core/pack.toml\" is a file, not a directory"
        );
        Ok(())
    }

    #[test]
    fn subdir_directory_passes() -> TestResult {
        let records = parse_ls_tree(&stdout(&G4))?;
        let subdir = SubPath::new("packs/core")?;
        assert!(check_subdir(&records, &subdir, &tag_pin()?).is_ok());
        assert!(check_subdir_at(&core()?, &records, &subdir, &tag_pin()?).is_ok());
        Ok(())
    }
}
