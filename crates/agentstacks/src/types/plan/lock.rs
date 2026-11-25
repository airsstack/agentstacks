//! The contents of the new lock, from the steps of a plan.
//!
//! # Responsibilities
//!
//! - [`lock_contents`] builds the [`LockContents`] of the new lock from the steps: after a full
//!   apply every step contributes its entry; after a failed swap only the paths that landed do,
//!   and the old entries stand for the ones that did not. [`Landed`] chooses between the two.
//! - [`airsl_pin`] gives the airsl version to record: the installed one, exactly when a locked
//!   pack declares `requires.airsl`.
//!
//! # Non-responsibilities
//!
//! Building the locked tools and packs, turning the contents into a
//! [`Lockfile`](crate::types::Lockfile) and checking the airsl section against the packs belong to
//! the plan flow.

use std::collections::BTreeSet;

use crate::types::lock::{FileEntry, LockContents, LockedEntry};
use crate::types::manifest::{PackManifest, PackVersion};
use crate::types::name::{PackName, ToolName};
use crate::types::op::OpPath;
use crate::types::plan::{Action, Step};
use crate::types::project::{InvalidSubPath, SubPath};
use crate::types::tree::ContentHash;

/// The airsl version to record in the lock: `installed`, when a locked pack declares
/// `requires.airsl`, and nothing otherwise.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{PackManifest, PackVersion};
/// use agentstacks::types::plan::airsl_pin;
///
/// let manifest = PackManifest::parse(
///     Path::new("packs/core/pack.toml"),
///     "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n\
///      [requires]\nairsl = \">=0.1.4\"\n",
/// )
/// .map_err(|refusals| format!("{refusals:?}"))?;
/// let installed = PackVersion::new("0.1.4")?;
/// assert_eq!(airsl_pin([&manifest], Some(&installed)), Some(installed));
/// assert_eq!(airsl_pin([], None), None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn airsl_pin<'a>(
    locked: impl IntoIterator<Item = &'a PackManifest>,
    installed: Option<&PackVersion>,
) -> Option<PackVersion> {
    let needed = locked
        .into_iter()
        .any(|manifest| manifest.requires().airsl().is_some());
    if needed { installed.cloned() } else { None }
}

/// How far an apply got.
#[derive(Debug, Clone, Copy)]
pub enum Landed<'a> {
    /// Every step was applied.
    All,
    /// A swap failed: only the steps that touch disk and are listed in `paths` landed.
    Only {
        /// The paths whose rename or delete was performed.
        paths: &'a BTreeSet<OpPath>,
        /// The `[[file]]` entries of the lock the plan was made against.
        old: &'a [FileEntry],
    },
}

/// The contents of the new lock.
///
/// `tools`, `packs` and `airsl` are carried as given. A step that gives the lock an entry is
/// one with an owner and one of the actions create, update, restore or unchanged: the entry's hash
/// is the hash of the new bytes. A shared file, a delete and a forget give none.
///
/// # Errors
///
/// [`InvalidSubPath`] when a step's path is not a valid lock path; a path an operation accepted
/// always is.
pub fn lock_contents(
    tools: Vec<LockedEntry<ToolName>>,
    packs: Vec<LockedEntry<PackName>>,
    steps: &[Step],
    airsl: Option<PackVersion>,
    landed: Landed<'_>,
) -> Result<LockContents, InvalidSubPath> {
    let mut files = Vec::new();
    for step in steps {
        let entry = match landed {
            Landed::Only { paths, old }
                if step.action.touches_disk() && !paths.contains(&step.path) =>
            {
                old_entry(old, &step.path)
            }
            Landed::All | Landed::Only { .. } => new_entry(step)?,
        };
        files.extend(entry);
    }
    Ok(LockContents {
        airsl,
        tools,
        packs,
        files,
    })
}

/// The entry `step` gives the new lock, when it gives one.
fn new_entry(step: &Step) -> Result<Option<FileEntry>, InvalidSubPath> {
    let records = matches!(
        step.action,
        Action::Create | Action::Update | Action::Restore | Action::Unchanged
    );
    let (true, Some(owner), Some(new)) = (records, &step.owner, &step.new) else {
        return Ok(None);
    };
    Ok(Some(FileEntry::new(
        SubPath::new(step.path.as_str())?,
        ContentHash::of_bytes(new),
        owner.clone(),
    )))
}

