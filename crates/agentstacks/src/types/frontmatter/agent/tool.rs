//! A capability an agent may be given, named without tying it to one tool.
//!
//! A closed enum rather than free strings, so an unknown capability is refused when the file is
//! read, and each tool extension maps every variant to its own tool's name.
//!
//! # Responsibilities
//!
//! - [`AgentTool`] is the closed set `read`, `write`, `edit`, `grep`, `glob`, `shell` and `skill`.
//! - [`AgentTool::ALL`] lists the values in declaration order.
//!
//! # Non-responsibilities
//!
//! Refusing a tool listed twice belongs to [`AgentMeta`](crate::types::AgentMeta), which sees the whole
//! list. Mapping a tool to a tool extension's own name belongs to that extension.

use std::fmt;
use std::str::FromStr;

use crate::types::frontmatter::error::FrontmatterError;

/// A tool-neutral agent capability.
///
/// # Examples
///
/// ```
/// use agentstacks::types::AgentTool;
///
/// assert_eq!("shell".parse::<AgentTool>()?, AgentTool::Shell);
/// let refused = "bash".parse::<AgentTool>().err().map(|error| error.to_string());
/// assert_eq!(refused.as_deref(), Some("unknown \"bash\""));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTool {
    /// Read files.
    Read,
    /// Write files.
    Write,
    /// Edit files.
    Edit,
    /// Search file contents.
    Grep,
    /// Match file names.
    Glob,
    /// Run shell commands.
    Shell,
    /// Run skills.
    Skill,
}

impl AgentTool {
    /// Every value, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Read,
        Self::Write,
        Self::Edit,
        Self::Grep,
        Self::Glob,
        Self::Shell,
        Self::Skill,
    ];

    /// Parses `value` as written in frontmatter.
    ///
    /// # Errors
    ///
    /// Returns [`FrontmatterError::UnknownTool`] when `value` is not one of [`AgentTool::ALL`].
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::AgentTool;
    ///
    /// assert_eq!(AgentTool::new("shell")?.as_str(), "shell");
    /// assert!(AgentTool::new("bash").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, FrontmatterError> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .ok_or_else(|| FrontmatterError::UnknownTool {
                text: value.to_owned(),
            })
    }

    /// The value as written in frontmatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::AgentTool;
    ///
    /// assert_eq!(AgentTool::Glob.as_str(), "glob");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Edit => "edit",
            Self::Grep => "grep",
            Self::Glob => "glob",
            Self::Shell => "shell",
            Self::Skill => "skill",
        }
    }
}

impl fmt::Display for AgentTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentTool {
    type Err = FrontmatterError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<&str> for AgentTool {
    type Error = FrontmatterError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tool_parses_each_value() {
        for text in ["read", "write", "edit", "grep", "glob", "shell", "skill"] {
            let parsed = text.parse::<AgentTool>().ok();
            assert_eq!(parsed.map(|tool| tool.to_string()).as_deref(), Some(text));
        }
        assert_eq!(AgentTool::ALL.len(), 7);
    }

    #[test]
    fn agent_tool_refuses_unknown_values() {
        for (text, shown) in [("bash", "unknown \"bash\""), ("Read", "unknown \"Read\"")] {
            let refused = text.parse::<AgentTool>().err().map(|e| e.to_string());
            assert_eq!(refused.as_deref(), Some(shown));
        }
    }

    #[test]
    fn agent_tool_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<AgentTool, FrontmatterError>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["shell", "bash"] {
            let expected = shown(AgentTool::new(text));
            assert_eq!(shown(text.parse::<AgentTool>()), expected);
            assert_eq!(shown(AgentTool::try_from(text)), expected);
        }
        assert!(AgentTool::new("shell").is_ok());
        assert!(AgentTool::new("bash").is_err());
    }
}
