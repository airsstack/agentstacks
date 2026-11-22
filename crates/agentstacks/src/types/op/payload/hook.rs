//! A hook of a pack, as the plan payload lists it.
//!
//! # Responsibilities
//!
//! - [`PlanAction`] is what a hook does: run a Lua script, or show a message.
//! - [`PlanHook`] is a neutral hook, whose event and `match` the format defines.
//! - [`PlanNativeHook`] is a hook of a tool's `native/<tool>/hooks.toml`, whose event and
//!   `match` stay as written because the tool defines them.
//!
//! # Non-responsibilities
//!
//! Building the command line of a Lua hook belongs to the hook protocol, and rendering the
//! message belongs to the plan flow.

use crate::types::hooks::{HookEvent, MatchValue};
use crate::types::name::ItemName;
use crate::types::op::ItemId;

/// What a hook does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAction {
    /// Runs a Lua script.
    Lua {
        /// Whether the hook may block the tool event, as the pack declares it.
        may_block: bool,
        /// The command line that runs the hook.
        command: Vec<String>,
    },
    /// Shows the agent a message.
    Message {
        /// The rendered text.
        text: String,
    },
}

impl PlanAction {
    /// Whether the action runs a Lua script.
    #[must_use]
    pub const fn is_lua(&self) -> bool {
        matches!(self, Self::Lua { .. })
    }
}

/// A neutral hook of a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanHook {
    /// The id an extension lists in `covers`.
    pub id: ItemId,
    /// The event it runs on.
    pub event: HookEvent,
    /// The values it matches, as the pack lists them.
    pub matches: Option<Vec<MatchValue>>,
    /// The agents it applies to, as the pack lists them.
    pub agents: Option<Vec<ItemName>>,
    /// What it does.
    pub action: PlanAction,
}

/// A hook of a tool's native directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNativeHook {
    /// The id an extension lists in `covers`.
    pub id: ItemId,
    /// The tool's own event name, as written.
    pub event: String,
    /// The tool's own match values, as written.
    pub matches: Option<Vec<String>>,
    /// What it does.
    pub action: PlanAction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn plan_action_tells_lua_from_message() {
        let lua = PlanAction::Lua {
            may_block: true,
            command: vec!["airsl".to_owned()],
        };
        let message = PlanAction::Message {
            text: "hi".to_owned(),
        };
        assert!(lua.is_lua());
        assert!(!message.is_lua());
    }

    #[test]
    fn plan_hooks_keep_match_and_agents_as_written() -> TestResult {
        let core = PackName::new("core")?;
        let hook = PlanHook {
            id: ItemId::hook(&core, 0),
            event: HookEvent::ToolBefore,
            matches: Some(vec![MatchValue::Read, MatchValue::Edit]),
            agents: None,
            action: PlanAction::Message {
                text: "hi".to_owned(),
            },
        };
        assert_eq!(
            hook.matches.as_deref(),
            Some([MatchValue::Read, MatchValue::Edit].as_slice())
        );
        assert_eq!(hook.agents, None);
        let native = PlanNativeHook {
            id: ItemId::native_hook(&core, 0),
            event: "Stop".to_owned(),
            matches: Some(vec!["Bash".to_owned()]),
            action: PlanAction::Message {
                text: "hi".to_owned(),
            },
        };
        assert_eq!(native.event, "Stop");
        assert_eq!(native.matches, Some(vec!["Bash".to_owned()]));
        Ok(())
    }
}
