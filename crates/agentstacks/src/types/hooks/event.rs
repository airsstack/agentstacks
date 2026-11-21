//! The events a pack hook can attach to.
//!
//! It is separate from `Hook` because the event decides which selectors are legal, and that table is
//! a rule of the vocabulary, not of one hook.
//!
//! # Responsibilities
//!
//! - [`HookEvent`] is the closed vocabulary of neutral events: `session.start`, `session.end`,
//!   `prompt.submit`, `tool.before`, `tool.after` and `agent.stop`.
//! - [`HookEvent::match_values`] says which `match` values an event accepts, or that it accepts
//!   none, and [`HookEvent::allows_agents`] says whether the event takes an `agents` selector.
//!
//! # Non-responsibilities
//!
//! Tool-specific events belong to `native/<tool>/hooks.toml`, where the event is an opaque string.

use std::fmt;

use crate::types::hooks::InvalidHookEvent;
use crate::types::hooks::MatchValue;

/// A neutral hook event.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{HookEvent, MatchValue};
///
/// let event = HookEvent::parse("tool.before")?;
/// assert_eq!(event.as_str(), "tool.before");
/// assert_eq!(
///     event.match_values(),
///     Some([MatchValue::Read, MatchValue::Edit, MatchValue::Write].as_slice())
/// );
/// assert!(!event.allows_agents());
/// assert!(HookEvent::AgentStop.allows_agents());
///
/// let refused = HookEvent::parse("tool.pre").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"tool.pre\" unknown");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    /// A session starts.
    SessionStart,
    /// A session ends.
    SessionEnd,
    /// The user submits a prompt.
    PromptSubmit,
    /// Before a tool call.
    ToolBefore,
    /// After a tool call.
    ToolAfter,
    /// An agent stops.
    AgentStop,
}

impl HookEvent {
    /// Parses the `event` field of a neutral hook.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidHookEvent`] for a name outside the vocabulary.
    pub fn parse(text: &str) -> Result<Self, InvalidHookEvent> {
        match text {
            "session.start" => Ok(Self::SessionStart),
            "session.end" => Ok(Self::SessionEnd),
            "prompt.submit" => Ok(Self::PromptSubmit),
            "tool.before" => Ok(Self::ToolBefore),
            "tool.after" => Ok(Self::ToolAfter),
            "agent.stop" => Ok(Self::AgentStop),
            _ => Err(InvalidHookEvent::new(text)),
        }
    }

    /// The `match` values this event accepts, or `None` when `match` is not allowed on it.
    #[must_use]
    pub const fn match_values(self) -> Option<&'static [MatchValue]> {
        match self {
            Self::SessionStart => Some(&[
                MatchValue::Startup,
                MatchValue::Resume,
                MatchValue::Clear,
                MatchValue::Compact,
            ]),
            Self::ToolBefore | Self::ToolAfter => {
                Some(&[MatchValue::Read, MatchValue::Edit, MatchValue::Write])
            }
            Self::SessionEnd | Self::PromptSubmit | Self::AgentStop => None,
        }
    }

    /// Whether the event takes an `agents` selector.
    #[must_use]
    pub const fn allows_agents(self) -> bool {
        matches!(self, Self::AgentStop)
    }

    /// The event as written in a manifest.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session.start",
            Self::SessionEnd => "session.end",
            Self::PromptSubmit => "prompt.submit",
            Self::ToolBefore => "tool.before",
            Self::ToolAfter => "tool.after",
            Self::AgentStop => "agent.stop",
        }
    }
}

impl fmt::Display for HookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const ALL: [HookEvent; 6] = [
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
        HookEvent::PromptSubmit,
        HookEvent::ToolBefore,
        HookEvent::ToolAfter,
        HookEvent::AgentStop,
    ];

    #[test]
    fn hook_event_parses_the_six_events() -> TestResult {
        for name in [
            "session.start",
            "session.end",
            "prompt.submit",
            "tool.before",
            "tool.after",
            "agent.stop",
        ] {
            assert_eq!(HookEvent::parse(name)?.to_string(), name);
        }
        let parsed: Result<Vec<HookEvent>, InvalidHookEvent> =
            ALL.iter().map(|e| HookEvent::parse(e.as_str())).collect();
        assert_eq!(parsed?, ALL);
        Ok(())
    }

    #[test]
    fn hook_event_refuses_unknown_events() -> TestResult {
        let error = HookEvent::parse("tool.pre")
            .err()
            .ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "hooks[0].event");
        assert_eq!(
            Diagnostic::new(location, ManifestError::from(error)).to_string(),
            "packs/core/pack.toml: hooks[0].event: \"tool.pre\" unknown"
        );
        Ok(())
    }

    #[test]
    fn hook_event_lists_selectors_per_event() {
        assert_eq!(
            HookEvent::SessionStart.match_values(),
            Some(
                [
                    MatchValue::Startup,
                    MatchValue::Resume,
                    MatchValue::Clear,
                    MatchValue::Compact
                ]
                .as_slice()
            )
        );
        let tools = Some([MatchValue::Read, MatchValue::Edit, MatchValue::Write].as_slice());
        assert_eq!(HookEvent::ToolBefore.match_values(), tools);
        assert_eq!(HookEvent::ToolAfter.match_values(), tools);
        for event in [
            HookEvent::SessionEnd,
            HookEvent::PromptSubmit,
            HookEvent::AgentStop,
        ] {
            assert_eq!(event.match_values(), None, "{event}");
        }
        for event in ALL {
            assert_eq!(
                event.allows_agents(),
                event == HookEvent::AgentStop,
                "{event}"
            );
        }
    }
}
