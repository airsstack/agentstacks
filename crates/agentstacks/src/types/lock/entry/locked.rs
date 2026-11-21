//! One locked tool or pack: what an install resolved it to.
//!
//! It is separate from [`FileEntry`](crate::types::FileEntry) and
//! [`MergeEntry`](crate::types::MergeEntry) because it records what was resolved (a source and its
//! tree), where those two record what was written, and one generic entry serves tools and packs.
//!
//! # Responsibilities
//!
//! - [`LockedEntry`] is a `[[tool]]` (`N` is a [`ToolName`](crate::types::ToolName)) or a
//!   `[[pack]]` (`N` is a [`PackName`](crate::types::PackName)): name, version, where it came from
//!   and the hash of its tree.
//! - [`LockedSource`] is that origin. A git origin always holds the commit it resolved to and keeps
//!   the tag the project file used, if any; a path origin holds a project-relative directory. The
//!   origin has no tag-pinned form, and its paths are [`SubPath`] and
//!   [`LocalPath`], so what is built in memory is what a read lock holds.
//! - Reading checks every key of one raw entry table in a single pass, so one bad field does not
//!   hide another; the whole-file reader in [`Lockfile`](crate::types::Lockfile) drives it.
//!
//! # Non-responsibilities
//!
//! Whether the recorded tag agrees with `agentstacks.toml` is never checked here: a missing or
//! extra tag makes the lock stale, not malformed. Names that repeat across entries are refused by
//! [`Lockfile`](crate::types::Lockfile), which holds the whole list.
//! Whether the recorded `tree` matches the content of the source belongs to the stage that plans
//! and applies an install.

use crate::types::format::TomlFields;
use crate::types::lock::error::LockError;
use crate::types::manifest::PackVersion;
use crate::types::name::InvalidName;
use crate::types::project::CommitId;
use crate::types::project::GitTag;
use crate::types::project::GitUrl;
use crate::types::project::LocalPath;
use crate::types::project::Pin;
use crate::types::project::PinRule;
use crate::types::project::Source;
use crate::types::project::SourceFields;
use crate::types::project::SubPath;
use crate::types::tree::ContentHash;

/// The keys a `[[tool]]` or `[[pack]]` entry may hold.
const ENTRY_KEYS: [&str; 8] = [
    "name", "version", "git", "path", "tag", "rev", "subdir", "tree",
];

/// Where a locked entry came from.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{CommitId, GitUrl, LocalPath, LockedSource, Pin, Source};
///
/// let rev = CommitId::new("0123456789abcdef0123456789abcdef01234567")?;
/// let git = LockedSource::Git {
///     url: GitUrl::new("https://example.org/r")?,
///     tag: None,
///     subdir: None,
///     rev: rev.clone(),
/// };
/// // A locked git origin is always pinned by its commit.
/// assert!(matches!(git.to_source(), Source::Git { pin: Pin::Rev(pinned), .. } if pinned == rev));
///
/// let local = LockedSource::Path(LocalPath::new("packs/team-lint")?);
/// assert_eq!(local.to_source(), Source::Path(LocalPath::new("packs/team-lint")?));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockedSource {
    /// A git repository, resolved to a commit.
    Git {
        /// The repository URL.
        url: GitUrl,
        /// The tag `agentstacks.toml` pinned when the lock was written, kept as written.
        tag: Option<GitTag>,
        /// The directory inside the repository, when the entry is not its root.
        subdir: Option<SubPath>,
        /// The commit the source resolved to.
        rev: CommitId,
    },
    /// A directory inside the project.
    Path(LocalPath),
}

impl LockedSource {
    /// The source this origin stands for, pinned by its commit.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{CommitId, GitTag, GitUrl, LockedSource, Pin, Source};
    ///
    /// let rev = CommitId::new("0123456789abcdef0123456789abcdef01234567")?;
    /// let url = GitUrl::new("https://example.org/r")?;
    /// // The tag the project file used is not part of the source: the commit is the pin.
    /// let locked = LockedSource::Git {
    ///     url: url.clone(),
    ///     tag: Some(GitTag::new("v1")?),
    ///     subdir: None,
    ///     rev: rev.clone(),
    /// };
    /// assert_eq!(
    ///     locked.to_source(),
    ///     Source::Git { url, pin: Pin::Rev(rev), subdir: None }
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn to_source(&self) -> Source {
        match self {
            Self::Git {
                url, subdir, rev, ..
            } => Source::Git {
                url: url.clone(),
                pin: Pin::Rev(rev.clone()),
                subdir: subdir.clone(),
            },
            Self::Path(path) => Source::Path(path.clone()),
        }
    }
}

