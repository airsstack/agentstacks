//! What a step does to its path.
//!
//! # Responsibilities
//!
//! - [`Action`] names the six outcomes for a path.
//! - [`Action::writes`] and [`Action::touches_disk`] say which outcomes the apply stages act on.
//!
//! # Non-responsibilities
//!
//! Choosing the action belongs to the decide functions; performing it belongs to the executor.

/// The action of one step.
///
/// # Examples
///
/// ```
/// use agentstacks::types::plan::Action;
///
/// assert!(Action::Create.writes());
/// assert!(Action::Delete.touches_disk());
/// assert!(!Action::Forget.touches_disk());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The file does not exist and is written.
    Create,
    /// The file exists and is rewritten with new content.
    Update,
    /// A file the lock records is missing and is written again.
    Restore,
    /// The file already holds the new content.
    Unchanged,
    /// A file a removed pack wrote is deleted.
    Delete,
    /// The lock entry is dropped and the disk is left alone.
    Forget,
}

impl Action {
    /// Whether the step stages new content: create, update and restore.
    #[must_use]
    pub const fn writes(self) -> bool {
        matches!(self, Self::Create | Self::Update | Self::Restore)
    }

    /// Whether apply changes the disk for the step: the writes and delete.
    #[must_use]
    pub const fn touches_disk(self) -> bool {
        self.writes() || matches!(self, Self::Delete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Action; 6] = [
        Action::Create,
        Action::Update,
        Action::Restore,
        Action::Unchanged,
        Action::Delete,
        Action::Forget,
    ];

    #[test]
    fn action_writes_only_for_create_update_restore() {
        let writing: Vec<Action> = ALL.into_iter().filter(|a| a.writes()).collect();
        assert_eq!(writing, [Action::Create, Action::Update, Action::Restore]);
    }

    #[test]
    fn action_touches_disk_except_unchanged_and_forget() {
        let idle: Vec<Action> = ALL.into_iter().filter(|a| !a.touches_disk()).collect();
        assert_eq!(idle, [Action::Unchanged, Action::Forget]);
    }
}
