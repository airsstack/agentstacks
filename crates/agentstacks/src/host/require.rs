//! `agentstacks.require`, which loads the extension's own modules from memory.
//!
//! # Responsibilities
//!
//! - [`function`] is the Lua function. It works in every phase and takes one string, a module
//!   name: letters, digits, `_` and `-` joined by dots, which names the `.lua` file of the
//!   extension that has the dots turned into `/`.
//! - It refuses, in this order: an argument that is not a string, a name that is not a module
//!   name, a module the extension does not hold, the entry file, and a module that is already
//!   being loaded, which is a cycle and is reported at the call that closes it. A refusal is an
//!   ordinary Lua error located at the caller: the extension may catch it, and uncaught it fails
//!   the run with its located message.
//! - A module runs once, as the chunk `@<path>`, so an error in it names its own file and line,
//!   and a call it makes is located in it. What it returns is kept for every later `require`,
//!   even when that is `nil`. A module that raised is not kept and runs again when required
//!   again.
//!
//! # Non-responsibilities
//!
//! Where the module files come from belongs to the file reader. `load` stays with the host and is
//! never given to the extension.

use std::sync::Arc;

use airsl::mlua::chunk::ChunkMode;
use airsl::mlua::{self, Function, Lua, Table, Value};

use crate::host::caller::locate;
use crate::host::convert::lua_type;
use crate::host::failure::Raised;
use crate::host::files::{LuaFiles, LuaModule};
use crate::host::state::{RunState, SharedState};
use crate::types::name::ToolName;
use crate::types::op::{ApiFunction, LuaType, OpError};
use crate::types::report::{Diagnostic, Quoted};
use crate::types::tree::TreePath;

/// The name of the table, in the Lua registry, that holds what each loaded module returned.
const CACHE: &str = "agentstacks.loaded";

/// The Lua function `agentstacks.require`.
pub(super) fn function(
    lua: &Lua,
    state: &SharedState,
    files: &Arc<LuaFiles>,
) -> mlua::Result<Function> {
    lua.set_named_registry_value(CACHE, lua.create_table()?)?;
    let state = state.clone();
    let files = Arc::clone(files);
    lua.create_function(move |lua, name: Value| load(lua, &state, &files, &name))
}

/// The module `name` names, from its first load or from what that load returned.
fn load(lua: &Lua, state: &SharedState, files: &LuaFiles, name: &Value) -> mlua::Result<Value> {
    let tool = state.with(|run| run.tool().clone());
    let (module, path, text) = resolve(state, files, &tool, name)
        .map_err(|error| Raised::ordinary(Diagnostic::new(locate(lua, &tool), error)).into_lua())?;
    let cache: Table = lua.named_registry_value(CACHE)?;
    if state.with(|run| run.is_loaded(&module)) {
        return cache.raw_get(module.as_str());
    }
    state.with(|run| run.start_loading(module.clone()));
    let loaded = lua
        .load(text)
        .set_name(format!("@{path}"))
        .set_mode(ChunkMode::Text)
        .call::<Value>(());
    state.with(RunState::stop_loading);
    let value = loaded?;
    if !matches!(value, Value::Nil) {
        cache.raw_set(module.as_str(), value.clone())?;
    }
    state.with(|run| run.mark_loaded(module));
    Ok(value)
}

