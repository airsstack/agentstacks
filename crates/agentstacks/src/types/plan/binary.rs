//! A binary requirement that was found on `PATH`.
//!
//! # Responsibilities
//!
//! - [`BinaryCheck`] is one passed check: the binary and who required it. A binary required by a
//!   pack and by an extension appears once for each.
//!
//! # Non-responsibilities
//!
//! A binary that is not found is a refusal, never a check.

use crate::types::name::BinaryName;
use crate::types::op::Origin;

/// One binary requirement that passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryCheck {
    /// The binary found on `PATH`.
    pub name: BinaryName,
    /// Who required it.
    pub origin: Origin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::{PackName, ToolName};

    #[test]
    fn one_check_per_requirement() -> Result<(), Box<dyn std::error::Error>> {
        let by_pack = BinaryCheck {
            name: BinaryName::new("git")?,
            origin: Origin::Core {
                pack: PackName::new("core")?,
            },
        };
        let by_tool = BinaryCheck {
            origin: Origin::Tool {
                tool: ToolName::new("claude")?,
            },
            ..by_pack.clone()
        };
        assert_eq!(by_pack.name, by_tool.name);
        assert_ne!(by_pack, by_tool);
        Ok(())
    }
}
