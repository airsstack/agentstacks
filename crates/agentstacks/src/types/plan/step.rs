//! One step of a plan: what happens to one path, and what it was decided on.
//!
//! # Responsibilities
//!
//! - [`Step`] holds the path, the [`Action`], who owns the file, who proposed it, the bytes to
//!   install, a shared file's current bytes for the diff, and the disk state the step was decided
//!   on, which apply checks again before it acts.
//!
//! # Non-responsibilities
//!
//! Deciding a step belongs to the decide functions, and acting on it belongs to the executor.

use crate::types::lock::Owner;
use crate::types::op::{OpPath, Origin};
use crate::types::plan::{Action, DiskState};

/// One path and what happens to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The project-relative path.
    pub path: OpPath,
    /// What happens to it.
    pub action: Action,
    /// Who wrote the file, for the lock. `None` for a file shared with the user, which has no
    /// single owner.
    pub owner: Option<Owner>,
    /// Who proposed the write. `None` for a delete or a forget, which the core makes from the lock.
    pub origin: Option<Origin>,
    /// The bytes to install. `None` for a delete or a forget.
    pub new: Option<Vec<u8>>,
    /// A shared file's current bytes, kept for the diff.
    pub old: Option<Vec<u8>>,
    /// The disk state the step was decided on, which apply checks again. `None` for a forget,
    /// which never touches disk.
    pub decided_on: Option<DiskState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_equality_covers_every_field() -> Result<(), Box<dyn std::error::Error>> {
        let step = Step {
            path: OpPath::new(".claude/x")?,
            action: Action::Create,
            owner: Some(Owner::new("core")?),
            origin: None,
            new: Some(b"new".to_vec()),
            old: None,
            decided_on: Some(DiskState::Absent),
        };
        assert_eq!(step.clone(), step);
        let other = Step {
            decided_on: None,
            ..step.clone()
        };
        assert_ne!(other, step);
        Ok(())
    }
}
