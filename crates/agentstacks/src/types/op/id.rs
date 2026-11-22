//! The id the plan payload gives each item an extension must account for.
//!
//! An extension copies an id back in `copy`, `skip` and `covers`; it never builds one. So an id
//! is built only here, from typed parts, and there is no parse from text.
//!
//! # Responsibilities
//!
//! - [`ItemId`] has one constructor per id format: `<pack>/skill/<skill>`,
//!   `<pack>/skill/<skill>/<path>`, `<pack>/agent/<agent>`, `<pack>/hook/<index>`,
//!   `<pack>/native/<tool>/<path>` and `<pack>/native-hook/<index>`.
//! - It answers which pack an id belongs to, which skill an id of a skill file is part of, and
//!   whether it names a file.
//! - Equality, ordering and hashing are by the id text, so "id order" is the byte order of the
//!   text and `core/hook/10` sorts before `core/hook/2`.
//!
//! # Non-responsibilities
//!
//! Which ids a run holds belongs to the plan payload.

use std::fmt;
use std::hash::{Hash, Hasher};

use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::tree::TreePath;

/// What an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Skill,
    SkillFile,
    Agent,
    Hook,
    NativeFile,
    NativeHook,
}

/// The id of one item in a run's plan payload.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ItemName, PackName};
/// use agentstacks::types::op::ItemId;
///
/// let core = PackName::new("core")?;
/// let skill = ItemId::skill(&core, &ItemName::new("status")?);
/// assert_eq!(skill.to_string(), "core/skill/status");
/// assert!(skill.is_skill_group());
/// assert_eq!(ItemId::hook(&core, 0).to_string(), "core/hook/0");
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone)]
pub struct ItemId {
    text: String,
    pack: PackName,
    skill: Option<ItemName>,
    kind: Kind,
}

impl ItemId {
    fn build(pack: &PackName, skill: Option<ItemName>, kind: Kind, text: String) -> Self {
        Self {
            text,
            pack: pack.clone(),
            skill,
            kind,
        }
    }

    /// The id of a whole skill: `core/skill/status`.
    #[must_use]
    pub fn skill(pack: &PackName, skill: &ItemName) -> Self {
        let text = format!("{pack}/skill/{skill}");
        Self::build(pack, Some(skill.clone()), Kind::Skill, text)
    }

    /// The id of a file of a skill, with `rel` relative to the skill directory:
    /// `core/skill/status/SKILL.md`.
    #[must_use]
    pub fn skill_file(pack: &PackName, skill: &ItemName, rel: &TreePath) -> Self {
        let text = format!("{pack}/skill/{skill}/{rel}");
        Self::build(pack, Some(skill.clone()), Kind::SkillFile, text)
    }

    /// The id of an agent: `core/agent/coder`.
    #[must_use]
    pub fn agent(pack: &PackName, agent: &ItemName) -> Self {
        Self::build(pack, None, Kind::Agent, format!("{pack}/agent/{agent}"))
    }

    /// The id of the hook at `index` in the pack's `[[hooks]]`: `core/hook/0`.
    #[must_use]
    pub fn hook(pack: &PackName, index: usize) -> Self {
        Self::build(pack, None, Kind::Hook, format!("{pack}/hook/{index}"))
    }

    /// The id of a file in `native/<tool>/`, with `rel` relative to that directory:
    /// `core/native/<tool>/x.md` (example: `core/native/claude/x.md` for the claude tool).
    #[must_use]
    pub fn native_file(pack: &PackName, tool: &ToolName, rel: &TreePath) -> Self {
        let text = format!("{pack}/native/{tool}/{rel}");
        Self::build(pack, None, Kind::NativeFile, text)
    }

    /// The id of the hook at `index` in `native/<tool>/hooks.toml`: `core/native-hook/0`.
    #[must_use]
    pub fn native_hook(pack: &PackName, index: usize) -> Self {
        let text = format!("{pack}/native-hook/{index}");
        Self::build(pack, None, Kind::NativeHook, text)
    }

    /// The pack the item belongs to.
    #[must_use]
    pub const fn pack(&self) -> &PackName {
        &self.pack
    }

