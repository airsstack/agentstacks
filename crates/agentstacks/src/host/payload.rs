//! The JSON an extension's `values` and `plan` handlers are called with.
//!
//! # Responsibilities
//!
//! - [`values_payload`] is the payload of the `values` handler: the API number, the tool, the
//!   packs with the skills and agents each holds, and the vars the packs use.
//! - [`plan_payload`] is the payload of the `plan` handler: the API number, the tool, the root,
//!   and every pack with its skills, agents, hooks and native files and hooks, each with the id
//!   the extension copies back.
//!   - A skill has its install directory, its frontmatter fields as `meta` and its files.
//!   - A file has `text` only when it is UTF-8. A binary file is installed with `copy`.
//!   - A hook has its `kind`, `lua` or `message`, and the fields of that kind. `match` and
//!     `agents` are lists, and are left out when the hook has none.
//!   - `meta.native` holds only the entry of this tool.
//!
//! # Non-responsibilities
//!
//! It translates and decides nothing: every value comes from the input, which the plan flow
//! builds and renders.

use serde_json::{Map, Number, Value, json};

use crate::types::api::API;
use crate::types::format::YamlValue;
use crate::types::frontmatter::{AgentMeta, SkillMeta};
use crate::types::name::{ItemName, ToolName};
use crate::types::op::{
    PlanAction, PlanAgent, PlanFile, PlanHook, PlanInput, PlanNativeHook, PlanPack, PlanSkill,
    ValuesInput, ValuesPack,
};

/// The payload of the `values` handler.
pub(super) fn values_payload(input: &ValuesInput) -> Value {
    json!({
        "api": API,
        "tool": input.tool.as_str(),
        "packs": input.packs.iter().map(values_pack).collect::<Vec<_>>(),
        "vars": input.vars.iter().map(|var| var.as_str()).collect::<Vec<_>>(),
    })
}

/// One pack of the `values` payload.
fn values_pack(pack: &ValuesPack) -> Value {
    json!({
        "name": pack.name.as_str(),
        "version": pack.version.to_string(),
        "skills": pack.skills.iter().map(ItemName::as_str).collect::<Vec<_>>(),
        "agents": pack.agents.iter().map(ItemName::as_str).collect::<Vec<_>>(),
    })
}

/// The payload of the `plan` handler.
pub(super) fn plan_payload(input: &PlanInput) -> Value {
    json!({
        "api": API,
        "tool": input.tool.as_str(),
        "root": input.root.as_str(),
        "packs": input
            .packs
            .iter()
            .map(|pack| plan_pack(pack, &input.tool))
            .collect::<Vec<_>>(),
    })
}

/// One pack of the `plan` payload, whose `meta.native` is `tool`'s entry.
fn plan_pack(pack: &PlanPack, tool: &ToolName) -> Value {
    json!({
        "name": pack.name.as_str(),
        "version": pack.version.to_string(),
        "skills": pack.skills.iter().map(|skill| skill_entry(skill, tool)).collect::<Vec<_>>(),
        "agents": pack.agents.iter().map(|agent| agent_entry(agent, tool)).collect::<Vec<_>>(),
        "hooks": pack.hooks.iter().map(hook_entry).collect::<Vec<_>>(),
        "native": {
            "files": pack.native.files.iter().map(file_entry).collect::<Vec<_>>(),
            "hooks": pack.native.hooks.iter().map(native_hook_entry).collect::<Vec<_>>(),
        },
    })
}

/// A skill with its install directory, its frontmatter fields and its files.
fn skill_entry(skill: &PlanSkill, tool: &ToolName) -> Value {
    json!({
        "id": skill.id.as_str(),
        "name": skill.name.as_str(),
        "dir": skill.dir.as_str(),
        "meta": skill_meta(&skill.meta, tool),
        "files": skill.files.iter().map(file_entry).collect::<Vec<_>>(),
    })
}

/// The frontmatter fields of a skill. A field that is not set is left out.
fn skill_meta(meta: &SkillMeta, tool: &ToolName) -> Value {
    let mut fields = Map::new();
    fields.insert("description".to_owned(), json!(meta.description()));
    fields.insert("invocation".to_owned(), json!(meta.invocation().as_str()));
    if let Some(hint) = meta.argument_hint() {
        fields.insert("argument_hint".to_owned(), json!(hint));
    }
    if let Some(native) = meta.native().get(tool) {
        fields.insert("native".to_owned(), yaml_json(native));
    }
    Value::Object(fields)
}