/// The entry the old lock held for `path`, when it held one.
fn old_entry(old: &[FileEntry], path: &OpPath) -> Option<FileEntry> {
    old.iter()
        .find(|entry| entry.path().normalized() == path.as_str())
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::lock::{LockedSource, Lockfile, Owner, check_airsl_presence, write_lock};
    use crate::types::project::LocalPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn tree() -> ContentHash {
        ContentHash::of_bytes(b"tree")
    }

    fn tool(
        name: &str,
        version: &str,
    ) -> Result<LockedEntry<ToolName>, Box<dyn std::error::Error>> {
        Ok(LockedEntry::new(
            ToolName::new(name)?,
            PackVersion::new(version)?,
            LockedSource::Path(LocalPath::new("extensions/claude")?),
            tree(),
        ))
    }

    fn pack(
        name: &str,
        version: &str,
    ) -> Result<LockedEntry<PackName>, Box<dyn std::error::Error>> {
        Ok(LockedEntry::new(
            PackName::new(name)?,
            PackVersion::new(version)?,
            LockedSource::Path(LocalPath::new("packs/core")?),
            tree(),
        ))
    }

    fn step(
        path: &str,
        action: Action,
        owner: Option<&str>,
        new: Option<&[u8]>,
    ) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(path)?,
            action,
            owner: owner.map(Owner::new).transpose()?,
            origin: None,
            new: new.map(<[u8]>::to_vec),
            old: None,
            decided_on: None,
        })
    }

    fn old(path: &str, bytes: &[u8], by: &str) -> Result<FileEntry, Box<dyn std::error::Error>> {
        Ok(FileEntry::new(
            SubPath::new(path)?,
            ContentHash::of_bytes(bytes),
            Owner::new(by)?,
        ))
    }

    fn all(steps: &[Step]) -> Result<LockContents, Box<dyn std::error::Error>> {
        Ok(lock_contents(
            Vec::new(),
            Vec::new(),
            steps,
            None,
            Landed::All,
        )?)
    }

    fn only(
        steps: &[Step],
        paths: &[&str],
        olds: &[FileEntry],
    ) -> Result<LockContents, Box<dyn std::error::Error>> {
        let landed = paths
            .iter()
            .map(|path| OpPath::new(path))
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok(lock_contents(
            Vec::new(),
            Vec::new(),
            steps,
            None,
            Landed::Only {
                paths: &landed,
                old: olds,
            },
        )?)
    }

    fn listed(contents: &LockContents) -> Vec<(String, String, String)> {
        let mut rows: Vec<_> = contents
            .files
            .iter()
            .map(|file| {
                (
                    file.path().as_str().to_owned(),
                    file.by().to_string(),
                    file.hash().hex().to_owned(),
                )
            })
            .collect();
        rows.sort();
        rows
    }

    fn row(path: &str, by: &str, bytes: &[u8]) -> (String, String, String) {
        (
            path.to_owned(),
            by.to_owned(),
            ContentHash::of_bytes(bytes).hex().to_owned(),
        )
    }

    #[test]
    fn lock_records_written_and_unchanged_steps() -> TestResult {
        let steps = [
            step("a", Action::Create, Some("core"), Some(b"a"))?,
            step("b", Action::Update, Some("core"), Some(b"b"))?,
            step("c", Action::Restore, Some("claude"), Some(b"c"))?,
            step("d", Action::Unchanged, Some("claude"), Some(b"d"))?,
        ];
        assert_eq!(
            listed(&all(&steps)?),
            [
                row("a", "core", b"a"),
                row("b", "core", b"b"),
                row("c", "claude", b"c"),
                row("d", "claude", b"d"),
            ]
        );
        Ok(())
    }

    #[test]
    fn lock_leaves_out_shared_delete_and_forget() -> TestResult {
        let steps = [
            step(".claude/settings.json", Action::Update, None, Some(b"s"))?,
            step("gone", Action::Delete, Some("core"), None)?,
            step("dropped", Action::Forget, Some("core"), None)?,
        ];
        assert!(all(&steps)?.files.is_empty());
        Ok(())
    }

    #[test]
    fn lock_takes_the_new_hash_for_the_same_user_edit() -> TestResult {
        let steps = [step("a", Action::Unchanged, Some("core"), Some(b"edited"))?];
        assert_eq!(listed(&all(&steps)?), [row("a", "core", b"edited")]);
        Ok(())
    }

    #[test]
    fn lock_contents_build_a_lockfile() -> TestResult {
        let steps = [
            step("a", Action::Create, Some("core"), Some(b"a"))?,
            step("b", Action::Create, Some("claude"), Some(b"b"))?,
        ];
        let contents = lock_contents(
            vec![tool("claude", "0.1.0")?],
            vec![pack("core", "0.3.0")?],
            &steps,
            None,
            Landed::All,
        )?;
        let file = Path::new("agentstacks.lock");
        let lock = Lockfile::new(file, contents).map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(lock.files().len(), 2);
        let text = write_lock(&lock);
        let reparsed = Lockfile::parse(file, &text).map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(reparsed, lock);
        Ok(())
    }

    fn manifest(requires: &str) -> Result<PackManifest, Box<dyn std::error::Error>> {
        let text = format!(
            "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n{requires}"
        );
        PackManifest::parse(Path::new("packs/core/pack.toml"), &text)
            .map_err(|refusals| format!("{refusals:?}").into())
    }

    #[test]
    fn airsl_pinned_exactly_when_a_locked_pack_requires_it() -> TestResult {
        let needing = manifest("[requires]\nairsl = \">=0.1.4\"\n")?;
        let plain = manifest("")?;
        let installed = PackVersion::new("0.1.4")?;
        let file = Path::new("agentstacks.lock");

        let pinned = airsl_pin([&needing, &plain], Some(&installed));
        assert_eq!(pinned, Some(installed));
        let lock = Lockfile::new(
            file,
            lock_contents(Vec::new(), Vec::new(), &[], pinned, Landed::All)?,
        )
        .map_err(|refusals| format!("{refusals:?}"))?;
        check_airsl_presence(file, &lock, [&needing, &plain]).map_err(|d| d.to_string())?;

        let unpinned = airsl_pin([&plain], Some(&PackVersion::new("0.1.4")?));
        assert_eq!(unpinned, None);
        let lock = Lockfile::new(
            file,
            lock_contents(Vec::new(), Vec::new(), &[], unpinned, Landed::All)?,
        )
        .map_err(|refusals| format!("{refusals:?}"))?;
        check_airsl_presence(file, &lock, [&plain]).map_err(|d| d.to_string())?;
        Ok(())
    }

    #[test]
    fn partial_lock_keeps_old_entries_for_what_did_not_land() -> TestResult {
        let steps = [
            step("a", Action::Update, Some("core"), Some(b"a2"))?,
            step("b", Action::Update, Some("core"), Some(b"b2"))?,
            step("c", Action::Create, Some("core"), Some(b"c"))?,
            step("d", Action::Delete, Some("core"), None)?,
            step("e", Action::Delete, Some("core"), None)?,
        ];
        let olds = [
            old("a", b"a1", "core")?,
            old("b", b"b1", "core")?,
            old("d", b"d1", "core")?,
            old("e", b"e1", "core")?,
        ];
        let contents = only(&steps, &["a", "d"], &olds)?;
        assert_eq!(
            listed(&contents),
            [
                row("a", "core", b"a2"),
                row("b", "core", b"b1"),
                row("e", "core", b"e1"),
            ]
        );
        Ok(())
    }

    #[test]
    fn partial_lock_applies_forget_and_unchanged() -> TestResult {
        let steps = [
            step("f", Action::Forget, Some("core"), None)?,
            step("g", Action::Unchanged, Some("core"), Some(b"g"))?,
            step("h", Action::Unchanged, Some("core"), Some(b"h-new"))?,
        ];
        let olds = [old("f", b"f1", "core")?, old("h", b"h-old", "core")?];
        let contents = only(&steps, &[], &olds)?;
        assert_eq!(
            listed(&contents),
            [row("g", "core", b"g"), row("h", "core", b"h-new")]
        );
        Ok(())
    }

    #[test]
    fn partial_lock_takes_new_tools_packs_and_airsl() -> TestResult {
        let paths = BTreeSet::new();
        let contents = lock_contents(
            vec![tool("claude", "0.2.0")?],
            vec![pack("core", "0.4.0")?],
            &[],
            Some(PackVersion::new("0.1.5")?),
            Landed::Only {
                paths: &paths,
                old: &[],
            },
        )?;
        assert_eq!(contents.tools[0].version().to_string(), "0.2.0");
        assert_eq!(contents.packs[0].version().to_string(), "0.4.0");
        assert_eq!(
            contents.airsl.map(|v| v.to_string()).as_deref(),
            Some("0.1.5")
        );
        Ok(())
    }
}
