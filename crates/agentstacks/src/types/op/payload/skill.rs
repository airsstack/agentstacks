//! A skill, as the plan payload lists it.
//!
//! # Responsibilities
//!
//! - [`PlanSkill`] holds the skill's id and name, the directory the tool installs it in, its
//!   checked frontmatter and its files.
//!
//! # Non-responsibilities
//!
//! Choosing the directory belongs to the tool's extension, which gave it in its `values`.

use crate::types::frontmatter::SkillMeta;
use crate::types::name::ItemName;
use crate::types::op::payload::PlanFile;
use crate::types::op::{ItemId, OpPath};

/// One skill in the plan payload.
#[derive(Debug, Clone)]
pub struct PlanSkill {
    /// The id of the whole skill, which an extension may skip but not cover.
    pub id: ItemId,
    /// The skill's name.
    pub name: ItemName,
    /// The directory the tool installs the skill in.
    pub dir: OpPath,
    /// The checked frontmatter of the rendered `SKILL.md`.
    pub meta: SkillMeta,
    /// The skill's files, `SKILL.md` included.
    pub files: Vec<PlanFile>,
}

#[cfg(test)]
mod tests {
    use crate::types::op::testing::plan_input;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn plan_skill_holds_its_directory_and_files() -> TestResult {
        let input = plan_input()?;
        let [pack] = input.packs.as_slice() else {
            return Err("one pack".into());
        };
        let [skill] = pack.skills.as_slice() else {
            return Err("one skill".into());
        };
        assert_eq!(skill.id.as_str(), "core/skill/status");
        assert_eq!(skill.name.as_str(), "status");
        assert_eq!(skill.dir.as_str(), ".claude/skills/status");
        assert_eq!(skill.meta.name().as_str(), "status");
        let files: Vec<&str> = skill.files.iter().map(|file| file.rel.as_str()).collect();
        assert_eq!(files, ["SKILL.md", "logo.png"]);
        Ok(())
    }
}
