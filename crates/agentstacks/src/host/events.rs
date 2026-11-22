//! The events the host dispatches to an extension, and how an extension registers for them.
//!
//! # Responsibilities
//!
//! - [`Event`] names the two events an extension registers a handler for, `values` and `plan`.
//! - [`on`] is the Lua function `agentstacks.on(event, fn)`. It forwards both arguments to
//!   `airsstack.ext.on`, looked up when it is called because the `airsstack` global does not
//!   exist yet while modules install. A later registration replaces the earlier one.
//! - [`dispatch`] calls the registered handler and tells apart the three answers: no handler, a
//!   handler that returned `nil`, and a handler that returned a table or another value.
//!
//! # Non-responsibilities
//!
//! Which events exist is the engine's declaration, and what a handler's answer means belongs to
//! the rule that reads it.

use airsl::mlua::{self, Function, Lua, MultiValue, Table};
use airsl::{Engine, EventName};
use serde_json::Value;

/// An event the host dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    /// Asks the extension where it puts each item of a pack.
    Values,
    /// Asks the extension for its operations.
    Plan,
}

impl Event {
    /// Every event, in the order the host dispatches them.
    pub(super) const ALL: [Self; 2] = [Self::Values, Self::Plan];

    /// The event as the Lua author spells it.
    pub(super) const fn word(self) -> &'static str {
        match self {
            Self::Values => "values",
            Self::Plan => "plan",
        }
    }

    /// The event as the engine names it.
    ///
    /// # Errors
    ///
    /// Never in practice: both words satisfy the engine's rules for an event name.
    pub(super) fn name(self) -> airsl::Result<EventName> {
        EventName::new(self.word())
    }
}

/// What a dispatch of an event gave back.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Dispatched {
    /// The extension registered no handler for the event.
    NoHandler,
    /// The handler returned `nil`, or nothing.
    Nil,
    /// The handler returned this value.
    Value(Value),
}

/// Calls the handler of `event` with `payload`.
///
/// # Errors
///
/// The engine's error when the handler fails or exceeds a limit.
pub(super) fn dispatch(
    engine: &Engine,
    event: Event,
    payload: &Value,
) -> airsl::Result<Dispatched> {
    Ok(match engine.dispatch(&event.name()?, payload)? {
        None => Dispatched::NoHandler,
        Some(Value::Null) => Dispatched::Nil,
        Some(other) => Dispatched::Value(other),
    })
}

