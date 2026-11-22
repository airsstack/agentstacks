//! Every refusal of a file step.
//!
//! The enum's `Display` is the text after the location: the project-relative path the step is
//! about, or `agentstacks.toml` and the key of a `path` source.
//!
//! # Responsibilities
//!
//! - [`StepError`] has one variant per refusal of a path on disk: its kind, whether it changed,
//!   whether agentstacks wrote it, and whether it resolves inside the project.
//!
//! # Non-responsibilities
//!
//! Deciding the step belongs to the code that compares the operation, the lock and the disk.

/// A file step refused.
///
/// # Examples
///
/// ```
/// use agentstacks::types::plan::StepError;
///
/// assert_eq!(StepError::Changed.to_string(), "changed since the last apply");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StepError {
    /// The path exists and is not a regular file.
    #[error("is not a regular file")]
    NotRegular,
    /// The file differs from what the last apply wrote.
    #[error("changed since the last apply")]
    Changed,
    /// The path exists and the lock does not say agentstacks wrote it.
    #[error("exists and was not written by agentstacks")]
    NotOurs,
    /// A file to delete that differs from what the last apply wrote.
    #[error("changed since the last apply; not deleting it")]
    ChangedNotDeleting,
    /// A file an extension read that changed before the plan was made.
    #[error("changed after it was read; run plan again")]
    ChangedAfterRead,
    /// The path resolves outside the project.
    #[error("resolves outside the project")]
    Outside,
    /// The file changed between the plan and the apply.
    #[error("changed after plan; run plan again")]
    ChangedAfterPlan,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::{Diagnostic, Location};

    fn at(path: &str, error: StepError) -> String {
        Diagnostic::new(Location::file(path), error).to_string()
    }

    #[test]
    fn step_error_not_regular() {
        assert_eq!(
            at(".claude/x", StepError::NotRegular),
            ".claude/x: is not a regular file"
        );
    }

    #[test]
    fn step_error_changed() {
        assert_eq!(
            at(".claude/skills/status/SKILL.md", StepError::Changed),
            ".claude/skills/status/SKILL.md: changed since the last apply"
        );
    }

    #[test]
    fn step_error_not_ours() {
        assert_eq!(
            at(".claude/agents/coder.md", StepError::NotOurs),
            ".claude/agents/coder.md: exists and was not written by agentstacks"
        );
    }

    #[test]
    fn step_error_changed_not_deleting() {
        assert_eq!(
            at(".claude/x", StepError::ChangedNotDeleting),
            ".claude/x: changed since the last apply; not deleting it"
        );
    }

    #[test]
    fn step_error_changed_after_read() {
        assert_eq!(
            at(".claude/settings.json", StepError::ChangedAfterRead),
            ".claude/settings.json: changed after it was read; run plan again"
        );
    }

    #[test]
    fn step_error_outside() {
        assert_eq!(
            at(".claude/x", StepError::Outside),
            ".claude/x: resolves outside the project"
        );
        let located = Diagnostic::new(
            Location::at_key("agentstacks.toml", "packs.team-lint.path"),
            StepError::Outside,
        );
        assert_eq!(
            located.to_string(),
            "agentstacks.toml: packs.team-lint.path: resolves outside the project"
        );
    }

    #[test]
    fn step_error_changed_after_plan() {
        assert_eq!(
            at(".claude/x", StepError::ChangedAfterPlan),
            ".claude/x: changed after plan; run plan again"
        );
    }
}