/// One locked tool or pack.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ContentHash, LocalPath, LockedEntry, LockedSource, PackName, PackVersion};
///
/// let tree = ContentHash::parse(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
/// let entry = LockedEntry::new(
///     PackName::new("team-lint")?,
///     PackVersion::new("0.1.0")?,
///     LockedSource::Path(LocalPath::new("packs/team-lint")?),
///     tree.clone(),
/// );
/// assert_eq!(entry.name().as_str(), "team-lint");
/// assert_eq!(entry.version().to_string(), "0.1.0");
/// assert_eq!(entry.tag(), None);
/// assert_eq!(entry.tree(), &tree);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedEntry<N> {
    name: N,
    version: PackVersion,
    source: LockedSource,
    tree: ContentHash,
}

impl<N> LockedEntry<N> {
    /// An entry for `name` at `version`, resolved to `source`, whose tree hashes to `tree`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{
    ///     CommitId, ContentHash, GitTag, GitUrl, LockedEntry, LockedSource, PackVersion, ToolName,
    /// };
    ///
    /// let tree = ContentHash::parse(&format!("sha256:{}", "0123456789abcdef".repeat(4)))?;
    /// let tag = GitTag::new("packs-v0.1.0")?;
    /// let entry = LockedEntry::new(
    ///     ToolName::new("claude")?,
    ///     PackVersion::new("0.1.0")?,
    ///     LockedSource::Git {
    ///         url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
    ///         tag: Some(tag.clone()),
    ///         subdir: None,
    ///         rev: CommitId::new("0123456789abcdef0123456789abcdef01234567")?,
    ///     },
    ///     tree,
    /// );
    /// assert_eq!(entry.tag(), Some(&tag));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn new(
        name: N,
        version: PackVersion,
        source: LockedSource,
        tree: ContentHash,
    ) -> Self {
        Self {
            name,
            version,
            source,
            tree,
        }
    }

    /// The entry's name.
    #[must_use]
    pub const fn name(&self) -> &N {
        &self.name
    }

    /// The version the pack or tool declared.
    #[must_use]
    pub const fn version(&self) -> &PackVersion {
        &self.version
    }

    /// Where the entry came from.
    #[must_use]
    pub const fn source(&self) -> &LockedSource {
        &self.source
    }

    /// The tag recorded beside the commit, when there is one.
    #[must_use]
    pub const fn tag(&self) -> Option<&GitTag> {
        match &self.source {
            LockedSource::Git { tag, .. } => tag.as_ref(),
            LockedSource::Path(_) => None,
        }
    }

    /// The hash of the entry's tree.
    #[must_use]
    pub const fn tree(&self) -> &ContentHash {
        &self.tree
    }
}

/// What reading one raw entry found: the name when it was valid, and the entry when every field
/// was.
pub(in crate::types::lock) struct ReadEntry<N> {
    /// The entry's name, when it was a valid name, even if another field was refused.
    pub(in crate::types::lock) name: Option<N>,
    /// The whole entry, when no field was refused.
    pub(in crate::types::lock) entry: Option<LockedEntry<N>>,
}

impl<N: Clone> LockedEntry<N> {
    /// Reads the entry behind `fields`, the table at `key_path`, refusing what is wrong in it.
    /// `name_rule` builds the name: a tool name or a pack name.
    pub(in crate::types::lock) fn read(
        fields: &mut TomlFields,
        key_path: &str,
        name_rule: fn(&str) -> Result<N, InvalidName>,
    ) -> ReadEntry<N> {
        let name = fields.required_string("name").and_then(|text| {
            name_rule(&text)
                .map_err(|error| fields.refuse("name", LockError::from(error)))
                .ok()
        });
        let version = fields.required_string("version").and_then(|text| {
            PackVersion::new(&text)
                .map_err(|error| fields.refuse("version", LockError::from(error)))
                .ok()
        });
        let tree = read_hash(fields, "tree");
        let source = read_source(fields, key_path);
        fields.refuse_unknown_keys(&ENTRY_KEYS);

        let entry = match (name.clone(), version, source, tree) {
            (Some(name), Some(version), Some(source), Some(tree)) => Some(Self {
                name,
                version,
                source,
                tree,
            }),
            _ => None,
        };
        ReadEntry { name, entry }
    }
}

