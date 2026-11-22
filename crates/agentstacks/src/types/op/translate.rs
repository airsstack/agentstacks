//! The rule that a tool which runs Lua hooks supplies the translator for them.
//!
//! # Responsibilities
//!
//! - [`check_translate`] applies when the hook runtime is written for the tool and the tool's
//!   extension wrote at least one Lua hook, neutral or native. It then refuses unless the
//!   extension also wrote `<root>/translate.lua`. A hook the extension skipped needs none.
//!
//! # Non-responsibilities
//!
//! Whether the runtime is written at all belongs to the plan flow, which decides it from the
//! packs.

use std::path::Path;

use crate::types::name::ToolName;
use crate::types::op::payload::PlanInput;
use crate::types::op::{OpError, Operation, Proposals};
use crate::types::report::{Diagnostic, Location};

/// The refusal, located at the extension entry `entry`, when `tool` wrote a Lua hook with the
/// runtime but no `translate.lua` under the root of `payload`.
#[must_use]
pub fn check_translate(
    tool: &ToolName,
    entry: &Path,
    payload: &PlanInput,
    proposals: &Proposals,
) -> Option<Diagnostic> {
    if !proposals.has_runtime(tool) {
        return None;
    }
    let lua = payload.lua_hooks();
    let written = proposals.for_tool(tool).any(|proposal| {
        matches!(proposal.op, Operation::Write { .. })
            && proposal.op.covered().iter().any(|id| lua.contains(id))
    });
    if !written || proposals.wrote(tool, &payload.root.join("translate.lua")) {
        return None;
    }
    Some(Diagnostic::new(
        Location::file(entry),
        OpError::TranslateMissing {
            root: payload.root.to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::Path;

    use super::*;
    use crate::types::op::testing::{cover, plan_input, record_runtime, skip_all, write_path};

    type TestResult = Result<(), Box<dyn Error>>;

    const ENTRY: &str = "claude/adapter.lua";
    const MISSING: &str =
        "claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)";

    fn tool() -> Result<ToolName, Box<dyn Error>> {
        Ok(ToolName::new("claude")?)
    }

    fn shown(input: &PlanInput, proposals: &Proposals) -> Result<Option<String>, Box<dyn Error>> {
        let found = check_translate(&tool()?, Path::new(ENTRY), input, proposals);
        Ok(found.map(|diagnostic| diagnostic.to_string()))
    }

    #[test]
    fn lua_hooks_written_without_translate_are_refused() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        record_runtime(&mut proposals, "claude", &input.root)?;
        cover(&mut proposals, &input, "claude", &["core/hook/0"])?;
        assert_eq!(shown(&input, &proposals)?.as_deref(), Some(MISSING));
        Ok(())
    }

    #[test]
    fn native_lua_hook_written_without_translate_is_refused() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        record_runtime(&mut proposals, "claude", &input.root)?;
        cover(&mut proposals, &input, "claude", &["core/native-hook/0"])?;
        assert_eq!(shown(&input, &proposals)?.as_deref(), Some(MISSING));
        Ok(())
    }

    #[test]
    fn lua_hooks_with_translate_pass() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        record_runtime(&mut proposals, "claude", &input.root)?;
        cover(&mut proposals, &input, "claude", &["core/hook/0"])?;
        write_path(
            &mut proposals,
            "claude",
            "./.claude/agentstacks/translate.lua",
        )?;
        assert_eq!(shown(&input, &proposals)?, None);
        Ok(())
    }

    #[test]
    fn skipped_lua_hooks_need_no_translate() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        record_runtime(&mut proposals, "claude", &input.root)?;
        skip_all(
            &mut proposals,
            &input,
            "claude",
            &["core/hook/0", "core/native-hook/0"],
        )?;
        assert_eq!(shown(&input, &proposals)?, None);
        Ok(())
    }

    #[test]
    fn message_hooks_need_no_translate() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        record_runtime(&mut proposals, "claude", &input.root)?;
        cover(&mut proposals, &input, "claude", &["core/hook/1"])?;
        assert_eq!(shown(&input, &proposals)?, None);
        Ok(())
    }

    #[test]
    fn no_runtime_no_translate_rule() -> TestResult {
        let input = plan_input()?;
        let mut proposals = Proposals::new();
        cover(&mut proposals, &input, "claude", &["core/hook/0"])?;
        assert_eq!(shown(&input, &proposals)?, None);
        Ok(())
    }
}
