//! The answer of the `values` handler, read into the checked values of the run.
//!
//! # Responsibilities
//!
//! - [`values_result`] refuses a handler that is not registered and an answer that is not a
//!   table, which are the two refusals that need to know what a handler returned. A table goes to
//!   the values rules of the operation types, which check it key by key. A type word in a
//!   refusal is the word the Lua author wrote: an object is a `table`, an array a `list`.
//!
//! # Non-responsibilities
//!
//! Which keys the table may hold, and what each must be, belong to the values rules.

use std::path::Path;

use serde_json::Value;

use crate::host::convert::json_word;
use crate::host::events::Dispatched;
use crate::types::op::{LuaType, OpError, ValuesInput, ValuesResult};
use crate::types::report::{Diagnostic, Diagnostics, Location};

/// The checked answer of the `values` handler of the extension whose entry file is `entry`,
/// which refusals name as `<tool>/<entry file>`.
///
/// # Errors
///
/// Every refusal found in the handler's answer, or the one refusal of a handler that is not
/// there.
pub(super) fn values_result(
    entry: &Path,
    dispatched: Dispatched,
    input: &ValuesInput,
) -> Result<ValuesResult, Diagnostics> {
    let refuse =
        |location: Location, error: OpError| Diagnostics::from(Diagnostic::new(location, error));
    match dispatched {
        Dispatched::NoHandler => Err(refuse(Location::file(entry), OpError::NoValuesHandler)),
        Dispatched::Nil => Err(refuse(
            Location::at_key(entry, "values"),
            OpError::ValuesType {
                expected: LuaType::Table,
                got: LuaType::Nil,
            },
        )),
        Dispatched::Value(Value::Object(table)) => {
            ValuesResult::read(&table, input, entry, json_word)
        }
        Dispatched::Value(other) => Err(refuse(
            Location::at_key(entry, "values"),
            OpError::ValuesType {
                expected: LuaType::Table,
                got: json_word(&other),
            },
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::*;
    use crate::host::events::Event;
    use crate::host::payload::values_payload;
    use crate::host::testing::{Rig, values_input};
    use crate::types::name::{ItemName, PackName};
    use crate::types::tokens::Var;

    type TestResult = Result<(), Box<dyn Error>>;

    const ENTRY: &str = "claude/adapter.lua";

    /// A `values` handler of a tool extension.
    const EXAMPLE: &str = "agentstacks.on('values', function(q)
  local out = { root = '.claude/agentstacks', skills = {}, agents = {}, vars = { tmpdir = '$TMPDIR' } }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. '/' .. s] = { invoke = '/' .. s, dir = '.claude/skills/' .. s }
    end
    for _, a in ipairs(p.agents) do out.agents[p.name .. '/' .. a] = a end
  end
  return out
end)";

    /// Runs `entry` as the entry file, dispatches `values` with the example payload and reads the
    /// answer.
    fn answer(entry: &str) -> Result<Result<ValuesResult, Vec<String>>, Box<dyn Error>> {
        let input = values_input()?;
        let rig = Rig::new()?;
        rig.run(entry)?;
        let dispatched = rig.dispatch(Event::Values, &values_payload(&input))?;
        Ok(
            values_result(Path::new(ENTRY), dispatched, &input).map_err(|found| {
                found
                    .into_sorted()
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            }),
        )
    }

    fn refused(entry: &str) -> Result<Vec<String>, Box<dyn Error>> {
        answer(entry)?
            .err()
            .ok_or_else(|| "the answer should be refused".into())
    }

    #[test]
    fn values_example_dispatches_to_the_spec_result() -> TestResult {
        let result = answer(EXAMPLE)?.map_err(|found| found.join("; "))?;
        let core = PackName::new("core")?;
        assert_eq!(result.root().as_str(), ".claude/agentstacks");
        let status = result
            .skill(&core, &ItemName::new("status")?)
            .ok_or("a value for the skill")?;
        assert_eq!(status.invoke, "/status");
        assert_eq!(status.dir.as_str(), ".claude/skills/status");
        assert_eq!(result.agent(&core, &ItemName::new("coder")?), Some("coder"));
        assert_eq!(result.var(Var::Tmpdir), Some("$TMPDIR"));
        assert_eq!(result.var(Var::SessionId), None);
        Ok(())
    }

    #[test]
    fn missing_values_handler_is_refused() -> TestResult {
        let rig = Rig::new()?;
        rig.run("agentstacks.on('plan', function() end)")?;
        let dispatched = rig.dispatch(Event::Values, &json!({}))?;
        let found = values_result(Path::new(ENTRY), dispatched, &values_input()?)
            .err()
            .ok_or("a refusal")?;
        let shown: Vec<String> = found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(shown, ["claude/adapter.lua: registers no values handler"]);
        Ok(())
    }

    #[test]
    fn values_returning_nil_is_refused() -> TestResult {
        assert_eq!(
            refused("agentstacks.on('values', function() return nil end)")?,
            ["claude/adapter.lua: values: expected table, got nil"]
        );
        Ok(())
    }

    #[test]
    fn values_returning_a_list_names_list() -> TestResult {
        assert_eq!(
            refused("agentstacks.on('values', function() return {1, 2} end)")?,
            ["claude/adapter.lua: values: expected table, got list"]
        );
        Ok(())
    }

    #[test]
    fn values_returning_a_scalar_names_its_word() -> TestResult {
        assert_eq!(
            refused("agentstacks.on('values', function() return 'x' end)")?,
            ["claude/adapter.lua: values: expected table, got string"]
        );
        Ok(())
    }

    #[test]
    fn empty_table_arrives_as_object() -> TestResult {
        assert_eq!(
            refused("agentstacks.on('values', function() return {} end)")?,
            ["claude/adapter.lua: values.root: required"]
        );
        Ok(())
    }

    #[test]
    fn nested_type_word_is_list() -> TestResult {
        assert_eq!(
            refused(
                "agentstacks.on('values', function() return { root = '.x', agents = {'x'} } end)"
            )?,
            ["claude/adapter.lua: values.agents: expected table, got list"]
        );
        Ok(())
    }

    #[test]
    fn skill_entry_needs_dir() -> TestResult {
        assert_eq!(
            refused(
                "agentstacks.on('values', function() return { root = '.x', skills = { ['core/status'] = { invoke = '/status' } } } end)"
            )?,
            ["claude/adapter.lua: values.skills.\"core/status\".dir: required"]
        );
        Ok(())
    }
}
