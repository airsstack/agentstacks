//! A skip, and the reason it carries.
//!
//! An item a tool cannot express is skipped with a reason, never dropped silently. The
//! extension skips what it cannot express, and the core skips what a tool gave no value for.
//!
//! # Responsibilities
//!
//! - [`SkipReason`] is a reason that is not blank, kept as written.
//! - [`SkipSubject`] is what a core skip is about: an item, or a root file of a pack, which is
//!   not an item.
//! - [`CoreSkip`] is one skip the core made.
//!
//! # Non-responsibilities
//!
//! The reasons the core gives, and when it skips, belong to the missing-value rules.

use std::fmt;

use crate::types::name::PackName;
use crate::types::op::{ApiFunction, ItemId, OpError};
use crate::types::tree::TreePath;

/// Why an item is skipped.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::SkipReason;
///
/// assert_eq!(SkipReason::new("not for this tool")?.as_str(), "not for this tool");
/// assert!(SkipReason::new("  ").is_err());
/// # Ok::<(), agentstacks::types::op::OpError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipReason(String);

impl SkipReason {
    /// A reason from the extension's `skip` call, kept as written.
    ///
    /// # Errors
    ///
    /// [`OpError::Empty`] for the field `reason` when `text` is empty or only whitespace.
    pub fn new(text: &str) -> Result<Self, OpError> {
        if text.trim().is_empty() {
            return Err(OpError::Empty {
                function: ApiFunction::Skip,
                field: "reason".to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// A reason the core words itself, which is never blank.
    pub(in crate::types::op) const fn core(text: String) -> Self {
        Self(text)
    }

    /// The reason as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a core skip is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipSubject {
    /// An item of a pack.
    Item(ItemId),
    /// A root file of a pack, which is not an item.
    RootFile {
        /// The pack.
        pack: PackName,
        /// The file, relative to the pack.
        rel: TreePath,
    },
}

/// One skip the core made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreSkip {
    subject: SkipSubject,
    reason: SkipReason,
}

impl CoreSkip {
    /// The skip of `subject` for `reason`.
    #[must_use]
    pub const fn new(subject: SkipSubject, reason: SkipReason) -> Self {
        Self { subject, reason }
    }

    /// What is skipped.
    #[must_use]
    pub const fn subject(&self) -> &SkipSubject {
        &self.subject
    }

    /// Why it is skipped.
    #[must_use]
    pub const fn reason(&self) -> &SkipReason {
        &self.reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn skip_reason_refuses_empty_and_blank() -> TestResult {
        for text in ["", "   ", "\t\n"] {
            let refused = SkipReason::new(text).err().ok_or("a blank reason")?;
            assert_eq!(refused.to_string(), "skip: reason: empty", "{text:?}");
        }
        let kept = SkipReason::new("not for this tool")?;
        assert_eq!(kept.as_str(), "not for this tool");
        assert_eq!(kept.to_string(), "not for this tool");
        Ok(())
    }

    #[test]
    fn core_skip_holds_its_subject_and_reason() -> TestResult {
        let subject = SkipSubject::RootFile {
            pack: PackName::new("core")?,
            rel: TreePath::from_parts(&["hooks", "x.lua"]),
        };
        let skip = CoreSkip::new(subject.clone(), SkipReason::new("why")?);
        assert_eq!(skip.subject(), &subject);
        assert_eq!(skip.reason().as_str(), "why");
        Ok(())
    }
}
