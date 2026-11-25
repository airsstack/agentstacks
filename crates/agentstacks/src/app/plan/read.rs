//! Step 1 of the plan flow: reading `agentstacks.toml` and `agentstacks.lock`.
//!
//! # Responsibilities
//!
//! - [`Inputs`] is the flow's carrier for the parsed project file and the lock, when there is one.
//! - [`read_inputs`] reads both texts through the [`Workspace`] port and parses them with the
//!   built readers. A read failure is located at the file it happened on. Parse refusals pass
//!   through unchanged, and when both files are refused both sets are returned. A project file
//!   written for the `0.x` releases parses but is refused here, naming `update`, so `plan` and
//!   `apply` never go on with one.
//!
//! # Non-responsibilities
//!
//! A missing lock is no refusal: there is simply no lock, and the rules that reconcile the project
//! with it treat every entry as new. Whether the lock records `[airsl]` rightly is checked on the
//! lock the plan builds, never on the one read here.

use std::path::Path;

use crate::port::Workspace;
use crate::types::ApiCheck;
use crate::types::lock::Lockfile;
use crate::types::project::{PROJECT_FILE, ProjectError, ProjectFile};
use crate::types::report::{Diagnostic, Diagnostics, LegacyProject, Location};

/// The file the lock is read from.
pub(super) const LOCK_FILE: &str = "agentstacks.lock";

/// What step 1 read: the project, and the lock when one exists.
pub(super) struct Inputs {
    /// The parsed `agentstacks.toml`.
    pub(super) project: ProjectFile,
    /// The parsed `agentstacks.lock`; `None` when there is none.
    pub(super) lock: Option<Lockfile>,
}

/// Reads and parses the project file and the lock.
///
/// # Errors
///
/// Every refusal of both files: an I/O failure of a read, what parsing refused, or a project file
/// written for the legacy `api` (`api 0 is from agentstacks 0.x (run agentstacks update)`).
pub(super) fn read_inputs<W: Workspace>(workspace: &W) -> Result<Inputs, Diagnostics> {
    let mut refusals = Diagnostics::new();
    let project = match workspace.project_text() {
        Ok(text) => ProjectFile::parse(Path::new(PROJECT_FILE), &text)
            .map_err(|found| refusals.extend(found))
            .ok()
            .filter(|project| match project.api() {
                ApiCheck::Legacy => {
                    refusals.push(Diagnostic::new(
                        Location::file(PROJECT_FILE),
                        ProjectError::from(LegacyProject),
                    ));
                    false
                }
                ApiCheck::Current | ApiCheck::Unsupported(_) => true,
            }),
        Err(error) => {
            refusals.push(Diagnostic::new(Location::file(PROJECT_FILE), error));
            None
        }
    };
    let lock = match workspace.lock_text() {
        Ok(Some(text)) => Lockfile::parse(Path::new(LOCK_FILE), &text)
            .map_err(|found| refusals.extend(found))
            .ok()
            .map(Some),
        Ok(None) => Some(None),
        Err(error) => {
            refusals.push(Diagnostic::new(Location::file(LOCK_FILE), error));
            None
        }
    };
    match (project, lock) {
        (Some(project), Some(lock)) => Ok(Inputs { project, lock }),
        _ => Err(refusals),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::testing::{Scenario, TestResult, shown};
    use crate::port::{FakeWorkspace, WorkspaceCall};
    use crate::types::name::{PackName, ToolName};
    use crate::types::tree::ContentHash;

    const PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";

    fn lock_text() -> String {
        let hash = ContentHash::of_bytes(b"x");
        format!(
            "api = 1\n\n[[pack]]\nname = \"core\"\nversion = \"0.1.0\"\npath = \"packs/core\"\ntree = \"{hash}\"\n"
        )
    }

    #[test]
    fn read_inputs_parses_the_project_and_the_lock() -> TestResult {
        let scenario = Scenario::new(PROJECT).lock(&lock_text());
        let inputs = read_inputs(&scenario.workspace).map_err(|found| shown(found).join("; "))?;
        assert!(
            inputs
                .project
                .tools()
                .contains_key(&ToolName::new("claude")?)
        );
        assert!(inputs.project.packs().contains_key(&PackName::new("core")?));
        let lock = inputs.lock.ok_or("the lock was read")?;
        assert_eq!(lock.packs().len(), 1);
        assert_eq!(
            lock.packs().first().map(|p| p.name().as_str()),
            Some("core")
        );
        Ok(())
    }

    #[test]
    fn read_inputs_refuses_a_legacy_project_and_still_reads_the_lock() -> TestResult {
        let legacy = PROJECT.replacen("api = 1", "api = 0", 1);
        assert_ne!(legacy, PROJECT);
        let scenario = Scenario::new(&legacy).lock("api = 2\n");
        let Err(found) = read_inputs(&scenario.workspace) else {
            return Err("a refusal".into());
        };
        assert_eq!(
            shown(found),
            [
                "agentstacks.lock: api 2 not supported (this agentstacks speaks 1)",
                "agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)",
            ]
        );
        Ok(())
    }

    #[test]
    fn read_inputs_without_a_lock_has_no_lock() -> TestResult {
        let scenario = Scenario::new(PROJECT);
        let inputs = read_inputs(&scenario.workspace).map_err(|found| shown(found).join("; "))?;
        assert!(inputs.lock.is_none());
        Ok(())
    }

    #[test]
    fn read_inputs_reports_an_unreadable_project_file_with_the_io_error() -> TestResult {
        let scenario = Scenario::new(PROJECT).workspace(|w| w.fail(WorkspaceCall::ProjectText));
        let found = read_inputs(&scenario.workspace).err().ok_or("a refusal")?;
        assert_eq!(
            shown(found),
            [format!("agentstacks.toml: {}", FakeWorkspace::denied())]
        );
        Ok(())
    }

    #[test]
    fn read_inputs_returns_project_and_lock_refusals_together() -> TestResult {
        let project = format!("foo = 1\n{PROJECT}");
        let lock = "api = 1\nmerge = 1\n";
        let scenario = Scenario::new(&project).lock(lock);
        let found = read_inputs(&scenario.workspace).err().ok_or("refusals")?;
        assert_eq!(
            shown(found),
            [
                "agentstacks.lock: merge: unknown key",
                "agentstacks.toml: foo: unknown key",
            ]
        );
        Ok(())
    }
}
