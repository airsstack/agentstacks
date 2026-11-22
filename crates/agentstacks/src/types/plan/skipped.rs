//! A skip in the plan, with who made it.
//!
//! # Responsibilities
//!
//! - [`Skipped`] is one skip the plan shows: an extension's skip, which keeps its [`Origin`] so
//!   the plan can say which tool skipped the item, or a skip the core made.
//!
//! # Non-responsibilities
//!
//! Making the skips belongs to the plan flow, and the reasons the core gives belong to the
//! missing-value rules.

use crate::types::name::ToolName;
use crate::types::op::{CoreSkip, ItemId, Origin, SkipReason, SkipSubject};

/// One skip in a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skipped {
    /// A skip an extension proposed.
    Extension {
        /// The item skipped.
        item: ItemId,
        /// Why.
        reason: SkipReason,
        /// Who proposed it.
        origin: Origin,
    },
    /// A skip the core made. Its reason names the tool.
    Core(CoreSkip),
}

impl Skipped {
    /// What is skipped.
    #[must_use]
    pub fn subject(&self) -> SkipSubject {
        match self {
            Self::Extension { item, .. } => SkipSubject::Item(item.clone()),
            Self::Core(skip) => skip.subject().clone(),
        }
    }

    /// Why it is skipped.
    #[must_use]
    pub const fn reason(&self) -> &SkipReason {
        match self {
            Self::Extension { reason, .. } => reason,
            Self::Core(skip) => skip.reason(),
        }
    }

    /// Who proposed the skip. `None` for a skip the core made.
    #[must_use]
    pub const fn origin(&self) -> Option<&Origin> {
        match self {
            Self::Extension { origin, .. } => Some(origin),
            Self::Core(_) => None,
        }
    }

    /// The tool of an extension's skip. `None` for a skip the core made.
    #[must_use]
    pub const fn tool(&self) -> Option<&ToolName> {
        match self {
            Self::Extension { origin, .. } => origin.tool(),
            Self::Core(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::name::{ItemName, PackName};
    use crate::types::tree::TreePath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn item() -> Result<ItemId, Box<dyn std::error::Error>> {
        Ok(ItemId::agent(
            &PackName::new("core")?,
            &ItemName::new("coder")?,
        ))
    }

    #[test]
    fn an_extension_skip_keeps_its_origin() -> TestResult {
        let origin = Origin::Extension {
            tool: ToolName::new("claude")?,
            file: TreePath::from_parts(&["adapter.lua"]),
            line: NonZeroUsize::new(7).ok_or("a line starts at 1")?,
        };
        let skip = Skipped::Extension {
            item: item()?,
            reason: SkipReason::new("no agents")?,
            origin: origin.clone(),
        };
        assert_eq!(skip.origin(), Some(&origin));
        assert_eq!(
            skip.tool().map(ToString::to_string).as_deref(),
            Some("claude")
        );
        assert_eq!(skip.subject(), SkipSubject::Item(item()?));
        assert_eq!(skip.reason().as_str(), "no agents");
        Ok(())
    }

    #[test]
    fn a_core_skip_has_no_origin() -> TestResult {
        let subject = SkipSubject::Item(item()?);
        let skip = Skipped::Core(CoreSkip::new(subject.clone(), SkipReason::new("why")?));
        assert_eq!(skip.origin(), None);
        assert_eq!(skip.tool(), None);
        assert_eq!(skip.subject(), subject);
        assert_eq!(skip.reason().as_str(), "why");
        Ok(())
    }
}