/// The module `name` names, with its path and text, or the first check it fails.
fn resolve<'a>(
    state: &SharedState,
    files: &'a LuaFiles,
    tool: &ToolName,
    name: &Value,
) -> Result<(LuaModule, &'a TreePath, &'a str), OpError> {
    let Value::String(written) = name else {
        return Err(OpError::ArgumentType {
            function: ApiFunction::Require,
            expected: LuaType::String,
            got: lua_type(name),
        });
    };
    let written = written.to_string_lossy();
    let module = LuaModule::new(&written).ok_or_else(|| {
        let name = Quoted::new(written.clone());
        if written.contains('/') {
            OpError::ModuleNameDotted {
                name,
                dotted: written.replace('/', "."),
            }
        } else {
            OpError::ModuleName { name }
        }
    })?;
    let (path, text) = files
        .module(&module)
        .ok_or_else(|| OpError::ModuleNotFound {
            module: Quoted::new(written.clone()),
            tool: tool.clone(),
        })?;
    if files.entry_module().as_ref() == Some(&module) {
        return Err(OpError::EntryModule {
            name: Quoted::new(written.clone()),
        });
    }
    let loading = state.with(|run| run.loading().to_vec());
    if let Some(start) = loading.iter().position(|loading| loading == &module) {
        let chain: Vec<String> = loading[start..]
            .iter()
            .map(|loading| loading.as_str().to_owned())
            .chain(std::iter::once(written.clone()))
            .collect();
        return Err(OpError::ModuleCycle {
            module: Quoted::new(written.clone()),
            chain,
        });
    }
    Ok((module, path, text))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use crate::host::events::{Dispatched, Event};
    use crate::host::testing::Rig;

    type TestResult = Result<(), Box<dyn Error>>;

    const UTIL: &[u8] = b"local M = {}\nfunction M.twice(x) return x * 2 end\nreturn M";

    /// Runs `entry` as the entry file of an extension that holds `files`, and returns the string
    /// it returns, or the text of the refusal.
    fn run(files: &[(&str, &[u8])], entry: &str) -> Result<Result<String, String>, Box<dyn Error>> {
        Ok(Rig::with(&[], files)?.text(entry))
    }

    fn refusal(files: &[(&str, &[u8])], entry: &str) -> Result<String, Box<dyn Error>> {
        run(files, entry)?
            .err()
            .ok_or_else(|| "the run should be refused".into())
    }

    #[test]
    fn require_returns_the_module_value() -> TestResult {
        let found = run(
            &[("lib/util.lua", UTIL)],
            "return tostring(agentstacks.require('lib.util').twice(21))",
        )??;
        assert_eq!(found, "42");
        Ok(())
    }

    #[test]
    fn require_loads_once_and_caches() -> TestResult {
        let counter: &[u8] = b"loads = (loads or 0) + 1\nreturn {}";
        let found = run(
            &[("lib/counter.lua", counter)],
            "local a = agentstacks.require('lib.counter')\nlocal b = agentstacks.require('lib.counter')\nreturn tostring(loads) .. tostring(rawequal(a, b))",
        )??;
        assert_eq!(found, "1true");
        Ok(())
    }

    #[test]
    fn a_module_that_returns_nil_loads_once() -> TestResult {
        let nothing: &[u8] = b"loads = (loads or 0) + 1";
        let found = run(
            &[("lib/nothing.lua", nothing)],
            "local a = agentstacks.require('lib.nothing')\nlocal b = agentstacks.require('lib.nothing')\nreturn tostring(loads) .. tostring(a) .. tostring(b)",
        )??;
        assert_eq!(found, "1nilnil");
        Ok(())
    }

    #[test]
    fn module_error_carries_its_file_and_line() -> TestResult {
        let boom: &[u8] = b"local x = 1\nerror('boom')";
        assert_eq!(
            refusal(&[("lib/util.lua", boom)], "agentstacks.require('lib.util')")?,
            "claude/adapter.lua: lua error in adapter.lua: runtime error: lib/util.lua:2: boom"
        );
        Ok(())
    }

    #[test]
    fn failed_module_is_not_cached() -> TestResult {
        let flaky: &[u8] =
            b"if not tried then\n  tried = true\n  error('first')\nend\nreturn { v = 7 }";
        let found = run(
            &[("lib/flaky.lua", flaky)],
            "local ok = pcall(agentstacks.require, 'lib.flaky')\nassert(ok == false)\nreturn tostring(agentstacks.require('lib.flaky').v)",
        )??;
        assert_eq!(found, "7");
        Ok(())
    }

    #[test]
    fn invalid_name_with_slash_gets_the_hint() -> TestResult {
        assert_eq!(
            refusal(&[], "agentstacks.require('lib/x')")?,
            "claude/adapter.lua:1: require: invalid module name \"lib/x\" (use dots: lib.x)"
        );
        Ok(())
    }

    #[test]
    fn invalid_name_without_slash_has_no_hint() -> TestResult {
        assert_eq!(
            refusal(&[], "agentstacks.require('x y')")?,
            "claude/adapter.lua:1: require: invalid module name \"x y\""
        );
        Ok(())
    }

    #[test]
    fn require_takes_one_string() -> TestResult {
        assert_eq!(
            refusal(&[], "agentstacks.require(5)")?,
            "claude/adapter.lua:1: require: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn missing_module_is_refused() -> TestResult {
        assert_eq!(
            refusal(&[], "agentstacks.require('lib.x')")?,
            "claude/adapter.lua:1: require: module \"lib.x\" not found in extension claude"
        );
        Ok(())
    }

    #[test]
    fn entry_module_is_refused() -> TestResult {
        assert_eq!(
            refusal(&[], "agentstacks.require('adapter')")?,
            "claude/adapter.lua:1: require: \"adapter\" is the entry file"
        );
        Ok(())
    }

    #[test]
    fn cycle_is_reported_at_the_closing_call() -> TestResult {
        let a: &[u8] = b"return agentstacks.require('lib.b')";
        let b: &[u8] = b"return agentstacks.require('lib.a')";
        assert_eq!(
            refusal(
                &[("lib/a.lua", a), ("lib/b.lua", b)],
                "agentstacks.require('lib.a')"
            )?,
            "claude/lib/b.lua:1: require: module \"lib.a\" requires itself (lib.a \u{2192} lib.b \u{2192} lib.a)"
        );
        Ok(())
    }

    #[test]
    fn a_module_requiring_itself_is_a_cycle_of_one() -> TestResult {
        let own: &[u8] = b"return agentstacks.require('lib.own')";
        assert_eq!(
            refusal(&[("lib/own.lua", own)], "agentstacks.require('lib.own')")?,
            "claude/lib/own.lua:1: require: module \"lib.own\" requires itself (lib.own \u{2192} lib.own)"
        );
        Ok(())
    }

    #[test]
    fn require_refusal_does_not_stick() -> TestResult {
        let rig = Rig::new()?;
        rig.run("local ok = pcall(agentstacks.require, 'lib.x')\nassert(ok == false)")?;
        assert_eq!(rig.recorded(), 0);
        Ok(())
    }

    #[test]
    fn load_stays_withheld() -> TestResult {
        let found = run(
            &[("lib/util.lua", UTIL)],
            "agentstacks.require('lib.util')\nreturn type(load)",
        )??;
        assert_eq!(found, "nil");
        Ok(())
    }

    #[test]
    fn require_works_inside_a_handler() -> TestResult {
        let rig = Rig::with(&[], &[("lib/util.lua", UTIL)])?;
        rig.run(
            "agentstacks.on('values', function()\n  return { twice = agentstacks.require('lib.util').twice(4) }\nend)",
        )?;
        assert_eq!(
            rig.dispatch(Event::Values, &json!({}))?,
            Dispatched::Value(json!({"twice": 8}))
        );
        Ok(())
    }
}