/// Reads the source keys of the entry at `key_path`: a git source needs its `rev`, a path source
/// holds no git key.
fn read_source(fields: &mut TomlFields, key_path: &str) -> Option<LockedSource> {
    match SourceFields::read_into(fields, key_path, PinRule::RevRequired)? {
        SourceFields::Git {
            url,
            pin: Pin::Rev(rev),
            tag,
            subdir,
        } => Some(LockedSource::Git {
            url,
            tag,
            subdir,
            rev,
        }),
        // `read_into` under `RevRequired` has already refused a git entry with no `rev`, so a tag
        // pin never reaches here with nothing refused.
        SourceFields::Git {
            pin: Pin::Tag(_), ..
        } => None,
        SourceFields::Path(path) => Some(LockedSource::Path(path)),
    }
}

/// Reads the string at `key` as a content hash, refusing a missing key, a wrong type and a string
/// that is not a hash.
pub(in crate::types::lock) fn read_hash(fields: &mut TomlFields, key: &str) -> Option<ContentHash> {
    let text = fields.required_string(key)?;
    match ContentHash::parse(&text) {
        Ok(hash) => Some(hash),
        Err(error) => {
            fields.refuse(key, LockError::from(error));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;
    use crate::types::name::PackName;
    use crate::types::name::ToolName;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";
    const HEX64: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    /// What reading an entry gave: the entry, or every refusal as displayed text.
    type Outcome<N> = Result<LockedEntry<N>, Vec<String>>;

    fn read<N: Clone>(
        key_path: &str,
        text: &str,
        name_rule: fn(&str) -> Result<N, InvalidName>,
    ) -> Result<Outcome<N>, Box<dyn std::error::Error>> {
        let table: toml::Table = text.parse()?;
        let mut fields = TomlFields::new(Path::new("agentstacks.lock"), key_path, table);
        let read = LockedEntry::read(&mut fields, key_path, name_rule);
        let shown: Vec<String> = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok(match read.entry {
            Some(entry) if shown.is_empty() => Ok(entry),
            _ => Err(shown),
        })
    }

    fn read_pack(
        key_path: &str,
        text: &str,
    ) -> Result<Outcome<PackName>, Box<dyn std::error::Error>> {
        read(key_path, text, PackName::new)
    }

    fn parsed_pack(
        key_path: &str,
        text: &str,
    ) -> Result<LockedEntry<PackName>, Box<dyn std::error::Error>> {
        read_pack(key_path, text)?.map_err(|shown| shown.join("; ").into())
    }

    fn git_entry(extra: &str) -> String {
        format!(
            "name = \"core\"\nversion = \"0.3.0\"\ngit = \"https://github.com/airsstack/agentstacks\"\n\
             rev = \"{HEX40}\"\ntree = \"sha256:{HEX64}\"\n{extra}"
        )
    }

    fn path_entry(extra: &str) -> String {
        format!(
            "name = \"team-lint\"\nversion = \"0.1.0\"\npath = \"packs/team-lint\"\n\
             tree = \"sha256:{HEX64}\"\n{extra}"
        )
    }

    #[test]
    fn locked_entry_parses_a_git_entry() -> TestResult {
        let entry = parsed_pack(
            "pack[0]",
            &git_entry("tag = \"packs-v0.1.0\"\nsubdir = \"packs/core\"\n"),
        )?;
        assert_eq!(entry.name(), &PackName::new("core")?);
        assert_eq!(entry.version(), &PackVersion::new("0.3.0")?);
        assert_eq!(
            entry.source(),
            &LockedSource::Git {
                url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
                tag: Some(GitTag::new("packs-v0.1.0")?),
                subdir: Some(SubPath::new("packs/core")?),
                rev: CommitId::new(HEX40)?,
            }
        );
        assert_eq!(entry.tag(), Some(&GitTag::new("packs-v0.1.0")?));
        assert_eq!(
            entry.tree(),
            &ContentHash::parse(&format!("sha256:{HEX64}"))?
        );
        assert_eq!(
            entry.source().to_source(),
            Source::Git {
                url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
                pin: Pin::Rev(CommitId::new(HEX40)?),
                subdir: Some(SubPath::new("packs/core")?),
            }
        );
        Ok(())
    }

    #[test]
    fn locked_entry_parses_a_path_entry() -> TestResult {
        let entry = parsed_pack("pack[2]", &path_entry(""))?;
        assert_eq!(entry.name(), &PackName::new("team-lint")?);
        assert_eq!(
            entry.source(),
            &LockedSource::Path(LocalPath::new("packs/team-lint")?)
        );
        assert_eq!(entry.tag(), None);
        assert_eq!(
            entry.source().to_source(),
            Source::Path(LocalPath::new("packs/team-lint")?)
        );
        Ok(())
    }

    #[test]
    fn locked_entry_keeps_the_tag_optional() -> TestResult {
        let without = parsed_pack("pack[0]", &git_entry(""))?;
        assert_eq!(without.tag(), None);

        let with = parsed_pack("pack[0]", &git_entry("tag = \"v1\"\n"))?;
        assert_eq!(with.tag(), Some(&GitTag::new("v1")?));
        assert!(matches!(
            with.source(),
            LockedSource::Git { rev, .. } if rev.as_str() == HEX40
        ));

        let tool: LockedEntry<ToolName> = read(
            "tool[0]",
            &format!(
                "name = \"claude\"\nversion = \"0.1.0\"\n\
                 git = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\n\
                 subdir = \"extensions/claude\"\nrev = \"{HEX40}\"\ntree = \"sha256:{HEX64}\"\n"
            ),
            ToolName::new,
        )?
        .map_err(|shown| shown.join("; "))?;
        assert_eq!(tool.name(), &ToolName::new("claude")?);
        Ok(())
    }

    /// Asserts each `(key path, entry text, message)` is refused with exactly that one message.
    fn assert_each_refused(cases: Vec<(&str, String, String)>) -> TestResult {
        for (key_path, text, expected) in cases {
            let shown = read_pack(key_path, &text)?
                .err()
                .ok_or("expected a refusal")?;
            assert_eq!(shown, [expected], "{text}");
        }
        Ok(())
    }

    #[test]
    fn locked_entry_refuses_missing_fields() -> TestResult {
        let tool = read(
            "tool[0]",
            &git_entry("").replace(&format!("rev = \"{HEX40}\"\n"), ""),
            ToolName::new,
        )?
        .err()
        .ok_or("expected a refusal")?;
        assert_eq!(tool, ["agentstacks.lock: tool[0].rev: required"]);

        let cases: Vec<(&str, String, String)> = vec![
            (
                "pack[1]",
                git_entry("").replace(&format!("rev = \"{HEX40}\"\n"), ""),
                "agentstacks.lock: pack[1].rev: required".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace(&format!("tree = \"sha256:{HEX64}\"\n"), ""),
                "agentstacks.lock: pack[1].tree: required".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace("name = \"core\"\n", ""),
                "agentstacks.lock: pack[1].name: required".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace("version = \"0.3.0\"\n", ""),
                "agentstacks.lock: pack[1].version: required".to_owned(),
            ),
        ];
        assert_each_refused(cases)
    }

    #[test]
    fn locked_entry_refuses_each_source_rule() -> TestResult {
        let cases: Vec<(&str, String, String)> = vec![
            (
                "pack[1]",
                git_entry("path = \"packs/x\"\n"),
                "agentstacks.lock: pack[1]: needs exactly one of git, path".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("")
                    .replace("git = \"https://github.com/airsstack/agentstacks\"\n", "")
                    .replace(&format!("rev = \"{HEX40}\"\n"), ""),
                "agentstacks.lock: pack[1]: needs exactly one of git, path".to_owned(),
            ),
            (
                "pack[2]",
                path_entry("tag = \"v1\"\n"),
                "agentstacks.lock: pack[2]: tag, rev and subdir apply only to git sources"
                    .to_owned(),
            ),
            (
                "pack[2]",
                path_entry(&format!("rev = \"{HEX40}\"\n")),
                "agentstacks.lock: pack[2]: tag, rev and subdir apply only to git sources"
                    .to_owned(),
            ),
            (
                "pack[2]",
                path_entry("subdir = \"x\"\n"),
                "agentstacks.lock: pack[2]: tag, rev and subdir apply only to git sources"
                    .to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace(&format!("rev = \"{HEX40}\""), "rev = \"main\""),
                "agentstacks.lock: pack[1].rev: not a commit id".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace("https://github.com/airsstack/agentstacks", "-x"),
                "agentstacks.lock: pack[1].git: URL must not start with \"-\"".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("subdir = \"../x\"\n"),
                "agentstacks.lock: pack[1].subdir: must stay inside the repository".to_owned(),
            ),
            (
                "pack[2]",
                path_entry("").replace("packs/team-lint", "/abs"),
                "agentstacks.lock: pack[2].path: absolute paths are not portable".to_owned(),
            ),
            (
                "pack[2]",
                path_entry("").replace("packs/team-lint", "../x"),
                "agentstacks.lock: pack[2].path: must stay inside the project".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("tag = \" \"\n"),
                "agentstacks.lock: pack[1].tag: empty".to_owned(),
            ),
        ];
        assert_each_refused(cases)
    }

    #[test]
    fn locked_entry_refuses_malformed_fields() -> TestResult {
        let hash_message = "bad hash (expected sha256:<64 lowercase hex>)";
        let cases: Vec<(&str, String, String)> = vec![
            (
                "pack[1]",
                git_entry("").replace(HEX64, &HEX64.to_uppercase()),
                format!("agentstacks.lock: pack[1].tree: {hash_message}"),
            ),
            (
                "pack[1]",
                git_entry("").replace("sha256:", ""),
                format!("agentstacks.lock: pack[1].tree: {hash_message}"),
            ),
            (
                "pack[1]",
                git_entry("").replace("name = \"core\"", "name = \"Core\""),
                "agentstacks.lock: pack[1].name: \"Core\" is not a valid name".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace("0.3.0", "1.2"),
                "agentstacks.lock: pack[1].version: \"1.2\" is not semver".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("").replace("name = \"core\"", "name = 3"),
                "agentstacks.lock: pack[1].name: expected string, got integer".to_owned(),
            ),
            (
                "pack[1]",
                git_entry("branch = \"main\"\n"),
                "agentstacks.lock: pack[1].branch: unknown key".to_owned(),
            ),
        ];
        assert_each_refused(cases)
    }

    #[test]
    fn locked_entry_reports_every_field_error() -> TestResult {
        let text = git_entry("branch = \"main\"\n")
            .replace("name = \"core\"\n", "")
            .replace(&format!("sha256:{HEX64}"), "sha256:XYZ");
        let shown: BTreeSet<String> = read_pack("pack[0]", &text)?
            .err()
            .ok_or("expected a refusal")?
            .into_iter()
            .collect();
        let expected: BTreeSet<String> = [
            "agentstacks.lock: pack[0].name: required",
            "agentstacks.lock: pack[0].tree: bad hash (expected sha256:<64 lowercase hex>)",
            "agentstacks.lock: pack[0].branch: unknown key",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn locked_entry_new_keeps_a_git_or_a_path_origin() -> TestResult {
        let tree = ContentHash::parse(&format!("sha256:{HEX64}"))?;
        let git = LockedEntry::new(
            PackName::new("core")?,
            PackVersion::new("0.3.0")?,
            LockedSource::Git {
                url: GitUrl::new("https://example.org/r")?,
                tag: Some(GitTag::new("v1")?),
                subdir: None,
                rev: CommitId::new(HEX40)?,
            },
            tree.clone(),
        );
        assert_eq!(git.tag(), Some(&GitTag::new("v1")?));
        assert_eq!(
            git.source().to_source(),
            Source::Git {
                url: GitUrl::new("https://example.org/r")?,
                pin: Pin::Rev(CommitId::new(HEX40)?),
                subdir: None,
            }
        );

        let path = LockedEntry::new(
            PackName::new("team-lint")?,
            PackVersion::new("0.1.0")?,
            LockedSource::Path(LocalPath::new("packs/team-lint")?),
            tree,
        );
        assert_eq!(path.tag(), None);
        assert_eq!(
            path.source().to_source(),
            Source::Path(LocalPath::new("packs/team-lint")?)
        );
        Ok(())
    }

    #[test]
    fn locked_entry_exposes_the_name_of_a_partly_valid_entry() -> TestResult {
        let table: toml::Table = git_entry("").replace("0.3.0", "1.2").parse()?;
        let mut fields = TomlFields::new(Path::new("agentstacks.lock"), "pack[0]", table);
        let read = LockedEntry::read(&mut fields, "pack[0]", PackName::new);
        assert!(read.entry.is_none());
        assert_eq!(read.name, Some(PackName::new("core")?));
        Ok(())
    }
}
