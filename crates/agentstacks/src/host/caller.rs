//! The Lua frame a host function was called from.
//!
//! # Responsibilities
//!
//! - [`caller`] walks the Lua stack up from the host function that asks and stops at the first
//!   frame that has a current line, which is the nearest Lua frame. A call made through `pcall`
//!   is therefore located at the `pcall` line, because `pcall` itself has no line.
//! - [`Caller`] is that frame as a path inside the extension and a line. It gives the
//!   [`Location`] a refusal is reported at, `<tool>/<file>:<line>`, and the [`Origin`] an
//!   accepted operation is recorded with.
//! - [`locate`] is the location of the nearest Lua frame, or the tool alone when no Lua frame is
//!   on the stack.
//!
//! # Non-responsibilities
//!
//! Naming a chunk is the loader's work: a frame belongs to a file only because the chunk was
//! loaded under `@<path>`.

use std::borrow::Cow;
use std::num::NonZeroUsize;

use airsl::mlua::Lua;

use crate::types::name::ToolName;
use crate::types::op::Origin;
use crate::types::report::Location;
use crate::types::tree::TreePath;

/// A call site in a file of the extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Caller {
    file: TreePath,
    line: NonZeroUsize,
}

impl Caller {
    /// Where a refusal of this call is reported: `<tool>/<file>:<line>`.
    pub(super) fn location(&self, tool: &ToolName) -> Location {
        Location::at_line(format!("{tool}/{}", self.file), self.line)
    }

    /// Who an operation proposed by this call is recorded as coming from.
    pub(super) fn origin(&self, tool: &ToolName) -> Origin {
        Origin::Extension {
            tool: tool.clone(),
            file: self.file.clone(),
            line: self.line,
        }
    }
}

/// The nearest Lua frame below the host function that is running, or `None` when the stack holds
/// none that names a file.
///
/// The file is the frame's chunk name without its leading `@`, the form every chunk of the
/// extension is loaded under.
pub(super) fn caller(lua: &Lua) -> Option<Caller> {
    // Level 0 is the host function itself.
    let mut level = 1;
    loop {
        let (source, line) = lua.inspect_stack(level, |frame| {
            let source = frame.source().source.map(Cow::into_owned);
            (source, frame.current_line())
        })?;
        let named = source
            .as_deref()
            .and_then(|source| source.strip_prefix('@'));
        if let (Some(file), Some(line)) = (named, line.and_then(NonZeroUsize::new)) {
            return Some(Caller {
                file: TreePath::from_parts(&[file]),
                line,
            });
        }
        level += 1;
    }
}

/// The location of the nearest Lua frame, or `tool` alone when there is none.
pub(super) fn locate(lua: &Lua, tool: &ToolName) -> Location {
    caller(lua).map_or_else(
        || Location::file(tool.as_str()),
        |caller| caller.location(tool),
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use airsl::Script;
    use serde_json::{Value, json};

    use super::*;
    use crate::host::engine::build_engine;
    use crate::host::events::Event;
    use crate::host::testing::Probe;

    type TestResult = Result<(), Box<dyn Error>>;

    fn engine() -> Result<airsl::Engine, Box<dyn Error>> {
        Ok(build_engine(Probe::new(ToolName::new("claude")?)?)?)
    }

    fn entry(engine: &airsl::Engine, source: &str) -> TestResult {
        engine.eval(&Script::from_source(source, "adapter.lua")?)?;
        Ok(())
    }

    fn string_of(value: Option<Value>) -> Result<String, Box<dyn Error>> {
        match value {
            Some(Value::String(text)) => Ok(text),
            other => Err(format!("expected a string, got {other:?}").into()),
        }
    }

    #[test]
    fn names_the_entry_file_and_line_at_top_level() -> TestResult {
        let engine = engine()?;
        let shown: String = engine.eval_to(&Script::from_source(
            "local a = 1\nlocal b = 2\nreturn airsstack.probe.where()",
            "adapter.lua",
        )?)?;
        assert_eq!(shown, "claude/adapter.lua:3");
        Ok(())
    }

    #[test]
    fn names_the_line_inside_a_dispatched_handler() -> TestResult {
        let engine = engine()?;
        entry(
            &engine,
            "airsstack.ext.on('plan', function()\n  return airsstack.probe.where()\nend)",
        )?;
        let shown = engine.dispatch(&Event::Plan.name()?, &json!({}))?;
        assert_eq!(string_of(shown)?, "claude/adapter.lua:2");
        Ok(())
    }

    #[test]
    fn names_the_required_module_not_the_entry() -> TestResult {
        let engine = engine()?;
        let shown: String = engine.eval_to(&Script::from_source(
            "local m = airsstack.probe.module('local M = {}\\nfunction M.go()\\n  return airsstack.probe.where()\\nend\\nreturn M', 'lib/settings.lua')\nreturn m.go()",
            "adapter.lua",
        )?)?;
        assert_eq!(shown, "claude/lib/settings.lua:3");
        Ok(())
    }

    #[test]
    fn skips_a_c_frame_to_the_pcall_line() -> TestResult {
        let engine = engine()?;
        let shown: String = engine.eval_to(&Script::from_source(
            "local a = 1\nlocal b = 2\nlocal c = 3\nlocal _, where = pcall(airsstack.probe.where)\nreturn where",
            "adapter.lua",
        )?)?;
        assert_eq!(shown, "claude/adapter.lua:4");
        Ok(())
    }

    #[test]
    fn gives_the_origin_of_the_call() -> TestResult {
        let engine = engine()?;
        let shown: String = engine.eval_to(&Script::from_source(
            "\nreturn airsstack.probe.origin()",
            "adapter.lua",
        )?)?;
        assert_eq!(shown, "claude/adapter.lua:2|adapter.lua:2");
        Ok(())
    }
}
