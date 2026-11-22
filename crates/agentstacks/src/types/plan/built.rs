//! The plan: everything a run decided, held as one value.
//!
//! # Responsibilities
//!
//! - [`Plan`] holds the steps in path order, the skips, the binary checks, the notes, the airsl
//!   pin, the new lock, and the file entries of the lock the plan was made against, so apply can
//!   build the partial lock of a failed swap from the plan alone.
//!
//! # Non-responsibilities
//!
//! Making the parts belongs to the plan flow, and showing the plan belongs to the commands.

use crate::types::lock::{FileEntry, Lockfile};
use crate::types::plan::{AirslPin, BinaryCheck, Note, Skipped, Step};

/// A built plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    steps: Vec<Step>,
    skips: Vec<Skipped>,
    binaries: Vec<BinaryCheck>,
    notes: Vec<Note>,
    airsl: Option<AirslPin>,
    lock: Lockfile,
    old_files: Vec<FileEntry>,
}

impl Plan {
    /// A plan of these parts, with the steps sorted by path so every run reads the same.
    ///
    /// `skips` holds each skipped subject with its reason: an extension's skip keeps its origin,
    /// and a skip the core made is held as the core made it. `old_files` is empty when there was
    /// no lock.
    #[must_use]
    pub fn new(
        steps: Vec<Step>,
        skips: Vec<Skipped>,
        binaries: Vec<BinaryCheck>,
        notes: Vec<Note>,
        airsl: Option<AirslPin>,
        lock: Lockfile,
        old_files: Vec<FileEntry>,
    ) -> Self {
        let mut steps = steps;
        steps.sort_by(|a, b| a.path.cmp(&b.path));
        Self {
            steps,
            skips,
            binaries,
            notes,
            airsl,
            lock,
            old_files,
        }
    }

    /// The steps, in path order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The skips, each with its reason and, for an extension's skip, who proposed it.
    #[must_use]
    pub fn skips(&self) -> &[Skipped] {
        &self.skips
    }

    /// The binary checks that passed.
    #[must_use]
    pub fn binaries(&self) -> &[BinaryCheck] {
        &self.binaries
    }

    /// The notes.
    #[must_use]
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// The airsl version change, when the lock pins airsl.
    #[must_use]
    pub const fn airsl(&self) -> Option<&AirslPin> {
        self.airsl.as_ref()
    }

    /// The lock a full apply writes.
    #[must_use]
    pub const fn lock(&self) -> &Lockfile {
        &self.lock
    }

    /// The file entries of the lock the plan was made against.
    #[must_use]
    pub fn old_files(&self) -> &[FileEntry] {
        &self.old_files
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::lock::{LockContents, Owner};
    use crate::types::manifest::PackVersion;
    use crate::types::name::{BinaryName, ItemName, PackName, ToolName};
    use crate::types::op::{CoreSkip, ItemId, OpPath, Origin, SkipReason, SkipSubject};
    use crate::types::plan::{Action, Skipped};
    use crate::types::project::SubPath;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn step(path: &str) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(path)?,
            action: Action::Create,
            owner: None,
            origin: None,
            new: Some(Vec::new()),
            old: None,
            decided_on: None,
        })
    }

    fn lock() -> Result<Lockfile, Box<dyn std::error::Error>> {
        Lockfile::new(Path::new("agentstacks.lock"), LockContents::default())
            .map_err(|refusals| format!("{refusals:?}").into())
    }

    fn paths(plan: &Plan) -> Vec<&str> {
        plan.steps().iter().map(|step| step.path.as_str()).collect()
    }

    #[test]
    fn plan_orders_steps_by_path() -> TestResult {
        let plan = Plan::new(
            vec![step("b")?, step("a")?],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            lock()?,
            Vec::new(),
        );
        assert_eq!(paths(&plan), ["a", "b"]);
        Ok(())
    }

    #[test]
    fn plan_keeps_every_part() -> TestResult {
        let skips = vec![
            Skipped::Extension {
                item: ItemId::agent(&PackName::new("core")?, &ItemName::new("coder")?),
                reason: SkipReason::new("not for this tool")?,
                origin: Origin::Tool {
                    tool: ToolName::new("opencode")?,
                },
            },
            Skipped::Core(CoreSkip::new(
                SkipSubject::Item(ItemId::agent(
                    &PackName::new("core")?,
                    &ItemName::new("reviewer")?,
                )),
                SkipReason::new("claude gave no value for {{var home}}")?,
            )),
        ];
        let binaries = vec![BinaryCheck {
            name: BinaryName::new("git")?,
            origin: Origin::Core {
                pack: PackName::new("core")?,
            },
        }];
        let notes = vec![Note::UnusedNative {
            dir: ToolName::new("cluade")?,
            tool: ToolName::new("claude")?,
        }];
        let airsl = AirslPin {
            locked: Some(PackVersion::new("0.1.4")?),
            installed: PackVersion::new("0.1.5")?,
        };
        let old_files = vec![FileEntry::new(
            SubPath::new("a")?,
            ContentHash::of_bytes(b"a"),
            Owner::new("core")?,
        )];
        let plan = Plan::new(
            vec![step("a")?],
            skips.clone(),
            binaries.clone(),
            notes.clone(),
            Some(airsl.clone()),
            lock()?,
            old_files.clone(),
        );
        assert_eq!(plan.skips(), skips);
        let tools: Vec<Option<String>> = plan
            .skips()
            .iter()
            .map(|skip| skip.tool().map(ToString::to_string))
            .collect();
        assert_eq!(tools, [Some("opencode".to_owned()), None]);
        assert_eq!(plan.binaries(), binaries);
        assert_eq!(plan.notes(), notes);
        assert_eq!(plan.airsl(), Some(&airsl));
        assert_eq!(plan.lock(), &lock()?);
        assert_eq!(plan.old_files(), old_files);
        Ok(())
    }
}
