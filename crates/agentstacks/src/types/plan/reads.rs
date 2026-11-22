//! What the reads of one tool returned, and which paths are shared with the user.
//!
//! # Responsibilities
//!
//! - [`ReadOutcome`] is what one read of a path returned: absent, or bytes with their hash.
//! - [`Reads`] is one tool's record of its reads in one run, keyed by the normalised path. The
//!   first read of a path is the one kept.
//! - [`Reads::shared`] is the one rule for a shared path: one the tool declared it reads and did
//!   read in this run.
//!
//! # Non-responsibilities
//!
//! Performing a read and deciding the step for a shared path belong to the extension host and the
//! decide functions.

use std::collections::BTreeMap;

use crate::types::op::OpPath;
use crate::types::tree::ContentHash;

/// What one read of a path returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    /// The file did not exist.
    Absent,
    /// The file existed with this hash.
    Bytes(ContentHash),
}

/// One tool's reads in one run.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::OpPath;
/// use agentstacks::types::plan::{ReadOutcome, Reads};
///
/// let settings = OpPath::new(".claude/settings.json")?;
/// let mut reads = Reads::default();
/// reads.record(settings.clone(), ReadOutcome::Absent);
/// assert_eq!(reads.shared(&settings, &[settings.clone()]), Some(ReadOutcome::Absent));
/// assert_eq!(reads.shared(&settings, &[]), None);
/// # Ok::<(), agentstacks::types::op::PathShape>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reads {
    outcomes: BTreeMap<OpPath, ReadOutcome>,
}

impl Reads {
    /// Records that reading `path` returned `outcome`; a path already recorded keeps its first
    /// outcome.
    pub fn record(&mut self, path: OpPath, outcome: ReadOutcome) {
        self.outcomes.entry(path).or_insert(outcome);
    }

    /// The outcome recorded for `path` when it is in `declared` and was read in this run.
    #[must_use]
    pub fn shared(&self, path: &OpPath, declared: &[OpPath]) -> Option<ReadOutcome> {
        if declared.contains(path) {
            self.outcomes.get(path).cloned()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn path(text: &str) -> Result<OpPath, Box<dyn std::error::Error>> {
        Ok(OpPath::new(text)?)
    }

    #[test]
    fn shared_needs_declared_and_read() -> TestResult {
        let settings = path(".claude/settings.json")?;
        let other = path(".claude/other.json")?;
        let mut reads = Reads::default();
        let hash = ContentHash::of_bytes(b"{}");
        reads.record(settings.clone(), ReadOutcome::Bytes(hash.clone()));
        reads.record(other.clone(), ReadOutcome::Absent);

        let declared = [settings.clone(), path(".claude/unread.json")?];
        assert_eq!(
            reads.shared(&settings, &declared),
            Some(ReadOutcome::Bytes(hash))
        );
        assert_eq!(reads.shared(&path(".claude/unread.json")?, &declared), None);
        assert_eq!(reads.shared(&other, &declared), None);
        Ok(())
    }

    #[test]
    fn reads_keeps_the_first_outcome_of_a_path() -> TestResult {
        let settings = path(".claude/settings.json")?;
        let mut reads = Reads::default();
        reads.record(settings.clone(), ReadOutcome::Absent);
        reads.record(
            settings.clone(),
            ReadOutcome::Bytes(ContentHash::of_bytes(b"x")),
        );
        assert_eq!(
            reads.shared(&settings, std::slice::from_ref(&settings)),
            Some(ReadOutcome::Absent)
        );
        Ok(())
    }

    #[test]
    fn reads_key_is_the_normalised_path() -> TestResult {
        let mut reads = Reads::default();
        reads.record(path("./.claude//settings.json")?, ReadOutcome::Absent);
        let plain = path(".claude/settings.json")?;
        assert_eq!(
            reads.shared(&plain, std::slice::from_ref(&plain)),
            Some(ReadOutcome::Absent)
        );
        Ok(())
    }
}