/// An agent with its frontmatter fields and its body.
fn agent_entry(agent: &PlanAgent, tool: &ToolName) -> Value {
    json!({
        "id": agent.id.as_str(),
        "name": agent.name.as_str(),
        "meta": agent_meta(&agent.meta, tool),
        "text": agent.text,
    })
}

/// The frontmatter fields of an agent. A field that is not set is left out.
fn agent_meta(meta: &AgentMeta, tool: &ToolName) -> Value {
    let mut fields = Map::new();
    fields.insert("description".to_owned(), json!(meta.description()));
    if let Some(model) = meta.model() {
        fields.insert("model".to_owned(), json!(model.as_str()));
    }
    if let Some(effort) = meta.effort() {
        fields.insert("effort".to_owned(), json!(effort.as_str()));
    }
    if let Some(tools) = meta.tools() {
        let named: Vec<&str> = tools.iter().map(|tool| tool.as_str()).collect();
        fields.insert("tools".to_owned(), json!(named));
    }
    if let Some(native) = meta.native().get(tool) {
        fields.insert("native".to_owned(), yaml_json(native));
    }
    Value::Object(fields)
}

/// A file with its id and its path. `text` is there only for a file that is UTF-8.
fn file_entry(file: &PlanFile) -> Value {
    let mut fields = Map::new();
    fields.insert("id".to_owned(), json!(file.id.as_str()));
    fields.insert("rel".to_owned(), json!(file.rel.as_str()));
    if let Some(text) = &file.text {
        fields.insert("text".to_owned(), json!(text));
    }
    Value::Object(fields)
}

/// A neutral hook.
fn hook_entry(hook: &PlanHook) -> Value {
    let mut fields = Map::new();
    fields.insert("id".to_owned(), json!(hook.id.as_str()));
    fields.insert("kind".to_owned(), json!(kind(&hook.action)));
    fields.insert("event".to_owned(), json!(hook.event.as_str()));
    if let Some(matches) = &hook.matches {
        let listed: Vec<&str> = matches.iter().map(|value| value.as_str()).collect();
        fields.insert("match".to_owned(), json!(listed));
    }
    if let Some(agents) = &hook.agents {
        let listed: Vec<&str> = agents.iter().map(ItemName::as_str).collect();
        fields.insert("agents".to_owned(), json!(listed));
    }
    action_fields(&hook.action, &mut fields);
    Value::Object(fields)
}

/// A hook of the tool's own, which names its event as the tool does.
fn native_hook_entry(hook: &PlanNativeHook) -> Value {
    let mut fields = Map::new();
    fields.insert("id".to_owned(), json!(hook.id.as_str()));
    fields.insert("kind".to_owned(), json!(kind(&hook.action)));
    fields.insert("event".to_owned(), json!(hook.event));
    if let Some(matches) = &hook.matches {
        fields.insert("match".to_owned(), json!(matches));
    }
    action_fields(&hook.action, &mut fields);
    Value::Object(fields)
}

/// The word that tells what a hook does.
const fn kind(action: &PlanAction) -> &'static str {
    match action {
        PlanAction::Lua { .. } => "lua",
        PlanAction::Message { .. } => "message",
    }
}

/// Adds the fields of `action` to `fields`.
fn action_fields(action: &PlanAction, fields: &mut Map<String, Value>) {
    match action {
        PlanAction::Lua { may_block, command } => {
            fields.insert("may_block".to_owned(), json!(may_block));
            fields.insert("command".to_owned(), json!(command));
        }
        PlanAction::Message { text } => {
            fields.insert("text".to_owned(), json!(text));
        }
    }
}

