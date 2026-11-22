//! The step for a locked path that is no longer proposed.
//!
//! # Responsibilities
//!
//! - [`decide_unproposed`] answers with a delete when the file is untouched since the last
//!   apply, a forget when there is nothing to delete or the path is now a symlink, and
//!   otherwise the refusal located at the path.
//!
//! # Non-responsibilities
//!
//! Reading the disk and the lock, and the order in which paths are decided, belong to the plan
//! flow.

use crate::types::lock::FileEntry;
use crate::types::op::OpPath;
use crate::types::plan::decide::proposed::refuse;
use crate::types::plan::{Action, DiskState, Link, Resolution, Step, StepError};
use crate::types::report::Diagnostic;

/// Decides the step for `path`, which the lock records as `entry` and nothing proposes now.
///
/// A path that is now a symlink is forgotten before any other check, so the link is never
/// followed to delete what it points at.
///
/// # Errors
///
/// The refusal, located at the path: it resolves outside the project, it changed since the last
/// apply, or it is not a regular file.
pub fn decide_unproposed(
    path: OpPath,
    entry: &FileEntry,
    resolution: &Resolution,
) -> Result<Step, Diagnostic> {
    let state = match resolution {
        Resolution::Inside {
            link: Link::Symlink,
            ..
        }
        | Resolution::Outside {
            link: Link::Symlink,
        } => return Ok(step(path, entry, Action::Forget, None)),
        Resolution::Outside { link: Link::Direct } => {
            return Err(refuse(&path, StepError::Outside));
        }
        Resolution::Inside { state, .. } => state,
    };
    match state {
        DiskState::Absent => Ok(step(path, entry, Action::Forget, None)),
        DiskState::File(file) if file.hash() == entry.hash() => {
            Ok(step(path, entry, Action::Delete, Some(state.clone())))
        }
        DiskState::File(_) => Err(refuse(&path, StepError::ChangedNotDeleting)),
        DiskState::NotRegular => Err(refuse(&path, StepError::NotRegular)),
    }
}

/// A step that carries no bytes and no origin, owned by the entry's owner.
fn step(path: OpPath, entry: &FileEntry, action: Action, decided_on: Option<DiskState>) -> Step {
    Step {
        path,
        action,
        owner: Some(entry.by().clone()),
        origin: None,
        new: None,
        old: None,
        decided_on,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::lock::Owner;
    use crate::types::plan::DiskFile;
    use crate::types::project::SubPath;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const X: &str = ".claude/x";

    fn entry(path: &str, bytes: &[u8]) -> Result<FileEntry, Box<dyn std::error::Error>> {
        Ok(FileEntry::new(
            SubPath::new(path)?,
            ContentHash::of_bytes(bytes),
            Owner::new("team-lint")?,
        ))
    }

    fn file(bytes: &[u8]) -> DiskState {
        DiskState::File(DiskFile::new(bytes.to_vec(), 0o644))
    }

    fn inside(state: DiskState) -> Resolution {
        Resolution::Inside {
            link: Link::Direct,
            state,
        }
    }

    fn decide(
        path: &str,
        locked: &[u8],
        resolution: &Resolution,
    ) -> Result<Result<Step, String>, Box<dyn std::error::Error>> {
        Ok(
            decide_unproposed(OpPath::new(path)?, &entry(path, locked)?, resolution)
                .map_err(|refusal| refusal.to_string()),
        )
    }

    #[test]
    fn unproposed_symlink_forgets_first() -> TestResult {
        let links = [
            Resolution::Inside {
                link: Link::Symlink,
                state: file(b"old"),
            },
            Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::NotRegular,
            },
            Resolution::Outside {
                link: Link::Symlink,
            },
        ];
        for resolution in &links {
            let step = decide(X, b"old", resolution)??;
            assert_eq!(step.action, Action::Forget);
            assert_eq!(step.decided_on, None);
        }
        Ok(())
    }

    #[test]
    fn unproposed_absent_forgets() -> TestResult {
        let step = decide(X, b"old", &inside(DiskState::Absent))??;
        assert_eq!(step.action, Action::Forget);
        assert_eq!(step.decided_on, None);
        assert_eq!(step.new, None);
        assert_eq!(step.origin, None);
        Ok(())
    }

    #[test]
    fn unproposed_hash_equal_deletes() -> TestResult {
        let state = file(b"old");
        let step = decide(X, b"old", &inside(state.clone()))??;
        assert_eq!(step.action, Action::Delete);
        assert_eq!(step.owner, Some(Owner::new("team-lint")?));
        assert_eq!(step.decided_on, Some(state));
        assert_eq!(step.new, None);
        Ok(())
    }

    #[test]
    fn unproposed_hash_differs_is_refused() -> TestResult {
        let refusal = decide(X, b"old", &inside(file(b"edited")))?
            .err()
            .ok_or("the delete was accepted")?;
        assert_eq!(
            refusal,
            ".claude/x: changed since the last apply; not deleting it"
        );
        Ok(())
    }

    #[test]
    fn unproposed_not_regular_is_refused() -> TestResult {
        let refusal = decide(X, b"old", &inside(DiskState::NotRegular))?
            .err()
            .ok_or("the delete was accepted")?;
        assert_eq!(refusal, ".claude/x: is not a regular file");
        Ok(())
    }

    #[test]
    fn unproposed_outside_is_refused() -> TestResult {
        let refusal = decide(X, b"old", &Resolution::Outside { link: Link::Direct })?
            .err()
            .ok_or("the delete was accepted")?;
        assert_eq!(refusal, ".claude/x: resolves outside the project");
        Ok(())
    }

    #[test]
    fn example_removed_pack_deletes() -> TestResult {
        let path = ".claude/skills/lint/SKILL.md";
        let step = decide(path, b"lint skill", &inside(file(b"lint skill")))??;
        assert_eq!(step.action, Action::Delete);
        assert_eq!(step.path.as_str(), path);
        Ok(())
    }
}
