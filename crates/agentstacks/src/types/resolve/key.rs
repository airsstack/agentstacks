//! The key of one `[tools.*]` or `[packs.*]` entry in `agentstacks.toml`.
//!
//! It is separate from the names because a refusal about a source names the entry's key path
//! (`packs.core`, `tools.claude`), and one value must give that text, the entry's locations and the
//! order every per-entry answer follows.
//!
//! # Responsibilities
//!
//! - [`SourceKey`] is a pack name or a tool name, displayed as the key path of its entry.
//! - [`SourceKey::location`] and [`SourceKey::subdir_location`] are where a refusal about the
//!   entry, or about its `subdir`, is reported.
//! - It orders packs before tools, and by name within each, so a map keyed by it lists `packs.*`
//!   first and then `tools.*`.
//!
//! # Non-responsibilities
//!
//! Which entries exist is the project file's; this value only names one.

use std::fmt;

use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::project::PROJECT_FILE;
use crate::types::report::Location;

/// The key of a pack or tool entry.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
/// use agentstacks::types::resolve::SourceKey;
///
/// let key = SourceKey::Pack(PackName::new("core")?);
/// assert_eq!(key.to_string(), "packs.core");
/// assert_eq!(key.location().to_string(), "agentstacks.toml: packs.core");
/// assert_eq!(key.subdir_location().to_string(), "agentstacks.toml: packs.core.subdir");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceKey {
    /// A `[packs.<name>]` entry.
    Pack(PackName),
    /// A `[tools.<name>]` entry.
    Tool(ToolName),
}

impl SourceKey {
    /// Where a refusal about the entry is reported: the project file at the entry's key path.
    #[must_use]
    pub fn location(&self) -> Location {
        Location::at_key(PROJECT_FILE, self.to_string())
    }

    /// Where a refusal about the entry's `subdir` is reported.
    #[must_use]
    pub fn subdir_location(&self) -> Location {
        Location::at_key(PROJECT_FILE, format!("{self}.subdir"))
    }
}

impl fmt::Display for SourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack(name) => write!(f, "packs.{name}"),
            Self::Tool(name) => write!(f, "tools.{name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn source_key_displays_its_key_path() -> TestResult {
        assert_eq!(
            SourceKey::Pack(PackName::new("core")?).to_string(),
            "packs.core"
        );
        assert_eq!(
            SourceKey::Tool(ToolName::new("claude")?).to_string(),
            "tools.claude"
        );
        Ok(())
    }

    #[test]
    fn source_key_locations() -> TestResult {
        let key = SourceKey::Pack(PackName::new("core")?);
        assert_eq!(key.location().to_string(), "agentstacks.toml: packs.core");
        assert_eq!(
            key.subdir_location().to_string(),
            "agentstacks.toml: packs.core.subdir"
        );
        Ok(())
    }

    #[test]
    fn source_key_orders_packs_before_tools() -> TestResult {
        let tool = SourceKey::Tool(ToolName::new("a")?);
        let late_pack = SourceKey::Pack(PackName::new("z")?);
        let early_pack = SourceKey::Pack(PackName::new("b")?);
        assert!(late_pack < tool);
        assert!(early_pack < late_pack);
        Ok(())
    }
}
