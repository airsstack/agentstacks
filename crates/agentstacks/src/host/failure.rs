//! How a failure inside the sandbox becomes a refusal the user reads.
//!
//! # Responsibilities
//!
//! - [`Raised`] is the error the host's own Lua functions raise. Its text is the located refusal,
//!   so Lua code that catches it with `pcall` sees the same words the user would. It is either
//!   recorded already, so the run is refused even when the extension catches it, or ordinary,
//!   so the refusal appears only when nothing catches it.
//! - [`wrap`] turns the engine's error from an entry run or a dispatch into the refusal to show:
//!   nothing more for a recorded [`Raised`], the [`Raised`]'s own located refusal for an ordinary
//!   one, and for anything else the first line of the engine's text at `<tool>/<entry>`. Resource
//!   breaches and syntax errors are the latter, so they read in the engine's own words.
//! - [`lua_failure`] is that last wording for any error. The session falls back to it when a
//!   failed call left no refusal to show, as when the extension kept an ordinary [`Raised`] that
//!   an earlier call already handed out and raised it again.
//!
//! # Non-responsibilities
//!
//! Deciding that a call is refused, and with which message, belongs to the function that raises.

use std::error::Error;
use std::fmt;
use std::sync::{Mutex, PoisonError};

use airsl::mlua;

use crate::types::name::ToolName;
use crate::types::op::OpError;
use crate::types::report::{Diagnostic, Location};
use crate::types::tree::TreePath;

/// A refusal raised by a host function, carried through Lua as an error value.
///
/// An ordinary refusal holds its diagnostic until [`wrap`] takes it. A recorded one holds none,
/// because the run's state already has it.
#[derive(Debug)]
pub(super) struct Raised {
    text: String,
    diagnostic: Mutex<Option<Diagnostic>>,
}

impl Raised {
    /// A refusal that is an ordinary Lua error: it fails the run only when nothing catches it.
    pub(super) fn ordinary(diagnostic: Diagnostic) -> Self {
        Self {
            text: diagnostic.to_string(),
            diagnostic: Mutex::new(Some(diagnostic)),
        }
    }

    /// A refusal whose diagnostic is already recorded, with `text` as the message Lua sees.
    pub(super) const fn recorded(text: String) -> Self {
        Self {
            text,
            diagnostic: Mutex::new(None),
        }
    }

    /// This refusal as the error value of a Lua call.
    pub(super) fn into_lua(self) -> mlua::Error {
        mlua::Error::external(self)
    }

    /// The diagnostic of an ordinary refusal, handed out once.
    fn take(&self) -> Option<Diagnostic> {
        self.diagnostic
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

impl fmt::Display for Raised {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Error for Raised {}

/// The refusal to show for `error`, which an entry run or a dispatch of `tool`'s extension
/// returned, or `None` when the refusal is already recorded or was already handed out.
///
/// `entry` is the extension's entry file. An ordinary [`Raised`] hands out its diagnostic once, so
/// a second call on the same error gives `None`; [`lua_failure`] words such an error when nothing
/// else says it failed.
pub(super) fn wrap(tool: &ToolName, entry: &TreePath, error: &airsl::Error) -> Option<Diagnostic> {
    if let Some(found) = raised(error) {
        return found.take();
    }
    Some(lua_failure(tool, entry, error))
}

/// `error` as a failure of the extension's Lua code: the first line of the engine's text at
/// `<tool>/<entry>`, whatever the error is.
pub(super) fn lua_failure(tool: &ToolName, entry: &TreePath, error: &airsl::Error) -> Diagnostic {
    let text = error.to_string();
    let first_line = text.lines().next().unwrap_or_default().to_owned();
    Diagnostic::new(
        Location::file(format!("{tool}/{entry}")),
        OpError::Lua { first_line },
    )
}

/// The [`Raised`] somewhere in `error`'s chain of causes, if a host function raised one.
fn raised(error: &airsl::Error) -> Option<&Raised> {
    let mut link: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(current) = link {
        let found = lua_error(current).and_then(mlua::Error::downcast_ref::<Raised>);
        if found.is_some() {
            return found;
        }
        link = current.source();
    }
    None
}

/// `link` as the Lua error it is, whether the engine boxed it or not.
fn lua_error<'a>(link: &'a (dyn Error + 'static)) -> Option<&'a mlua::Error> {
    link.downcast_ref::<mlua::Error>().or_else(|| {
        link.downcast_ref::<Box<mlua::Error>>()
            .map(|boxed| &**boxed)
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use airsl::Script;

    use super::*;
    use crate::host::engine::build_engine;
    use crate::host::testing::Probe;
    use crate::types::report::FieldError;

    type TestResult = Result<(), Box<dyn Error>>;

    fn tool() -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new("claude")?)
    }

    fn entry() -> TreePath {
        TreePath::from_parts(&["adapter.lua"])
    }

    fn failure(source: &str) -> Result<airsl::Error, Box<dyn Error>> {
        let engine = build_engine(Probe::new(tool()?)?)?;
        engine
            .eval(&Script::from_source(source, "adapter.lua")?)
            .err()
            .ok_or_else(|| "the script should fail".into())
    }

    #[test]
    fn a_recorded_refusal_adds_nothing() -> TestResult {
        let error = failure("airsstack.probe.raise_recorded('already recorded')")?;
        assert!(wrap(&tool()?, &entry(), &error).is_none());
        Ok(())
    }

    #[test]
    fn an_ordinary_refusal_is_its_own_located_diagnostic() -> TestResult {
        let error = failure("\nairsstack.probe.raise_ordinary()")?;
        let shown = wrap(&tool()?, &entry(), &error).map(|d| d.to_string());
        assert_eq!(shown.as_deref(), Some("claude/adapter.lua:2: required"));
        Ok(())
    }

    #[test]
    fn any_other_failure_is_the_first_line_at_the_entry() -> TestResult {
        let error = failure("error('path must be relative')")?;
        let shown = wrap(&tool()?, &entry(), &error).map(|d| d.to_string());
        assert_eq!(
            shown.as_deref(),
            Some(
                "claude/adapter.lua: lua error in adapter.lua: runtime error: adapter.lua:1: path must be relative"
            )
        );
        Ok(())
    }

    #[test]
    fn raised_text_is_the_located_refusal() {
        let diagnostic = Diagnostic::new(Location::file("claude/adapter.lua"), FieldError::Empty);
        let raised = Raised::ordinary(diagnostic);
        assert_eq!(raised.to_string(), "claude/adapter.lua: empty");
        assert!(raised.take().is_some());
        assert!(raised.take().is_none());
    }
}
