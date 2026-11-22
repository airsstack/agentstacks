//! The `values` payload: the packs of a run and what the extension must give values for.
//!
//! # Responsibilities
//!
//! - [`ValuesInput`] lists each pack with its version, skills and agents, and every var a
//!   pack's text uses, once, sorted by name. A tool that is no longer listed gets an input with
//!   no packs and no vars.
//! - It answers whether a skill, an agent or a var is part of the run, which the rules that read
//!   the extension's answer need.
//!
//! # Non-responsibilities
//!
//! Turning the input into the table the extension reads belongs to the extension host.

use crate::types::manifest::PackVersion;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::pack::Pack;
use crate::types::tokens::{TokenTarget, Var};

/// One pack in the `values` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesPack {
    /// The pack's name.
    pub name: PackName,
    /// The pack's version.
    pub version: PackVersion,
    /// The skills the pack ships, in the pack's order.
    pub skills: Vec<ItemName>,
    /// The agents the pack ships, in the pack's order.
    pub agents: Vec<ItemName>,
}

/// What the core tells an extension before it asks for the values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesInput {
    /// The tool the extension is for.
    pub tool: ToolName,
    /// The packs of the run.
    pub packs: Vec<ValuesPack>,
    /// The vars these packs use, sorted by name.
    pub vars: Vec<Var>,
}

impl ValuesInput {
    /// The input for `tool` over `packs`. A removed tool has no packs.
    #[must_use]
    pub fn new(tool: ToolName, packs: &[&Pack]) -> Self {
        let listed = packs
            .iter()
            .map(|pack| ValuesPack {
                name: pack.manifest().name().clone(),
                version: pack.manifest().version().clone(),
                skills: pack
                    .skills()
                    .iter()
                    .map(|skill| skill.name().clone())
                    .collect(),
                agents: pack
                    .agents()
                    .iter()
                    .map(|agent| agent.name().clone())
                    .collect(),
            })
            .collect();
        let mut vars: Vec<Var> = packs
            .iter()
            .flat_map(|pack| pack.tokens())
            .filter_map(|located| match located.token().target() {
                TokenTarget::Var(var) => Some(*var),
                _ => None,
            })
            .collect();
        vars.sort_by_key(|var| var.as_str());
        vars.dedup();
        Self {
            tool,
            packs: listed,
            vars,
        }
    }

    /// Whether the run has the skill `skill` in `pack`.
    #[must_use]
    pub fn has_skill(&self, pack: &PackName, skill: &ItemName) -> bool {
        self.packs
            .iter()
            .any(|listed| &listed.name == pack && listed.skills.contains(skill))
    }

    /// Whether the run has the agent `agent` in `pack`.
    #[must_use]
    pub fn has_agent(&self, pack: &PackName, agent: &ItemName) -> bool {
        self.packs
            .iter()
            .any(|listed| &listed.name == pack && listed.agents.contains(agent))
    }

    /// Whether the run uses `var`.
    #[must_use]
    pub fn has_var(&self, var: Var) -> bool {
        self.vars.contains(&var)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::op::testing::{doc, load};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn core() -> Result<Pack, Box<dyn std::error::Error>> {
        let skill = doc(
            "status",
            "{{var tmpdir}} {{var session_id}}\n{{var tmpdir}}",
        );
        load(
            "core",
            "",
            &[
                ("skills/status/SKILL.md", &skill),
                ("agents/coder.md", &doc("coder", "x")),
            ],
        )
    }

    fn names(listed: &[ItemName]) -> Vec<&str> {
        listed.iter().map(ItemName::as_str).collect()
    }

    #[test]
    fn values_input_lists_each_pack_with_its_skills_and_agents() -> TestResult {
        let pack = core()?;
        let input = ValuesInput::new(ToolName::new("claude")?, &[&pack]);
        assert_eq!(input.tool.as_str(), "claude");
        let [listed] = input.packs.as_slice() else {
            return Err("one pack".into());
        };
        assert_eq!(listed.name.as_str(), "core");
        assert_eq!(listed.version.to_string(), "0.1.0");
        assert_eq!(names(&listed.skills), ["status"]);
        assert_eq!(names(&listed.agents), ["coder"]);
        Ok(())
    }

    #[test]
    fn values_input_lists_used_vars_sorted_by_name() -> TestResult {
        let pack = core()?;
        let input = ValuesInput::new(ToolName::new("claude")?, &[&pack]);
        let vars: Vec<&str> = input.vars.iter().map(|var| var.as_str()).collect();
        assert_eq!(vars, ["session_id", "tmpdir"]);
        Ok(())
    }

    #[test]
    fn values_input_for_a_removed_tool_is_empty() -> TestResult {
        let input = ValuesInput::new(ToolName::new("claude")?, &[]);
        assert!(input.packs.is_empty());
        assert!(input.vars.is_empty());
        Ok(())
    }

    #[test]
    fn values_input_answers_membership() -> TestResult {
        let pack = core()?;
        let input = ValuesInput::new(ToolName::new("claude")?, &[&pack]);
        let core = PackName::new("core")?;
        assert!(input.has_skill(&core, &ItemName::new("status")?));
        assert!(input.has_agent(&core, &ItemName::new("coder")?));
        assert!(input.has_var(Var::Tmpdir));
        assert!(!input.has_skill(&core, &ItemName::new("x")?));
        assert!(!input.has_agent(&core, &ItemName::new("y")?));
        assert!(!input.has_var(Var::Home));
        assert!(!input.has_skill(&PackName::new("other")?, &ItemName::new("status")?));
        Ok(())
    }
}
