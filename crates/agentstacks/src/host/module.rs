//! The `agentstacks` table an extension calls, installed into the sandbox.
//!
//! # Responsibilities
//!
//! - [`AgentstacksModule`] is the airsl host module that installs `airsstack.agentstacks`, sets
//!   the same table as the global `agentstacks`, and removes the two output channels, `print`
//!   and `warn`.
//! - The table holds `on`, `require`, `write`, `copy`, `skip`, `require_binary`, `read` and
//!   `json.edit`, each served by its own file.
//!
//! # Non-responsibilities
//!
//! What each function does belongs to the file that owns it, and which modules make up the
//! sandbox belongs to the engine builder.

use std::sync::Arc;

use airsl::mlua::{self, Lua, Table, Value};
use airsl::{HostModule, InstallContext, ModuleName};

use crate::host::files::LuaFiles;
use crate::host::state::SharedState;
use crate::host::{events, json, ops, read, require};

/// The name the module is installed under in the `airsstack` table.
const NAME: &str = "agentstacks";

/// Installs `airsstack.agentstacks` and the global `agentstacks`.
#[derive(Debug)]
pub(super) struct AgentstacksModule {
    name: ModuleName,
    state: SharedState,
    files: Arc<LuaFiles>,
}

impl AgentstacksModule {
    /// The module whose functions share `state` and whose `require` serves `files`.
    ///
    /// # Errors
    ///
    /// Never in practice: the module's name satisfies the engine's rules for one.
    pub(super) fn new(state: SharedState, files: Arc<LuaFiles>) -> airsl::Result<Self> {
        Ok(Self {
            name: ModuleName::new(NAME)?,
            state,
            files,
        })
    }
}

impl HostModule for AgentstacksModule {
    fn name(&self) -> &ModuleName {
        &self.name
    }

    fn install(
        &self,
        lua: &Lua,
        table: &Table,
        _context: &InstallContext<'_>,
    ) -> airsl::Result<()> {
        fill(lua, table, &self.state, &self.files).map_err(|error| airsl::Error::ModuleInstall {
            module: NAME.to_owned(),
            reason: error.to_string(),
        })
    }
}

/// Fills `table` with the API, makes it the global `agentstacks`, and removes `print` and `warn`.
fn fill(lua: &Lua, table: &Table, state: &SharedState, files: &Arc<LuaFiles>) -> mlua::Result<()> {
    table.set("on", events::on(lua)?)?;
    table.set("require", require::function(lua, state, files)?)?;
    ops::install(lua, table, state)?;
    table.set("read", read::function(lua, state)?)?;
    let helpers = lua.create_table()?;
    helpers.set("edit", json::edit(lua, state)?)?;
    table.set("json", helpers)?;

    let globals = lua.globals();
    globals.set(NAME, table.clone())?;
    globals.set("print", Value::Nil)?;
    globals.set("warn", Value::Nil)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use airsl::Script;

    use super::*;
    use crate::host::engine::build_engine;
    use crate::host::testing::lua_files;
    use crate::types::name::ToolName;

    type TestResult = Result<(), Box<dyn Error>>;

    fn run(source: &str) -> Result<String, Box<dyn Error>> {
        let state = SharedState::new(ToolName::new("claude")?);
        let engine = build_engine(AgentstacksModule::new(state, lua_files(&[])?)?)?;
        Ok(engine.eval_to::<String>(&Script::from_source(source, "adapter.lua")?)?)
    }

    const KEYS: &str = "local keys = {} for key in pairs(%) do keys[#keys + 1] = key end \
                        table.sort(keys) return table.concat(keys, ',')";

    fn keys_of(table: &str) -> String {
        KEYS.replace('%', table)
    }

    #[test]
    fn agentstacks_global_is_the_module_table() -> TestResult {
        assert_eq!(
            run("return tostring(rawequal(agentstacks, airsstack.agentstacks))")?,
            "true"
        );
        Ok(())
    }

    #[test]
    fn agentstacks_table_has_exactly_the_api_one_functions() -> TestResult {
        assert_eq!(
            run(&keys_of("agentstacks"))?,
            "copy,json,on,read,require,require_binary,skip,write"
        );
        assert_eq!(run(&keys_of("agentstacks.json"))?, "edit");
        Ok(())
    }
}
