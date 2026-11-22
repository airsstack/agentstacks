//! Who proposed an operation, and where.
//!
//! A refusal that names an earlier proposal needs to say whose it was: a line in an extension
//! file, another tool, or the core itself.
//!
//! # Responsibilities
//!
//! - [`Origin`] is an extension's call at a line of one of its files, a pack's root file the
//!   core writes, or the hook runtime the core writes for a tool.
//! - It gives the location an extension call is reported at, and the same place without the
//!   tool in front, for a message that names an earlier call.
//!
//! # Non-responsibilities
//!
//! Deciding what an origin means for the owner of a file belongs to the owner rule.

use std::num::NonZeroUsize;

use crate::types::name::{PackName, ToolName};
use crate::types::report::Location;
use crate::types::tree::TreePath;

/// Who proposed an operation.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{PackName, ToolName};
/// use agentstacks::types::op::Origin;
///
/// let origin = Origin::Tool { tool: ToolName::new("claude")? };
/// assert_eq!(origin.tool().map(ToString::to_string).as_deref(), Some("claude"));
/// assert_eq!(origin.location(), None);
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A call in a file of a tool's extension.
    Extension {
        /// The extension's tool.
        tool: ToolName,
        /// The file inside the extension (`adapter.lua`, `lib/settings.lua`).
        file: TreePath,
        /// The line of the call.
        line: NonZeroUsize,
    },
    /// A root file of a pack, or a binary the pack requires, which the core handles.
    Core {
        /// The pack.
        pack: PackName,
    },
    /// The hook runtime the core writes for a tool.
    Tool {
        /// The tool.
        tool: ToolName,
    },
}

impl Origin {
    /// Where the call is reported: `<tool>/<file>:<line>`. `None` for the core.
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        match self {
            Self::Extension { tool, file, line } => {
                Some(Location::at_line(format!("{tool}/{file}"), *line))
            }
            Self::Core { .. } | Self::Tool { .. } => None,
        }
    }

    /// Where the call is, inside its extension: `<file>:<line>`. `None` for the core.
    #[must_use]
    pub fn site(&self) -> Option<Location> {
        match self {
            Self::Extension { file, line, .. } => Some(Location::at_line(file.as_str(), *line)),
            Self::Core { .. } | Self::Tool { .. } => None,
        }
    }

    /// The tool of an extension call or of the runtime write.
    #[must_use]
    pub const fn tool(&self) -> Option<&ToolName> {
        match self {
            Self::Extension { tool, .. } | Self::Tool { tool } => Some(tool),
            Self::Core { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn line(n: usize) -> Result<NonZeroUsize, Box<dyn std::error::Error>> {
        Ok(NonZeroUsize::new(n).ok_or("a line starts at 1")?)
    }

    fn extension(file: &str, n: usize) -> Result<Origin, Box<dyn std::error::Error>> {
        Ok(Origin::Extension {
            tool: ToolName::new("claude")?,
            file: TreePath::from_parts(&file.split('/').collect::<Vec<_>>()),
            line: line(n)?,
        })
    }

    #[test]
    fn origin_locates_an_extension_op() -> TestResult {
        let adapter = extension("adapter.lua", 12)?;
        assert_eq!(
            adapter.location().map(|l| l.to_string()).as_deref(),
            Some("claude/adapter.lua:12")
        );
        assert_eq!(
            adapter.site().map(|l| l.to_string()).as_deref(),
            Some("adapter.lua:12")
        );
        let module = extension("lib/settings.lua", 3)?;
        assert_eq!(
            module.location().map(|l| l.to_string()).as_deref(),
            Some("claude/lib/settings.lua:3")
        );
        assert_eq!(
            module.tool().map(ToString::to_string).as_deref(),
            Some("claude")
        );
        Ok(())
    }

    #[test]
    fn origin_of_the_core_has_no_location() -> TestResult {
        let core = Origin::Core {
            pack: PackName::new("core")?,
        };
        let runtime = Origin::Tool {
            tool: ToolName::new("claude")?,
        };
        for origin in [&core, &runtime] {
            assert_eq!(origin.location(), None);
            assert_eq!(origin.site(), None);
        }
        assert_eq!(core.tool(), None);
        assert_eq!(
            runtime.tool().map(ToString::to_string).as_deref(),
            Some("claude")
        );
        Ok(())
    }
}
