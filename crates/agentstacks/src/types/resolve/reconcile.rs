//! Classifying every tool and pack entry against the lock.
//!
//! It is separate from the tree check because it only compares the entries' recorded fields, so
//! it needs no fetched tree and never touches the network.
//!
//! # Responsibilities
//!
//! - [`same_source`] says whether a source in `agentstacks.toml` is the one the lock recorded,
//!   comparing the fields as normalised values.
//! - [`Reconciled`] is the class of one entry.
//! - [`reconcile`] classifies every entry of the project file and of the lock, once each.
//!
//! # Non-responsibilities
//!
//! Resolving a tag, fetching a tree and comparing it with the lock's belong to the flows and the
//! tree check.

use std::collections::BTreeMap;

use crate::types::lock::LockedEntry;
use crate::types::lock::LockedSource;
use crate::types::lock::Lockfile;
use crate::types::project::CommitId;
use crate::types::project::LocalPath;
use crate::types::project::Pin;
use crate::types::project::ProjectFile;
use crate::types::project::Source;
use crate::types::project::SubPath;
use crate::types::resolve::SourceKey;
use crate::types::tree::ContentHash;

/// What the lock says about one entry of the project file, or about an entry the file dropped.
///
/// # Examples
///
/// ```
/// use agentstacks::types::resolve::Reconciled;
///
/// // An entry the lock has never seen is resolved from scratch.
/// assert_eq!(Reconciled::New, Reconciled::New);
/// assert_ne!(Reconciled::New, Reconciled::Stale);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciled {
    /// The entry is in the project file and not in the lock: resolve it.
    New,
    /// The entry's source fields equal the lock's: use the lock's revision and check the tree.
    Locked {
        /// The locked commit; `None` for a `path` source.
        rev: Option<CommitId>,
        /// The tree hash the lock pins.
        tree: ContentHash,
    },
    /// The entry's source fields differ from the lock's: resolve it as new.
    Stale,
    /// A locked pack the project file no longer lists: its files are deleted.
    RemovedPack,
    /// A locked tool the project file no longer lists. Its locked source is fetched and its tree
    /// checked like any locked entry.
    RemovedTool {
        /// Where the lock fetched the tool from.
        source: LockedSource,
        /// The tree hash the lock pins.
        tree: ContentHash,
    },
}

/// A project directory with every empty and `.` segment removed; `..` segments stay as written.
fn normalized_path(path: &LocalPath) -> String {
    path.as_str()
        .split('/')
        .filter(|segment| !matches!(*segment, "" | "."))
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether `source`, as `agentstacks.toml` states it, is the source `locked` recorded.
///
/// Git sources match on the URL, the normalised `subdir` (none equals none) and the pin: a tag pin
/// matches a lock that holds the same tag, whatever its revision, and a revision pin matches a lock
/// that holds the same revision and no tag. Path sources match on the normalised directory.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{LocalPath, LockedSource, Source};
/// use agentstacks::types::resolve::same_source;
///
/// let source = Source::Path(LocalPath::new("./packs/team-lint")?);
/// let locked = LockedSource::Path(LocalPath::new("packs/team-lint")?);
/// assert!(same_source(&source, &locked));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn same_source(source: &Source, locked: &LockedSource) -> bool {
    match (source, locked) {
        (
            Source::Git { url, pin, subdir },
            LockedSource::Git {
                url: locked_url,
                tag,
                subdir: locked_subdir,
                rev,
            },
        ) => {
            let pinned = match pin {
                Pin::Tag(wanted) => tag.as_ref() == Some(wanted),
                Pin::Rev(wanted) => wanted == rev && tag.is_none(),
            };
            url == locked_url
                && pinned
                && subdir.as_ref().map(SubPath::normalized)
                    == locked_subdir.as_ref().map(SubPath::normalized)
        }
        (Source::Path(path), LockedSource::Path(locked_path)) => {
            normalized_path(path) == normalized_path(locked_path)
        }
        _ => false,
    }
}

