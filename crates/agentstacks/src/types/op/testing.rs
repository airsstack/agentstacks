//! Builders the unit tests of the operation rules share.
//!
//! # Responsibilities
//!
//! - [`load`] builds a pack from an in-memory directory through [`Pack::load`], so no test builds
//!   a pack by hand.
//! - [`plan_input`] is the plan payload of one pack with every kind of item, written out from
//!   typed parts.
//! - [`values_input`] and [`values_result`] are the `values` payload of that pack and the
//!   answer an extension gives to it, and [`tokens`] scans the tokens of a text.
//! - [`cover`], [`skip_all`], [`write_path`] and [`record_runtime`] fill a [`Proposals`] the way
//!   an extension and the core would.
//!
//! # Non-responsibilities
//!
//! Compiled for tests only; it asserts nothing itself.

use std::error::Error;
use std::num::NonZeroUsize;
use std::path::Path;

use serde_json::Value;

use crate::port::FakeFileTree;
use crate::types::frontmatter::{AgentMeta, SkillMeta};
use crate::types::hooks::HookEvent;
use crate::types::manifest::PackVersion;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::answer::ValuesResult;
use crate::types::op::payload::{
    PlanAction, PlanAgent, PlanFile, PlanHook, PlanInput, PlanNative, PlanNativeHook, PlanPack,
    PlanSkill, ValuesInput,
};
use crate::types::op::{
    ItemId, LuaType, OpPath, Operation, Origin, Proposal, Proposals, SkipReason,
};
use crate::types::pack::Pack;
use crate::types::tokens::{TextOrigin, Token, scan};
use crate::types::tree::TreePath;

/// A Markdown file whose frontmatter names `name`, followed by `tail`.
pub(super) fn doc(name: &str, tail: &str) -> String {
    format!("---\nname: {name}\ndescription: d\n---\n\n{tail}\n")
}

/// The pack `name` with `tables` appended to its `pack.toml` and the files `files` (path and
/// text), loaded from an in-memory directory.
pub(super) fn load(
    name: &str,
    tables: &str,
    files: &[(&str, &str)],
) -> Result<Pack, Box<dyn Error>> {
    let manifest = format!(
        "[pack]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 0\ndescription = \"d\"\n{tables}"
    );
    let mut tree =
        FakeFileTree::new(format!("packs/{name}")).file("pack.toml", manifest.as_bytes());
    for (path, text) in files {
        tree = tree.file(path, text.as_bytes());
    }
    Pack::load(&tree).map_err(|found| {
        let shown: Vec<String> = found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        shown.join("; ").into()
    })
}

/// The Lua type word of a JSON value.
pub(super) fn json_word(value: &Value) -> LuaType {
    match value {
        Value::Null => LuaType::Nil,
        Value::Bool(_) => LuaType::Boolean,
        Value::Number(_) => LuaType::Number,
        Value::String(_) => LuaType::String,
        Value::Array(_) => LuaType::List,
        Value::Object(_) => LuaType::Table,
    }
}

/// The `values` payload of the pack `core` (skill `status`, agent `coder`, vars `session_id`
/// and `tmpdir`) for the tool `claude`.
pub(super) fn values_input() -> Result<ValuesInput, Box<dyn Error>> {
    let skill = doc("status", "{{var tmpdir}} {{var session_id}}");
    let pack = load(
        "core",
        "",
        &[
            ("skills/status/SKILL.md", &skill),
            ("agents/coder.md", &doc("coder", "x")),
        ],
    )?;
    Ok(ValuesInput::new(ToolName::new("claude")?, &[&pack]))
}

/// The `values` answer of `claude` to [`values_input`].
pub(super) fn values_result() -> Result<ValuesResult, Box<dyn Error>> {
    let text = r#"{"agents":{"core/coder":"coder"},"root":".claude/agentstacks","skills":{"core/status":{"dir":".claude/skills/status","invoke":"/status"}},"vars":{"tmpdir":"$TMPDIR"}}"#;
    let value: Value = serde_json::from_str(text)?;
    let object = value.as_object().ok_or("an object")?;
    ValuesResult::read(
        object,
        &values_input()?,
        Path::new("claude/adapter.lua"),
        json_word,
    )
    .map_err(|found| format!("{found:?}").into())
}

/// The tokens of `text`, in text order.
pub(super) fn tokens(text: &str) -> Vec<Token> {
    let origin = TextOrigin::Lines {
        file: "x.md".into(),
        first_line: NonZeroUsize::MIN,
    };
    let (found, _) = scan(text, &origin);
    found
        .iter()
        .map(|located| located.token().clone())
        .collect()
}

/// The place `tool`'s extension makes its `n`th call at: `adapter.lua`, line `n`.
fn call_at(tool: &str, n: usize) -> Result<Origin, Box<dyn Error>> {
    Ok(Origin::Extension {
        tool: ToolName::new(tool)?,
        file: rel(&["adapter.lua"]),
        line: NonZeroUsize::new(n).ok_or("a line starts at 1")?,
    })
}