    /// The id of the skill a skill file is part of; `None` for every other id.
    #[must_use]
    pub fn group(&self) -> Option<Self> {
        match (&self.kind, &self.skill) {
            (Kind::SkillFile, Some(skill)) => Some(Self::skill(&self.pack, skill)),
            _ => None,
        }
    }

    /// Whether the id names a whole skill.
    #[must_use]
    pub fn is_skill_group(&self) -> bool {
        self.kind == Kind::Skill
    }

    /// Whether the id names a file: a skill file or a native file. These are the ids `copy`
    /// takes.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self.kind, Kind::SkillFile | Kind::NativeFile)
    }

    /// The id text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl PartialEq for ItemId {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for ItemId {}

impl PartialOrd for ItemId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.text.cmp(&other.text)
    }
}

impl Hash for ItemId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn pack(text: &str) -> Result<PackName, Box<dyn std::error::Error>> {
        Ok(PackName::new(text)?)
    }

    fn item(text: &str) -> Result<ItemName, Box<dyn std::error::Error>> {
        Ok(ItemName::new(text)?)
    }

    fn rel(parts: &[&str]) -> TreePath {
        TreePath::from_parts(parts)
    }

    /// Every id format, each built by its own constructor.
    fn all() -> Result<Vec<ItemId>, Box<dyn std::error::Error>> {
        let core = pack("core")?;
        let status = item("status")?;
        let claude = ToolName::new("claude")?;
        Ok(vec![
            ItemId::skill(&core, &status),
            ItemId::skill_file(&core, &status, &rel(&["SKILL.md"])),
            ItemId::skill_file(&core, &status, &rel(&["references", "x.md"])),
            ItemId::agent(&core, &item("coder")?),
            ItemId::hook(&core, 0),
            ItemId::native_file(&core, &claude, &rel(&["x.md"])),
            ItemId::native_hook(&core, 0),
        ])
    }

    #[test]
    fn item_id_formats_follow_the_spec() -> TestResult {
        let shown: Vec<String> = all()?.iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            [
                "core/skill/status",
                "core/skill/status/SKILL.md",
                "core/skill/status/references/x.md",
                "core/agent/coder",
                "core/hook/0",
                "core/native/claude/x.md",
                "core/native-hook/0",
            ]
        );
        assert_eq!(all()?[1].as_str(), "core/skill/status/SKILL.md");
        Ok(())
    }

    #[test]
    fn item_id_knows_its_pack_and_group() -> TestResult {
        let ids = all()?;
        for id in &ids {
            assert_eq!(id.pack().as_str(), "core", "{id}");
        }
        let group = ids[1].group().map(|id| id.to_string());
        assert_eq!(group.as_deref(), Some("core/skill/status"));
        let nested = ids[2].group().map(|id| id.to_string());
        assert_eq!(nested.as_deref(), Some("core/skill/status"));
        for id in [&ids[0], &ids[3], &ids[4], &ids[5], &ids[6]] {
            assert_eq!(id.group(), None, "{id}");
        }
        Ok(())
    }

    #[test]
    fn item_id_knows_which_ids_are_files() -> TestResult {
        let ids = all()?;
        let files: Vec<bool> = ids.iter().map(ItemId::is_file).collect();
        assert_eq!(files, [false, true, true, false, false, true, false]);
        let groups: Vec<bool> = ids.iter().map(ItemId::is_skill_group).collect();
        assert_eq!(groups, [true, false, false, false, false, false, false]);
        Ok(())
    }

    #[test]
    fn item_id_orders_by_text() -> TestResult {
        let core = pack("core")?;
        let mut ids = [
            ItemId::skill(&core, &item("status")?),
            ItemId::agent(&core, &item("coder")?),
            ItemId::hook(&core, 0),
        ];
        ids.sort();
        let shown: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            ["core/agent/coder", "core/hook/0", "core/skill/status"]
        );
        Ok(())
    }

    #[test]
    fn item_id_orders_hook_indexes_as_text() -> TestResult {
        let core = pack("core")?;
        let mut ids = [ItemId::hook(&core, 2), ItemId::hook(&core, 10)];
        ids.sort();
        let shown: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(shown, ["core/hook/10", "core/hook/2"]);
        Ok(())
    }
}
