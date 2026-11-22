//! The `plan` payload: every rendered item an extension must account for.
//!
//! # Responsibilities
//!
//! - [`PlanInput`] holds the run's packs and the root the hook runtime lives under.
//! - It resolves the id an extension copies back, refusing one that is not in the payload, and
//!   a `copy` of something that is not a file, and a `covers` entry that names a whole skill.
//! - It lists the ids that need an operation, and the ids of the hooks that run Lua.
//!
//! # Non-responsibilities
//!
//! Items the core already skipped are never added, so they need no operation. Checking that an
//! extension covered every item belongs to the coverage rule.

use crate::types::frontmatter::AgentMeta;
use crate::types::manifest::PackVersion;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::payload::{PlanFile, PlanHook, PlanNativeHook, PlanSkill};
use crate::types::op::{ApiFunction, ItemId, OpError, OpPath};
use crate::types::report::Quoted;

/// One agent in the plan payload.
#[derive(Debug, Clone)]
pub struct PlanAgent {
    /// The id an extension lists in `covers`.
    pub id: ItemId,
    /// The agent's name.
    pub name: ItemName,
    /// The checked frontmatter of the rendered file.
    pub meta: AgentMeta,
    /// The rendered body.
    pub text: String,
}

/// The files and hooks of a pack's `native/<tool>/` directory for this tool.
#[derive(Debug, Clone, Default)]
pub struct PlanNative {
    /// The files, relative to `native/<tool>/`.
    pub files: Vec<PlanFile>,
    /// The hooks of its `hooks.toml`.
    pub hooks: Vec<PlanNativeHook>,
}

/// One pack in the plan payload.
#[derive(Debug, Clone)]
pub struct PlanPack {
    /// The pack's name.
    pub name: PackName,
    /// The pack's version.
    pub version: PackVersion,
    /// The skills the tool gets.
    pub skills: Vec<PlanSkill>,
    /// The agents the tool gets.
    pub agents: Vec<PlanAgent>,
    /// The neutral hooks.
    pub hooks: Vec<PlanHook>,
    /// This tool's native directory.
    pub native: PlanNative,
}

/// What the core tells an extension to plan.
#[derive(Debug, Clone)]
pub struct PlanInput {
    /// The tool the extension is for.
    pub tool: ToolName,
    /// The directory the extension's root files and the hook runtime are written under.
    pub root: OpPath,
    /// The packs of the run.
    pub packs: Vec<PlanPack>,
}

impl PlanInput {
    /// Every id in the payload, skill groups included.
    fn ids(&self) -> Vec<&ItemId> {
        let mut ids = Vec::new();
        for pack in &self.packs {
            for skill in &pack.skills {
                ids.push(&skill.id);
                ids.extend(skill.files.iter().map(|file| &file.id));
            }
            ids.extend(pack.agents.iter().map(|agent| &agent.id));
            ids.extend(pack.hooks.iter().map(|hook| &hook.id));
            ids.extend(pack.native.files.iter().map(|file| &file.id));
            ids.extend(pack.native.hooks.iter().map(|hook| &hook.id));
        }
        ids
    }

    /// The id `text` names, or the refusal for `function`'s `field`.
    fn find(&self, text: &str, function: ApiFunction, field: &str) -> Result<&ItemId, OpError> {
        self.ids()
            .into_iter()
            .find(|id| id.as_str() == text)
            .ok_or_else(|| OpError::NotInPlan {
                function,
                field: field.to_owned(),
                id: Quoted::new(text),
            })
    }

    /// The file a `copy` names.
    ///
    /// # Errors
    ///
    /// [`OpError::NotInPlan`] when `text` is not an id of the payload, and
    /// [`OpError::NotAFile`] when it is an id that is not a file.
    pub fn copy_file(&self, text: &str) -> Result<ItemId, OpError> {
        let id = self.find(text, ApiFunction::Copy, "file")?;
        if id.is_file() {
            Ok(id.clone())
        } else {
            Err(OpError::NotAFile {
                id: Quoted::new(text),
            })
        }
    }

    /// The item a `skip` names. Any id may be skipped, a whole skill included.
    ///
    /// # Errors
    ///
    /// [`OpError::NotInPlan`] when `text` is not an id of the payload.
    pub fn skip_item(&self, text: &str) -> Result<ItemId, OpError> {
        self.find(text, ApiFunction::Skip, "item").cloned()
    }

    /// The item at 1-based position `index` of a `write`'s `covers`.
    ///
    /// # Errors
    ///
    /// [`OpError::NotInPlan`] when `text` is not an id of the payload, and
    /// [`OpError::CoversSkill`] when it names a whole skill instead of its files.
    pub fn cover(&self, index: usize, text: &str) -> Result<ItemId, OpError> {
        let field = format!("covers[{index}]");
        let id = self.find(text, ApiFunction::Write, &field)?;
        if id.is_skill_group() {
            return Err(OpError::CoversSkill {
                field,
                id: Quoted::new(text),
            });
        }
        Ok(id.clone())
    }

