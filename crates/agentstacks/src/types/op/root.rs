//! The files the core writes itself under the extension's root.
//!
//! The root holds each pack's root files and the hook runtime. The core writes them as
//! proposals of its own, so a clash with an extension's path is found the same way as a clash
//! between two extensions.
//!
//! # Responsibilities
//!
//! - [`root_file_write`] is the write of a pack's root file to `<root>/<pack>/<rel>`, owned by
//!   the pack.
//! - [`runtime_write`] is the write of the hook runtime to `<root>/runtime.lua`, owned by the
//!   tool.
//! - [`runtime_needed`] tells whether any pack has a Lua hook for a tool, whether or not the
//!   extension later skips it.
//!
//! # Non-responsibilities
//!
//! The bytes are passed in, and recording the writes in the order a run needs belongs to the
//! plan flow.

use crate::types::hooks::HookAction;
use crate::types::name::{PackName, ToolName};
use crate::types::op::{OpPath, Operation, Origin, Proposal};
use crate::types::pack::Pack;
use crate::types::tree::TreePath;

/// The core's write of the root file `rel` of `pack` under `root`.
#[must_use]
pub fn root_file_write(root: &OpPath, pack: &PackName, rel: &TreePath, bytes: Vec<u8>) -> Proposal {
    Proposal {
        op: Operation::Write {
            path: root.join(pack.as_str()).join(rel.as_str()),
            content: bytes,
            covers: Vec::new(),
        },
        origin: Origin::Core { pack: pack.clone() },
    }
}

/// The core's write of the hook runtime `bytes` for `tool` to `<root>/runtime.lua`.
#[must_use]
pub fn runtime_write(root: &OpPath, tool: &ToolName, bytes: Vec<u8>) -> Proposal {
    Proposal {
        op: Operation::Write {
            path: root.join("runtime.lua"),
            content: bytes,
            covers: Vec::new(),
        },
        origin: Origin::Tool { tool: tool.clone() },
    }
}

/// Whether any of `packs` has a hook that runs Lua for `tool`: a neutral hook, or a hook of the
/// pack's `native/<tool>/hooks.toml`.
#[must_use]
pub fn runtime_needed(packs: &[&Pack], tool: &ToolName) -> bool {
    let is_lua = |action: &HookAction| matches!(action, HookAction::Lua(_));
    packs.iter().any(|pack| {
        let neutral = pack
            .manifest()
            .hooks()
            .iter()
            .any(|hook| is_lua(hook.action()));
        let native = pack
            .native()
            .iter()
            .filter(|dir| dir.tool() == tool)
            .filter_map(|dir| dir.hooks())
            .any(|hooks| hooks.hooks().iter().any(|hook| is_lua(hook.action())));
        neutral || native
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::op::owner;
    use crate::types::op::testing::{doc, load, rel};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const LUA_HOOK: &str = "[requires]\nairsl = \">=0.1.4\"\n\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/enforce.lua\"\n";
    const MESSAGE_HOOK: &str =
        "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"t\"\n";
    const NATIVE_LUA: &str =
        "[[hooks]]\nkind = \"lua\"\nevent = \"Stop\"\nrun = \"hooks/enforce.lua\"\n";
    const NATIVE_MESSAGE: &str = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"t\"\n";

    fn claude() -> Result<ToolName, Box<dyn std::error::Error>> {
        Ok(ToolName::new("claude")?)
    }

    fn root() -> Result<OpPath, Box<dyn std::error::Error>> {
        Ok(OpPath::new(".claude/agentstacks")?)
    }

    fn shown(proposal: &Proposal) -> (Option<String>, Option<String>) {
        (
            proposal.op.path().map(ToString::to_string),
            owner(proposal).map(|owner| owner.to_string()),
        )
    }

    #[test]
    fn root_file_goes_under_root_and_pack() -> TestResult {
        let core = PackName::new("core")?;
        let proposal = root_file_write(
            &root()?,
            &core,
            &rel(&["hooks", "enforce.lua"]),
            b"x".to_vec(),
        );
        assert_eq!(
            shown(&proposal),
            (
                Some(".claude/agentstacks/core/hooks/enforce.lua".to_owned()),
                Some("core".to_owned())
            )
        );
        assert_eq!(proposal.origin, Origin::Core { pack: core });
        let Operation::Write {
            content, covers, ..
        } = &proposal.op
        else {
            return Err("a write".into());
        };
        assert_eq!(content, b"x");
        assert!(covers.is_empty());
        Ok(())
    }

    #[test]
    fn runtime_goes_to_root_runtime_lua() -> TestResult {
        let proposal = runtime_write(&root()?, &claude()?, b"rt".to_vec());
        assert_eq!(
            shown(&proposal),
            (
                Some(".claude/agentstacks/runtime.lua".to_owned()),
                Some("claude".to_owned())
            )
        );
        assert_eq!(proposal.origin, Origin::Tool { tool: claude()? });
        let Operation::Write { content, .. } = &proposal.op else {
            return Err("a write".into());
        };
        assert_eq!(content, b"rt");
        Ok(())
    }

    #[test]
    fn runtime_needed_for_a_neutral_lua_hook() -> TestResult {
        let pack = load("core", LUA_HOOK, &[("hooks/enforce.lua", "return {}\n")])?;
        assert!(runtime_needed(&[&pack], &claude()?));
        Ok(())
    }

    #[test]
    fn runtime_needed_for_a_native_lua_hook_of_this_tool() -> TestResult {
        let pack = load(
            "core",
            "[requires]\nairsl = \">=0.1.4\"\n",
            &[
                ("skills/s/SKILL.md", &doc("s", "x")),
                ("hooks/enforce.lua", "return {}\n"),
                ("native/claude/hooks.toml", NATIVE_LUA),
            ],
        )?;
        assert!(runtime_needed(&[&pack], &claude()?));
        assert!(!runtime_needed(&[&pack], &ToolName::new("opencode")?));
        Ok(())
    }

    #[test]
    fn runtime_not_needed_for_message_hooks_only() -> TestResult {
        let messages = load(
            "core",
            MESSAGE_HOOK,
            &[("native/claude/hooks.toml", NATIVE_MESSAGE)],
        )?;
        assert!(!runtime_needed(&[&messages], &claude()?));
        let none = load("plain", "", &[("skills/s/SKILL.md", &doc("s", "x"))])?;
        assert!(!runtime_needed(&[&none], &claude()?));
        assert!(!runtime_needed(&[], &claude()?));
        Ok(())
    }
}
