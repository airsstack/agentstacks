//! The airsl engine every extension runs in.
//!
//! # Responsibilities
//!
//! - [`build_engine`] builds an engine under `Policy::pure()` with an explicit set of three host
//!   modules: `airsstack.json`, `airsstack.ext` for the two events the host dispatches, and the
//!   module the caller gives, which is the `agentstacks` table.
//!
//! # Non-responsibilities
//!
//! `Policy::pure()` alone also installs `stdio`, `hook` and `time`, so the set is always spelled
//! out here. What the `agentstacks` table holds belongs to the module.

use airsl::modules::{Ext, Json};
use airsl::{Engine, HostModule, ModuleSet, Policy};

use crate::host::events::Event;

/// An engine with no grants whose only modules are `json`, `ext` and `module`.
///
/// # Errors
///
/// The engine's own refusal when a module cannot be installed.
pub(super) fn build_engine<M: HostModule + 'static>(module: M) -> airsl::Result<Engine> {
    let mut events = Vec::with_capacity(Event::ALL.len());
    for event in Event::ALL {
        events.push(event.name()?);
    }
    let mut set = ModuleSet::new();
    // dyn: airsl's ModuleSet stores trait objects
    set.insert(Box::new(Json::new()))?;
    set.insert(Box::new(Ext::with_events(events)))?;
    set.insert(Box::new(module))?;
    Engine::builder().policy(Policy::pure()).stdlib(set).build()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use airsl::Script;

    use super::*;
    use crate::host::module::AgentstacksModule;
    use crate::host::state::SharedState;
    use crate::host::testing::lua_files;
    use crate::types::name::ToolName;

    type TestResult = Result<(), Box<dyn Error>>;

    fn run(source: &str) -> Result<String, Box<dyn Error>> {
        let state = SharedState::new(ToolName::new("claude")?);
        let engine = build_engine(AgentstacksModule::new(state, lua_files(&[])?)?)?;
        Ok(engine.eval_to::<String>(&Script::from_source(source, "adapter.lua")?)?)
    }

    fn type_of(name: &str) -> Result<String, Box<dyn Error>> {
        run(&format!("return type({name})"))
    }

    #[test]
    fn sandbox_globals_are_exactly_the_allowed_set() -> TestResult {
        let found = run(
            "local g = {} for k in pairs(_G) do g[#g + 1] = tostring(k) end \
                         table.sort(g) return table.concat(g, ',')",
        )?;
        let mut allowed = vec![
            "_G",
            "_VERSION",
            "agentstacks",
            "airsstack",
            "arg",
            "assert",
            "collectgarbage",
            "error",
            "getmetatable",
            "ipairs",
            "math",
            "next",
            "pairs",
            "pcall",
            "rawequal",
            "rawget",
            "rawlen",
            "rawset",
            "select",
            "setmetatable",
            "string",
            "table",
            "tonumber",
            "tostring",
            "type",
            "utf8",
            "xpcall",
        ];
        allowed.sort_unstable();
        assert_eq!(found, allowed.join(","));
        Ok(())
    }

    #[test]
    fn print_and_warn_are_removed() -> TestResult {
        assert_eq!(type_of("print")?, "nil");
        assert_eq!(type_of("warn")?, "nil");
        Ok(())
    }

    #[test]
    fn withheld_globals_are_absent() -> TestResult {
        for name in ["require", "load", "dofile", "os", "io", "coroutine"] {
            assert_eq!(type_of(name)?, "nil", "{name}");
        }
        Ok(())
    }

    #[test]
    fn airsstack_holds_only_agentstacks_ext_and_json() -> TestResult {
        let found = run(
            "local k = {} for n in pairs(airsstack) do k[#k + 1] = n end \
                         table.sort(k) return table.concat(k, ',')",
        )?;
        assert_eq!(found, "agentstacks,ext,json");
        Ok(())
    }

    #[test]
    fn string_table_math_utf8_are_present() -> TestResult {
        for name in ["string", "table", "math", "utf8"] {
            assert_eq!(type_of(name)?, "table", "{name}");
        }
        Ok(())
    }
}
