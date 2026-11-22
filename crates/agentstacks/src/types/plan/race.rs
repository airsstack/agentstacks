//! The check apply makes before it acts: has the disk changed since the plan was made?
//!
//! # Responsibilities
//!
//! - [`race`] compares where a step's path leads now with the state the step was decided on.
//!   It answers only for a step that touches disk, so a forget of a dangling or outside symlink
//!   never blocks apply. A change of mode alone is not a change.
//! - [`Race::refusal`] is the refusal for a path that changed or now leads outside the project.
//!
//! # Non-responsibilities
//!
//! Reading the disk, and which steps are checked and in which order, belong to the apply flow.

use crate::types::op::OpPath;
use crate::types::plan::{DiskState, Resolution, Step, StepError};
use crate::types::report::{Diagnostic, Location};

/// How a step's path compares with the state it was decided on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Race {
    /// Nothing changed.
    Same,
    /// The path is absent, present or another kind of file than before, or its content differs.
    Changed,
    /// The path now resolves outside the project.
    Outside,
}

/// Compares `fresh` with the state `step` was decided on; `None` for a step that does not touch
/// disk.
#[must_use]
pub fn race(step: &Step, fresh: &Resolution) -> Option<Race> {
    if !step.action.touches_disk() {
        return None;
    }
    let Resolution::Inside { state, .. } = fresh else {
        return Some(Race::Outside);
    };
    let same = match (&step.decided_on, state) {
        (Some(DiskState::Absent), DiskState::Absent) => true,
        (Some(DiskState::File(decided)), DiskState::File(now)) => decided.hash() == now.hash(),
        _ => false,
    };
    Some(if same { Race::Same } else { Race::Changed })
}

impl Race {
    /// The refusal for `path`; `None` when nothing changed.
    #[must_use]
    pub fn refusal(&self, path: &OpPath) -> Option<Diagnostic> {
        let error = match self {
            Self::Same => return None,
            Self::Changed => StepError::ChangedAfterPlan,
            Self::Outside => StepError::Outside,
        };
        Some(Diagnostic::new(Location::file(path.as_str()), error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::plan::{Action, DiskFile, Link};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn file(bytes: &[u8]) -> DiskState {
        DiskState::File(DiskFile::new(bytes.to_vec(), 0o644))
    }

    fn step(
        action: Action,
        decided_on: Option<DiskState>,
    ) -> Result<Step, Box<dyn std::error::Error>> {
        Ok(Step {
            path: OpPath::new(".claude/x")?,
            action,
            owner: None,
            origin: None,
            new: None,
            old: None,
            decided_on,
        })
    }

    fn inside(state: DiskState) -> Resolution {
        Resolution::Inside {
            link: Link::Direct,
            state,
        }
    }

    #[test]
    fn race_same_file_and_same_absence() -> TestResult {
        let on_file = step(Action::Update, Some(file(b"a")))?;
        assert_eq!(race(&on_file, &inside(file(b"a"))), Some(Race::Same));
        let on_absent = step(Action::Create, Some(DiskState::Absent))?;
        assert_eq!(
            race(&on_absent, &inside(DiskState::Absent)),
            Some(Race::Same)
        );
        Ok(())
    }

    #[test]
    fn race_changed_hash_or_presence() -> TestResult {
        let on_file = step(Action::Update, Some(file(b"a")))?;
        let on_absent = step(Action::Create, Some(DiskState::Absent))?;
        let cases = [
            (&on_file, inside(file(b"b"))),
            (&on_absent, inside(file(b"a"))),
            (&on_file, inside(DiskState::Absent)),
            (&on_file, inside(DiskState::NotRegular)),
        ];
        for (decided, fresh) in cases {
            let found = race(decided, &fresh).ok_or("a step that touches disk")?;
            assert_eq!(found, Race::Changed);
            assert_eq!(
                found
                    .refusal(&decided.path)
                    .map(|d| d.to_string())
                    .as_deref(),
                Some(".claude/x: changed after plan; run plan again")
            );
        }
        Ok(())
    }

    #[test]
    fn race_outside_is_refused() -> TestResult {
        let decided = step(Action::Update, Some(file(b"a")))?;
        let fresh = Resolution::Outside {
            link: Link::Symlink,
        };
        let found = race(&decided, &fresh).ok_or("a step that touches disk")?;
        assert_eq!(found, Race::Outside);
        assert_eq!(
            found
                .refusal(&decided.path)
                .map(|d| d.to_string())
                .as_deref(),
            Some(".claude/x: resolves outside the project")
        );
        Ok(())
    }

    #[test]
    fn race_shared_step_compares_with_the_read() -> TestResult {
        let shared = step(Action::Create, Some(DiskState::Absent))?;
        assert_eq!(race(&shared, &inside(file(b"user"))), Some(Race::Changed));
        Ok(())
    }

    #[test]
    fn race_skips_steps_that_do_not_touch_disk() -> TestResult {
        let forget = step(Action::Forget, None)?;
        let dangling = Resolution::Inside {
            link: Link::Symlink,
            state: DiskState::NotRegular,
        };
        let outside = Resolution::Outside {
            link: Link::Symlink,
        };
        assert_eq!(race(&forget, &dangling), None);
        assert_eq!(race(&forget, &outside), None);
        let unchanged = step(Action::Unchanged, Some(file(b"a")))?;
        assert_eq!(race(&unchanged, &inside(DiskState::Absent)), None);
        assert!(Race::Same.refusal(&OpPath::new(".claude/x")?).is_none());
        Ok(())
    }
}