/// The class of a source that has a locked entry beside it.
fn against_lock<N>(source: &Source, locked: &LockedEntry<N>) -> Reconciled {
    if !same_source(source, locked.source()) {
        return Reconciled::Stale;
    }
    let rev = match locked.source() {
        LockedSource::Git { rev, .. } => Some(rev.clone()),
        LockedSource::Path(_) => None,
    };
    Reconciled::Locked {
        rev,
        tree: locked.tree().clone(),
    }
}

/// Classifies every tool and pack of `project` and of `lock`.
///
/// Without a lock every entry of the project file is [`Reconciled::New`]. Only the lock is read, so
/// a remote tag that moved cannot change the answer.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::ProjectFile;
/// use agentstacks::types::resolve::{Reconciled, reconcile};
///
/// let text = "api = 1\n\
///     [tools.claude]\n\
///     path = \"extensions/claude\"\n\
///     [packs.core]\n\
///     path = \"packs/core\"\n";
/// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// let classes = reconcile(&project, None);
/// assert_eq!(classes.len(), 2);
/// assert!(classes.values().all(|class| *class == Reconciled::New));
/// # Ok::<(), String>(())
/// ```
#[must_use]
pub fn reconcile(
    project: &ProjectFile,
    lock: Option<&Lockfile>,
) -> BTreeMap<SourceKey, Reconciled> {
    let mut classes = BTreeMap::new();
    let locked_tools = lock.map_or(&[][..], Lockfile::tools);
    let locked_packs = lock.map_or(&[][..], Lockfile::packs);

    for (name, source) in project.tools() {
        let class = locked_tools
            .iter()
            .find(|entry| entry.name() == name)
            .map_or(Reconciled::New, |entry| against_lock(source, entry));
        classes.insert(SourceKey::Tool(name.clone()), class);
    }
    for (name, entry) in project.packs() {
        let class = locked_packs
            .iter()
            .find(|locked| locked.name() == name)
            .map_or(Reconciled::New, |locked| {
                against_lock(entry.source(), locked)
            });
        classes.insert(SourceKey::Pack(name.clone()), class);
    }
    for locked in locked_tools {
        classes
            .entry(SourceKey::Tool(locked.name().clone()))
            .or_insert_with(|| Reconciled::RemovedTool {
                source: locked.source().clone(),
                tree: locked.tree().clone(),
            });
    }
    for locked in locked_packs {
        classes
            .entry(SourceKey::Pack(locked.name().clone()))
            .or_insert(Reconciled::RemovedPack);
    }
    classes
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::lock::LockContents;
    use crate::types::manifest::PackVersion;
    use crate::types::name::PackName;
    use crate::types::name::ToolName;
    use crate::types::project::GitTag;
    use crate::types::project::GitUrl;
    use crate::types::project::SubPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const URL: &str = "https://github.com/airsstack/agentstacks";
    const REV: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER_REV: &str = "89abcdef0123456789abcdef0123456789abcdef";

    fn git(pin: Pin, subdir: Option<&str>) -> Result<Source, Box<dyn std::error::Error>> {
        Ok(Source::Git {
            url: GitUrl::new(URL)?,
            pin,
            subdir: subdir.map(SubPath::new).transpose()?,
        })
    }

    fn tag_pin(tag: &str) -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Tag(GitTag::new(tag)?))
    }

    fn rev_pin(rev: &str) -> Result<Pin, Box<dyn std::error::Error>> {
        Ok(Pin::Rev(CommitId::new(rev)?))
    }

    fn locked_git(
        tag: Option<&str>,
        subdir: Option<&str>,
        rev: &str,
    ) -> Result<LockedSource, Box<dyn std::error::Error>> {
        Ok(LockedSource::Git {
            url: GitUrl::new(URL)?,
            tag: tag.map(GitTag::new).transpose()?,
            subdir: subdir.map(SubPath::new).transpose()?,
            rev: CommitId::new(rev)?,
        })
    }

    fn locked_path(path: &str) -> Result<LockedSource, Box<dyn std::error::Error>> {
        Ok(LockedSource::Path(LocalPath::new(path)?))
    }

    fn tree(seed: &str) -> ContentHash {
        ContentHash::of_bytes(seed.as_bytes())
    }

    #[test]
    fn same_source_tag_pin_matches_the_locked_tag() -> TestResult {
        let source = git(tag_pin("packs-v0.1.0")?, None)?;
        assert!(same_source(
            &source,
            &locked_git(Some("packs-v0.1.0"), None, REV)?
        ));
        assert!(same_source(
            &source,
            &locked_git(Some("packs-v0.1.0"), None, OTHER_REV)?
        ));
        assert!(!same_source(
            &source,
            &locked_git(Some("packs-v0.2.0"), None, REV)?
        ));
        assert!(!same_source(&source, &locked_git(None, None, REV)?));
        Ok(())
    }

    #[test]
    fn same_source_rev_pin_needs_no_locked_tag() -> TestResult {
        let source = git(rev_pin(REV)?, None)?;
        assert!(same_source(&source, &locked_git(None, None, REV)?));
        assert!(!same_source(
            &source,
            &locked_git(Some("packs-v0.1.0"), None, REV)?
        ));
        assert!(!same_source(&source, &locked_git(None, None, OTHER_REV)?));
        Ok(())
    }

    #[test]
    fn same_source_compares_normalised_subdir_and_path() -> TestResult {
        let source = git(tag_pin("v1")?, Some("./packs/core"))?;
        assert!(same_source(
            &source,
            &locked_git(Some("v1"), Some("packs/core"), REV)?
        ));
        let path = Source::Path(LocalPath::new("packs//team-lint/.")?);
        assert!(same_source(&path, &locked_path("packs/team-lint")?));
        Ok(())
    }

    #[test]
    fn same_source_differs_on_url_subdir_path_or_kind() -> TestResult {
        let source = git(tag_pin("v1")?, Some("packs/core"))?;
        let other_url = Source::Git {
            url: GitUrl::new("https://example.org/other")?,
            pin: tag_pin("v1")?,
            subdir: Some(SubPath::new("packs/core")?),
        };
        let locked = locked_git(Some("v1"), Some("packs/core"), REV)?;
        assert!(same_source(&source, &locked));
        assert!(!same_source(&other_url, &locked));
        assert!(!same_source(
            &source,
            &locked_git(Some("v1"), Some("packs/other"), REV)?
        ));
        assert!(!same_source(&source, &locked_git(Some("v1"), None, REV)?));
        let no_subdir = git(tag_pin("v1")?, None)?;
        assert!(!same_source(&no_subdir, &locked));

        let path = Source::Path(LocalPath::new("packs/a")?);
        assert!(!same_source(&path, &locked_path("packs/b")?));
        assert!(!same_source(&path, &locked));
        assert!(!same_source(&source, &locked_path("packs/core")?));
        Ok(())
    }

    fn project(tools: &str, packs: &str) -> Result<ProjectFile, Box<dyn std::error::Error>> {
        let text = format!("api = 1\n{tools}{packs}");
        ProjectFile::parse(Path::new("agentstacks.toml"), &text)
            .map_err(|refusals| format!("{refusals:?}").into())
    }

    fn lock(
        tools: Vec<LockedEntry<ToolName>>,
        packs: Vec<LockedEntry<PackName>>,
    ) -> Result<Lockfile, Box<dyn std::error::Error>> {
        let contents = LockContents {
            tools,
            packs,
            ..LockContents::default()
        };
        Lockfile::new(Path::new("agentstacks.lock"), contents)
            .map_err(|refusals| format!("{refusals:?}").into())
    }

    fn tool_entry(
        name: &str,
        source: LockedSource,
        seed: &str,
    ) -> Result<LockedEntry<ToolName>, Box<dyn std::error::Error>> {
        Ok(LockedEntry::new(
            ToolName::new(name)?,
            PackVersion::new("0.1.0")?,
            source,
            tree(seed),
        ))
    }

    fn pack_entry(
        name: &str,
        source: LockedSource,
        seed: &str,
    ) -> Result<LockedEntry<PackName>, Box<dyn std::error::Error>> {
        Ok(LockedEntry::new(
            PackName::new(name)?,
            PackVersion::new("0.1.0")?,
            source,
            tree(seed),
        ))
    }

    fn pack_key(name: &str) -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new(name)?))
    }

    fn tool_key(name: &str) -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Tool(ToolName::new(name)?))
    }

    const TOOL: &str = "[tools.claude]\npath = \"extensions/claude\"\n";
    const GIT_PACK: &str = "[packs.core]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/core\"\n";

    #[test]
    fn reconcile_new_entry() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let lock = lock(Vec::new(), Vec::new())?;
        for lock in [Some(&lock), None] {
            let classes = reconcile(&file, lock);
            assert_eq!(classes.len(), 2);
            assert_eq!(classes.get(&pack_key("core")?), Some(&Reconciled::New));
            assert_eq!(classes.get(&tool_key("claude")?), Some(&Reconciled::New));
        }
        Ok(())
    }

    #[test]
    fn reconcile_locked_entry_uses_the_lock_rev() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let source = locked_git(Some("packs-v0.1.0"), Some("packs/core"), REV)?;
        let lock = lock(
            vec![tool_entry(
                "claude",
                locked_path("extensions/claude")?,
                "t",
            )?],
            vec![pack_entry("core", source, "p")?],
        )?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(
            classes.get(&pack_key("core")?),
            Some(&Reconciled::Locked {
                rev: Some(CommitId::new(REV)?),
                tree: tree("p"),
            })
        );
        Ok(())
    }

    #[test]
    fn reconcile_path_entry_locked_without_rev() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let lock = lock(
            vec![tool_entry(
                "claude",
                locked_path("./extensions/claude")?,
                "t",
            )?],
            Vec::new(),
        )?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(
            classes.get(&tool_key("claude")?),
            Some(&Reconciled::Locked {
                rev: None,
                tree: tree("t"),
            })
        );
        Ok(())
    }

    #[test]
    fn reconcile_changed_source_is_stale() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let source = locked_git(Some("packs-v0.0.9"), Some("packs/core"), REV)?;
        let lock = lock(
            vec![tool_entry("claude", locked_path("extensions/other")?, "t")?],
            vec![pack_entry("core", source, "p")?],
        )?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(classes.get(&pack_key("core")?), Some(&Reconciled::Stale));
        assert_eq!(classes.get(&tool_key("claude")?), Some(&Reconciled::Stale));
        Ok(())
    }

    #[test]
    fn reconcile_removed_pack() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let lock = lock(
            Vec::new(),
            vec![pack_entry("old", locked_path("packs/old")?, "o")?],
        )?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(
            classes.get(&pack_key("old")?),
            Some(&Reconciled::RemovedPack)
        );
        assert_eq!(classes.len(), 3);
        Ok(())
    }

    #[test]
    fn reconcile_removed_tool_keeps_its_locked_source() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let source = locked_git(None, None, REV)?;
        let lock = lock(
            vec![tool_entry("opencode", source.clone(), "x")?],
            Vec::new(),
        )?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(
            classes.get(&tool_key("opencode")?),
            Some(&Reconciled::RemovedTool {
                source,
                tree: tree("x"),
            })
        );
        Ok(())
    }

    #[test]
    fn moved_tag_keeps_the_lock_rev() -> TestResult {
        // The remote tag now names another commit, but only the lock is input: the toml tag equals
        // the lock's tag, so the lock's revision stands.
        let file = project(TOOL, GIT_PACK)?;
        let source = locked_git(Some("packs-v0.1.0"), Some("packs/core"), OTHER_REV)?;
        let lock = lock(Vec::new(), vec![pack_entry("core", source, "p")?])?;
        let classes = reconcile(&file, Some(&lock));
        assert_eq!(
            classes.get(&pack_key("core")?),
            Some(&Reconciled::Locked {
                rev: Some(CommitId::new(OTHER_REV)?),
                tree: tree("p"),
            })
        );
        Ok(())
    }

    #[test]
    fn classes_follow_packs_then_tools_order() -> TestResult {
        let file = project(TOOL, GIT_PACK)?;
        let classes = reconcile(&file, None);
        let keys: Vec<String> = classes.keys().map(ToString::to_string).collect();
        assert_eq!(keys, ["packs.core", "tools.claude"]);
        Ok(())
    }
}