/// A YAML value as JSON. A float that JSON cannot hold becomes `null`.
fn yaml_json(value: &YamlValue) -> Value {
    match value {
        YamlValue::Null => Value::Null,
        YamlValue::Bool(flag) => Value::Bool(*flag),
        YamlValue::Int(number) => Value::from(*number),
        YamlValue::Float(number) => Number::from_f64(*number).map_or(Value::Null, Value::Number),
        YamlValue::Str(text) => Value::String(text.clone()),
        YamlValue::Seq(items) => Value::Array(items.iter().map(yaml_json).collect()),
        YamlValue::Map(entries) => Value::Object(
            entries
                .iter()
                .map(|(key, entry)| (key.clone(), yaml_json(entry)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::*;
    use crate::host::testing::{plan_input, values_input};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn values_payload_matches_the_spec_example() -> TestResult {
        assert_eq!(
            values_payload(&values_input()?),
            json!({
                "api": 0,
                "tool": "claude",
                "packs": [
                    {"name": "core", "version": "0.3.0", "skills": ["status"], "agents": ["coder"]}
                ],
                "vars": ["session_id", "tmpdir"]
            })
        );
        Ok(())
    }

    #[test]
    fn plan_payload_matches_the_spec_example() -> TestResult {
        assert_eq!(
            plan_payload(&plan_input()?),
            json!({
                "api": 0,
                "tool": "claude",
                "root": ".claude/agentstacks",
                "packs": [{
                    "name": "core",
                    "version": "0.3.0",
                    "skills": [{
                        "id": "core/skill/status",
                        "name": "status",
                        "dir": ".claude/skills/status",
                        "meta": {"description": "Shows the status", "invocation": "user"},
                        "files": [
                            {"id": "core/skill/status/SKILL.md", "rel": "SKILL.md", "text": "body"},
                            {"id": "core/skill/status/logo.png", "rel": "logo.png"}
                        ]
                    }],
                    "agents": [{
                        "id": "core/agent/coder",
                        "name": "coder",
                        "meta": {"description": "Writes code", "model": "balanced"},
                        "text": "body"
                    }],
                    "hooks": [
                        {
                            "id": "core/hook/0",
                            "kind": "lua",
                            "event": "tool.before",
                            "match": ["read", "edit", "write"],
                            "may_block": false,
                            "command": ["airsl", "run", "x"]
                        },
                        {
                            "id": "core/hook/1",
                            "kind": "message",
                            "event": "session.end",
                            "text": "Reminder"
                        }
                    ],
                    "native": {
                        "files": [{"id": "core/native/claude/x.md", "rel": "x.md", "text": "x"}],
                        "hooks": [{
                            "id": "core/native-hook/0",
                            "kind": "lua",
                            "event": "SubagentStop",
                            "match": ["Edit", "Write"],
                            "may_block": false,
                            "command": ["airsl", "run", "y"]
                        }]
                    }
                }]
            })
        );
        Ok(())
    }

    #[test]
    fn native_hook_match_is_the_readers_list() -> TestResult {
        let payload = plan_payload(&plan_input()?);
        let matched = payload.pointer("/packs/0/native/hooks/0/match");
        assert_eq!(matched, Some(&json!(["Edit", "Write"])));
        Ok(())
    }

    #[test]
    fn a_hook_with_no_match_or_agents_leaves_the_keys_out() -> TestResult {
        let payload = plan_payload(&plan_input()?);
        let hook = payload
            .pointer("/packs/0/hooks/1")
            .and_then(Value::as_object);
        let keys: Vec<&str> = hook
            .into_iter()
            .flatten()
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(keys, ["id", "kind", "event", "text"]);
        Ok(())
    }

    #[test]
    fn a_pack_with_nothing_has_empty_lists() -> TestResult {
        let mut input = plan_input()?;
        input.packs.clear();
        assert_eq!(
            plan_payload(&input),
            json!({"api": 0, "tool": "claude", "root": ".claude/agentstacks", "packs": []})
        );
        Ok(())
    }

    #[test]
    fn meta_native_holds_only_this_tools_entry() -> TestResult {
        let text = "---\nname: status\ndescription: d\nargument_hint: <x>\nnative:\n  claude:\n    allowed: [a, b]\n  opencode:\n    z: 1\n---\nbody\n";
        let name = crate::types::name::ItemName::new("status")?;
        let meta = SkillMeta::parse(std::path::Path::new("SKILL.md"), text, &name)
            .map_err(|found| format!("{found:?}"))?;
        assert_eq!(
            skill_meta(&meta, &ToolName::new("claude")?),
            json!({
                "description": "d",
                "invocation": "any",
                "argument_hint": "<x>",
                "native": {"allowed": ["a", "b"]}
            })
        );
        assert_eq!(
            skill_meta(&meta, &ToolName::new("other")?).get("native"),
            None
        );
        Ok(())
    }

    #[test]
    fn an_agent_lists_its_effort_and_tools_when_it_has_them() -> TestResult {
        let text =
            "---\nname: coder\ndescription: d\neffort: high\ntools: [read, shell]\n---\nbody\n";
        let name = crate::types::name::ItemName::new("coder")?;
        let meta = AgentMeta::parse(std::path::Path::new("coder.md"), text, &name)
            .map_err(|found| format!("{found:?}"))?;
        assert_eq!(
            agent_meta(&meta, &ToolName::new("claude")?),
            json!({"description": "d", "effort": "high", "tools": ["read", "shell"]})
        );
        Ok(())
    }
}
