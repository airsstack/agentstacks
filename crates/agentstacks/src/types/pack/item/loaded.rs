//! The skills and agents a loaded pack ships.
//!
//! It is separate from the loader so the checked shape of a skill and an agent can be read by later
//! stages without the code that built it.
//!
//! # Responsibilities
//!
//! - [`PackSkill`] pairs a skill's checked frontmatter with the files installed from its
//!   directory, listed relative to that directory, `SKILL.md` included. A file that `exclude`
//!   removes is not listed.
//! - [`PackAgent`] pairs an agent's checked frontmatter with the pack-relative path of its file.
//!
//! # Non-responsibilities
//!
//! Reading and checking the frontmatter belongs to the frontmatter parsers, and deciding which
//! files are installed belongs to [`InstallSet`].

use crate::types::frontmatter::AgentMeta;
use crate::types::frontmatter::SkillMeta;
use crate::types::name::ItemName;
use crate::types::pack::install::set::InstallSet;
use crate::types::pack::layout::SKILLS_DIR;
use crate::types::tree::TreePath;

/// A skill of a loaded pack.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{Pack, TreePath};
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-tokens");
/// let pack = Pack::load(&StdFileTree::new(dir)).map_err(|refusals| format!("{refusals:?}"))?;
/// let skill = pack.skills().first().ok_or("the pack ships a skill")?;
/// assert_eq!(skill.name().as_str(), "a");
/// let files: Vec<&str> = skill.files().iter().map(TreePath::as_str).collect();
/// assert_eq!(files, ["SKILL.md"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct PackSkill {
    meta: SkillMeta,
    files: Vec<TreePath>,
}

impl PackSkill {
    /// A skill whose directory holds `inside` (paths relative to the directory), keeping the
    /// files `install` installs.
    #[must_use]
    pub fn from_installed(meta: SkillMeta, install: &InstallSet, inside: &[TreePath]) -> Self {
        let name = meta.name().clone();
        let files = inside
            .iter()
            .filter(|file| {
                let path = TreePath::from_parts(&[SKILLS_DIR, name.as_str(), file.as_str()]);
                install.is_installed(&path)
            })
            .cloned()
            .collect();
        Self { meta, files }
    }

    /// The skill's checked frontmatter.
    #[must_use]
    pub const fn meta(&self) -> &SkillMeta {
        &self.meta
    }

    /// The skill's name, equal to its directory name.
    #[must_use]
    pub const fn name(&self) -> &ItemName {
        self.meta.name()
    }

    /// The installed files, relative to the skill's directory.
    #[must_use]
    pub fn files(&self) -> &[TreePath] {
        &self.files
    }
}

/// An agent of a loaded pack.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::Pack;
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-tokens");
/// let pack = Pack::load(&StdFileTree::new(dir)).map_err(|refusals| format!("{refusals:?}"))?;
/// let agent = pack.agents().first().ok_or("the pack ships an agent")?;
/// assert_eq!(agent.name().as_str(), "b");
/// assert_eq!(agent.file().as_str(), "agents/b.md");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct PackAgent {
    meta: AgentMeta,
    file: TreePath,
}

impl PackAgent {
    /// An agent read from `file`, a path relative to the pack root.
    #[must_use]
    pub const fn new(meta: AgentMeta, file: TreePath) -> Self {
        Self { meta, file }
    }

    /// The agent's checked frontmatter.
    #[must_use]
    pub const fn meta(&self) -> &AgentMeta {
        &self.meta
    }

    /// The agent's name, equal to its file stem.
    #[must_use]
    pub const fn name(&self) -> &ItemName {
        self.meta.name()
    }

    /// The agent's file, relative to the pack root.
    #[must_use]
    pub const fn file(&self) -> &TreePath {
        &self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::pack::testing::{DIR, layout, manifest_with};
    use std::error::Error;
    use std::path::Path;

    type TestResult = Result<(), Box<dyn Error>>;

    fn skill_meta() -> Result<SkillMeta, Box<dyn Error>> {
        let name = ItemName::new("a")?;
        let text = "---\nname: a\ndescription: d\n---\n\nbody\n";
        SkillMeta::parse(Path::new("skills/a/SKILL.md"), text, &name)
            .map_err(|d| format!("{d:?}").into())
    }

    #[test]
    fn pack_skill_lists_files_relative_to_its_directory() -> TestResult {
        let files = layout(&["pack.toml", "skills/a/SKILL.md", "skills/a/references/r.md"]);
        let manifest = manifest_with("", "")?;
        let (install, _) = InstallSet::resolve(Path::new(DIR), &files, &manifest, &[]);
        let inside = files.skills().get(&ItemName::new("a")?).ok_or("skill a")?;
        let skill = PackSkill::from_installed(skill_meta()?, &install, inside);
        let listed: Vec<&str> = skill.files().iter().map(TreePath::as_str).collect();
        assert_eq!(listed, ["SKILL.md", "references/r.md"]);
        assert_eq!(skill.name().as_str(), "a");
        Ok(())
    }

    #[test]
    fn pack_skill_leaves_out_excluded_files() -> TestResult {
        let files = layout(&[
            "pack.toml",
            "skills/a/SKILL.md",
            "skills/a/scripts/b_test.lua",
        ]);
        let manifest = manifest_with("exclude = [\"**/*_test.lua\"]", "")?;
        let (install, _) = InstallSet::resolve(Path::new(DIR), &files, &manifest, &[]);
        let inside = files.skills().get(&ItemName::new("a")?).ok_or("skill a")?;
        let skill = PackSkill::from_installed(skill_meta()?, &install, inside);
        let listed: Vec<&str> = skill.files().iter().map(TreePath::as_str).collect();
        assert_eq!(listed, ["SKILL.md"]);
        Ok(())
    }
}