    /// Every id that needs an operation, in id order: skill files, agents, hooks, native files
    /// and native hooks, never a whole skill.
    #[must_use]
    pub fn items(&self) -> Vec<ItemId> {
        let mut items: Vec<ItemId> = self
            .ids()
            .into_iter()
            .filter(|id| !id.is_skill_group())
            .cloned()
            .collect();
        items.sort();
        items
    }

    /// The ids of the hooks that run Lua, neutral and native, pack by pack.
    #[must_use]
    pub fn lua_hooks(&self) -> Vec<ItemId> {
        let mut ids = Vec::new();
        for pack in &self.packs {
            let neutral = pack.hooks.iter().filter(|hook| hook.action.is_lua());
            ids.extend(neutral.map(|hook| hook.id.clone()));
            let native = pack.native.hooks.iter().filter(|hook| hook.action.is_lua());
            ids.extend(native.map(|hook| hook.id.clone()));
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::op::testing::plan_input;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const IDS: [&str; 7] = [
        "core/agent/coder",
        "core/hook/0",
        "core/hook/1",
        "core/native-hook/0",
        "core/native/claude/x.md",
        "core/skill/status/SKILL.md",
        "core/skill/status/logo.png",
    ];

    fn shown<T: std::fmt::Display>(ids: &[T]) -> Vec<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    fn refusal(result: Result<ItemId, OpError>) -> Result<String, Box<dyn std::error::Error>> {
        Ok(result.err().ok_or("a refusal")?.to_string())
    }

    #[test]
    fn plan_input_resolves_every_payload_id() -> TestResult {
        let input = plan_input()?;
        for text in IDS.iter().chain(&["core/skill/status"]) {
            assert_eq!(input.skip_item(text)?.as_str(), *text);
        }
        for text in ["core/skill/status/SKILL.md", "core/skill/status/logo.png"] {
            assert_eq!(input.copy_file(text)?.as_str(), text);
        }
        for text in IDS {
            assert_eq!(input.cover(1, text)?.as_str(), text);
        }
        Ok(())
    }

    #[test]
    fn plan_input_refuses_an_id_not_in_the_payload() -> TestResult {
        let input = plan_input()?;
        let text = "core/skill/x/SKILL.md";
        assert_eq!(
            refusal(input.copy_file(text))?,
            "copy: file \"core/skill/x/SKILL.md\" is not in this plan"
        );
        assert_eq!(
            refusal(input.skip_item(text))?,
            "skip: item \"core/skill/x/SKILL.md\" is not in this plan"
        );
        assert_eq!(
            refusal(input.cover(1, text))?,
            "write: covers[1] \"core/skill/x/SKILL.md\" is not in this plan"
        );
        assert_eq!(
            refusal(input.cover(3, text))?,
            "write: covers[3] \"core/skill/x/SKILL.md\" is not in this plan"
        );
        Ok(())
    }

    #[test]
    fn plan_input_refuses_copying_a_non_file() -> TestResult {
        let input = plan_input()?;
        assert_eq!(
            refusal(input.copy_file("core/agent/coder"))?,
            "copy: file \"core/agent/coder\" is not a file"
        );
        Ok(())
    }

    #[test]
    fn plan_input_refuses_covering_a_skill_group() -> TestResult {
        let input = plan_input()?;
        assert_eq!(
            refusal(input.cover(1, "core/skill/status"))?,
            "write: covers[1]: \"core/skill/status\" is a skill; list its files"
        );
        Ok(())
    }

    #[test]
    fn plan_input_items_exclude_skill_groups_and_are_sorted() -> TestResult {
        let input = plan_input()?;
        assert_eq!(shown(&input.items()), IDS);
        Ok(())
    }

    #[test]
    fn plan_input_lists_neutral_and_native_lua_hooks() -> TestResult {
        let input = plan_input()?;
        assert_eq!(
            shown(&input.lua_hooks()),
            ["core/hook/0", "core/native-hook/0"]
        );
        Ok(())
    }

    #[test]
    fn payloads_clone_equal() -> TestResult {
        let input = plan_input()?;
        let copy = input.clone();
        assert_eq!(copy.tool, input.tool);
        assert_eq!(copy.root, input.root);
        assert_eq!(shown(&copy.items()), shown(&input.items()));
        let names: Vec<&str> = copy.packs.iter().map(|pack| pack.name.as_str()).collect();
        assert_eq!(names, ["core"]);
        Ok(())
    }
}