/// Proposes, for `tool`, one write per id of `ids` (their text, as in `input`) covering it.
pub(super) fn cover(
    proposals: &mut Proposals,
    input: &PlanInput,
    tool: &str,
    ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for (index, text) in ids.iter().enumerate() {
        let op = Operation::Write {
            path: OpPath::new(&format!(".{tool}/out/{text}"))?,
            content: Vec::new(),
            covers: vec![input.cover(1, text)?],
        };
        let origin = call_at(tool, index + 1)?;
        proposals.propose(Proposal { op, origin })?;
    }
    Ok(())
}

/// Proposes, for `tool`, a skip of each id of `ids` (their text, as in `input`).
pub(super) fn skip_all(
    proposals: &mut Proposals,
    input: &PlanInput,
    tool: &str,
    ids: &[&str],
) -> Result<(), Box<dyn Error>> {
    for (index, text) in ids.iter().enumerate() {
        let op = Operation::Skip {
            item: input.skip_item(text)?,
            reason: SkipReason::new("not for this tool")?,
        };
        let origin = call_at(tool, index + 1)?;
        proposals.propose(Proposal { op, origin })?;
    }
    Ok(())
}

/// Proposes, for `tool`, a write of `path`, which covers nothing.
pub(super) fn write_path(
    proposals: &mut Proposals,
    tool: &str,
    path: &str,
) -> Result<(), Box<dyn Error>> {
    let op = Operation::Write {
        path: OpPath::new(path)?,
        content: Vec::new(),
        covers: Vec::new(),
    };
    proposals.propose(Proposal {
        op,
        origin: call_at(tool, 1)?,
    })?;
    Ok(())
}

/// Records the core's write of the hook runtime of `tool` at `<root>/runtime.lua`.
pub(super) fn record_runtime(
    proposals: &mut Proposals,
    tool: &str,
    root: &OpPath,
) -> Result<(), Box<dyn Error>> {
    let tool = ToolName::new(tool)?;
    let op = Operation::Write {
        path: root.join("runtime.lua"),
        content: Vec::new(),
        covers: Vec::new(),
    };
    let origin = Origin::Tool { tool: tool.clone() };
    proposals
        .record_core(
            &tool,
            Proposal { op, origin },
            Path::new("agentstacks.toml"),
        )
        .map_err(|found| found.to_string())?;
    Ok(())
}

/// The path `parts` joined with `/`.
pub(super) fn rel(parts: &[&str]) -> TreePath {
    TreePath::from_parts(parts)
}

/// A Lua action that may not block.
pub(super) fn lua_action() -> PlanAction {
    PlanAction::Lua {
        may_block: false,
        command: vec!["airsl".to_owned(), "run".to_owned()],
    }
}

/// The plan payload of the pack `core` for the tool `claude`, with root `.claude/agentstacks`:
/// the skill `status` (`SKILL.md` and a binary `logo.png`), the agent `coder`, the neutral hooks
/// 0 (lua) and 1 (message), the native file `x.md` and the native hook 0 (lua).
pub(super) fn plan_input() -> Result<PlanInput, Box<dyn Error>> {
    let core = PackName::new("core")?;
    let status = ItemName::new("status")?;
    let coder = ItemName::new("coder")?;
    let claude = ToolName::new("claude")?;
    let skill_text = doc("status", "body");
    let skill = PlanSkill {
        id: ItemId::skill(&core, &status),
        name: status.clone(),
        dir: OpPath::new(".claude/skills/status")?,
        meta: SkillMeta::parse(Path::new("SKILL.md"), &skill_text, &status)
            .map_err(|found| format!("{found:?}"))?,
        files: vec![
            PlanFile {
                id: ItemId::skill_file(&core, &status, &rel(&["SKILL.md"])),
                rel: rel(&["SKILL.md"]),
                text: Some("body".to_owned()),
            },
            PlanFile {
                id: ItemId::skill_file(&core, &status, &rel(&["logo.png"])),
                rel: rel(&["logo.png"]),
                text: None,
            },
        ],
    };
    let agent_text = doc("coder", "body");
    let agent = PlanAgent {
        id: ItemId::agent(&core, &coder),
        name: coder.clone(),
        meta: AgentMeta::parse(Path::new("coder.md"), &agent_text, &coder)
            .map_err(|found| format!("{found:?}"))?,
        text: "body".to_owned(),
    };
    let hooks = vec![
        PlanHook {
            id: ItemId::hook(&core, 0),
            event: HookEvent::SessionEnd,
            matches: None,
            agents: None,
            action: lua_action(),
        },
        PlanHook {
            id: ItemId::hook(&core, 1),
            event: HookEvent::SessionEnd,
            matches: None,
            agents: None,
            action: PlanAction::Message {
                text: "hi".to_owned(),
            },
        },
    ];
    let native = PlanNative {
        files: vec![PlanFile {
            id: ItemId::native_file(&core, &claude, &rel(&["x.md"])),
            rel: rel(&["x.md"]),
            text: Some("x".to_owned()),
        }],
        hooks: vec![PlanNativeHook {
            id: ItemId::native_hook(&core, 0),
            event: "Stop".to_owned(),
            matches: None,
            action: lua_action(),
        }],
    };
    Ok(PlanInput {
        tool: claude,
        root: OpPath::new(".claude/agentstacks")?,
        packs: vec![PlanPack {
            name: core,
            version: PackVersion::new("0.1.0")?,
            skills: vec![skill],
            agents: vec![agent],
            hooks,
            native,
        }],
    })
}