/// The Lua function `agentstacks.on`.
pub(super) fn on(lua: &Lua) -> mlua::Result<Function> {
    lua.create_function(|lua, arguments: MultiValue| {
        let airsstack: Table = lua.globals().get("airsstack")?;
        let ext: Table = airsstack.get("ext")?;
        let register: Function = ext.get("on")?;
        register.call::<()>(arguments)
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::host::testing::Rig;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn both_events_have_engine_names() {
        let words: Vec<String> = Event::ALL
            .iter()
            .filter_map(|event| event.name().ok())
            .map(|name| name.to_string())
            .collect();
        assert_eq!(words, ["values", "plan"]);
    }

    #[test]
    fn on_forwards_to_ext_on_at_call_time() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('values', function() return { root = '.x' } end)")?;
        assert_eq!(
            rig.dispatch(Event::Values, &json!({}))?,
            Dispatched::Value(json!({"root": ".x"}))
        );
        Ok(())
    }

    #[test]
    fn second_registration_replaces_the_first() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('values', function() return { which = 'first' } end)\n\
             agentstacks.on('values', function() return { which = 'second' } end)",
        )?;
        assert_eq!(
            rig.dispatch(Event::Values, &json!({}))?,
            Dispatched::Value(json!({"which": "second"}))
        );
        Ok(())
    }

    #[test]
    fn registration_inside_values_takes_effect_at_plan() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('values', function()\n\
               agentstacks.on('plan', function() return { ran = 'plan' } end)\n\
               return {}\n\
             end)",
        )?;
        assert_eq!(
            rig.dispatch(Event::Plan, &json!({}))?,
            Dispatched::NoHandler
        );
        rig.dispatch(Event::Values, &json!({}))?;
        assert_eq!(
            rig.dispatch(Event::Plan, &json!({}))?,
            Dispatched::Value(json!({"ran": "plan"}))
        );
        Ok(())
    }

    #[test]
    fn no_handler_is_distinct_from_nil_and_table() -> TestResult {
        let rig = Rig::new()?;
        assert_eq!(
            rig.dispatch(Event::Values, &json!({}))?,
            Dispatched::NoHandler
        );
        rig.run("agentstacks.on('values', function() return nil end)")?;
        assert_eq!(rig.dispatch(Event::Values, &json!({}))?, Dispatched::Nil);
        rig.run("agentstacks.on('values', function() return {a = 1} end)")?;
        assert_eq!(
            rig.dispatch(Event::Values, &json!({}))?,
            Dispatched::Value(json!({"a": 1}))
        );
        Ok(())
    }

    #[test]
    fn unknown_event_is_airsl_text_with_sorted_events() -> TestResult {
        let rig = Rig::new()?;
        let refused = rig.run("agentstacks.on('nope', function() end)").err();
        assert_eq!(
            refused.as_deref(),
            Some(
                "claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values"
            )
        );
        Ok(())
    }

    #[test]
    fn syntax_error_shows_first_line() -> TestResult {
        let rig = Rig::new()?;
        let refused = rig.run("local function f(").err();
        assert_eq!(
            refused.as_deref(),
            Some(
                "claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>"
            )
        );
        Ok(())
    }

    #[test]
    fn runtime_error_in_plan_names_the_event_and_drops_the_traceback() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('plan', function()\n  local x = 1\n  error('path must be relative')\nend)",
        )?;
        let refused = rig.dispatch(Event::Plan, &json!({})).err();
        assert_eq!(
            refused.as_deref(),
            Some(
                "claude/adapter.lua: lua error in plan: runtime error: adapter.lua:3: path must be relative"
            )
        );
        assert!(refused.is_some_and(|text| !text.contains("stack traceback")));
        Ok(())
    }

    #[test]
    fn instruction_limit_in_plan() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('plan', function() while true do end end)")?;
        let refused = rig.dispatch(Event::Plan, &json!({})).err();
        assert_eq!(
            refused.as_deref(),
            Some("claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000")
        );
        Ok(())
    }

    #[test]
    fn memory_limit_in_entry() -> TestResult {
        let rig = Rig::new()?;
        let refused = rig.run("local s = string.rep('x', 20*1024*1024)").err();
        assert_eq!(
            refused.as_deref(),
            Some(
                "claude/adapter.lua: script `adapter.lua` exceeded its memory limit of 16777216 bytes"
            )
        );
        Ok(())
    }

    #[test]
    fn breach_caught_by_pcall_is_the_extensions_business() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('plan', function()\n  local ok = pcall(function() while true do end end)\n  return { caught = not ok }\nend)",
        )?;
        assert_eq!(
            rig.dispatch(Event::Plan, &json!({}))?,
            Dispatched::Value(json!({"caught": true}))
        );
        Ok(())
    }

    #[test]
    fn budget_resets_on_each_dispatch() -> TestResult {
        let rig = Rig::new()?;
        rig.run(
            "agentstacks.on('values', function()\n  pcall(function() while true do end end)\n  return {}\nend)\n\
             agentstacks.on('plan', function()\n  for i = 1, 1000000 do end\n  return { done = true }\nend)",
        )?;
        rig.dispatch(Event::Values, &json!({}))?;
        assert_eq!(
            rig.dispatch(Event::Plan, &json!({}))?,
            Dispatched::Value(json!({"done": true}))
        );
        Ok(())
    }
}
