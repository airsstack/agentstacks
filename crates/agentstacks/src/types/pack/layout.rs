//! Where each file of a pack belongs.
//!
//! It is separate from the loader so the structure rules need only a walk result and never a file's
//! contents: they run before any file is read, and every later rule asks the layout where a file
//! belongs.
//!
//! # Responsibilities
//!
//! - [`PackLayout::classify`] sorts every walked file into exactly one group: `pack.toml`, a
//!   skill, an agent, a tool's `native/<tool>/` directory, or the root files installed under the
//!   pack's install root.
//! - It refuses what breaks the layout rules, each at a path under the pack directory with no
//!   line: no `pack.toml`; files under the reserved `commands/` and `rules/`, once per
//!   directory; a file directly in `skills/` or `native/`; a skill, agent or tool name that is not
//!   valid; a skill directory without `SKILL.md`; and, in `agents/`, a file that is not
//!   `<name>.md` or a subdirectory (once, at its own path).
//! - It remembers which entries the walk refused, so a later rule can tell a file that is absent
//!   from one the author already heard about: a refused `pack.toml` is not "no `pack.toml`", a
//!   refused `SKILL.md` is not "missing `SKILL.md`", and [`PackLayout::lacks_skill`],
//!   [`PackLayout::lacks_agent`] and [`PackLayout::is_refused`] answer for a refused entry as
//!   "not known to be missing".
//! - A skill and an agent may share a name.
//!
//! # Non-responsibilities
//!
//! Reading file contents, applying `exclude` and `render`, and every rule that needs the manifest
//! belong to the pack loader and the checks it runs. A directory that is empty never reaches the
//! layout: the walker lists files only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::types::name::InvalidName;
use crate::types::name::ItemName;
use crate::types::name::ToolName;
use crate::types::pack::error::PackError;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tree::TreePath;
use crate::types::tree::Walked;

/// The file that makes a directory a pack.
pub(in crate::types) const MANIFEST_FILE: &str = "pack.toml";

/// The file every skill directory must hold.
pub(super) const SKILL_FILE: &str = "SKILL.md";

/// The directory every skill lives under.
pub(super) const SKILLS_DIR: &str = "skills";

/// The directory every agent file lives under.
pub(super) const AGENTS_DIR: &str = "agents";

/// The directory every tool's content lives under.
pub(super) const NATIVE_DIR: &str = "native";

/// The file that declares a tool's native hooks.
pub(super) const NATIVE_HOOKS_FILE: &str = "hooks.toml";

/// The pack-relative path of the hooks file of `tool`: `native/<tool>/hooks.toml`.
pub(in crate::types) fn native_hooks_file(tool: &ToolName) -> TreePath {
    TreePath::from_parts(&[NATIVE_DIR, tool.as_str(), NATIVE_HOOKS_FILE])
}

/// The directories the format reserves.
const RESERVED_DIRECTORIES: [&str; 2] = ["commands", "rules"];

/// The files of one `native/<tool>/` directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct NativeLayout {
    files: Vec<TreePath>,
    hooks: Option<TreePath>,
}

impl NativeLayout {
    /// The files to install, relative to the tool's directory, `hooks.toml` not among them.
    #[must_use]
    pub(super) fn files(&self) -> &[TreePath] {
        &self.files
    }

    /// The tool's `hooks.toml`, relative to the pack root, when the directory holds one.
    #[must_use]
    pub(super) const fn hooks(&self) -> Option<&TreePath> {
        self.hooks.as_ref()
    }
}

/// A pack's files sorted by the group they belong to.
#[derive(Debug, Clone, Default)]
pub(super) struct PackLayout {
    all: Vec<TreePath>,
    skills: BTreeMap<ItemName, Vec<TreePath>>,
    agents: BTreeMap<ItemName, TreePath>,
    native: BTreeMap<ToolName, NativeLayout>,
    root: Vec<TreePath>,
    refused: Walked,
    unseen_items: bool,
}

impl PackLayout {
    /// Sorts the files of the `walked` pack at `dir` and returns the layout with every refusal of
    /// the layout rules.
    ///
    /// # Errors
    ///
    /// The one refusal that `dir` is not a pack, when it holds no `pack.toml` and the walk did not
    /// refuse `pack.toml` or a directory above it: a pack directory the walk could not list is not
    /// reported as holding no `pack.toml`.
    pub(super) fn classify(dir: &Path, walked: &Walked) -> Result<(Self, Diagnostics), Diagnostic> {
        let files = walked.files();
        let has_manifest = files.iter().any(|file| file.as_str() == MANIFEST_FILE)
            || walked.covers(Path::new(MANIFEST_FILE));
        if !has_manifest {
            return Err(Diagnostic::new(Location::file(dir), PackError::NotAPack));
        }
        let mut sorter = Sorter::new(dir, walked);
        for file in files {
            sorter.sort(file);
        }
        Ok(sorter.finish(files))
    }

    /// Every walked file, relative to the pack root, sorted by path bytes.
    #[must_use]
    pub(super) fn files(&self) -> &[TreePath] {
        &self.all
    }

    /// The skills with their files, relative to each skill's directory, `SKILL.md` included.
    #[must_use]
    pub(super) const fn skills(&self) -> &BTreeMap<ItemName, Vec<TreePath>> {
        &self.skills
    }

    /// The agents with their file, relative to the pack root.
    #[must_use]
    pub(super) const fn agents(&self) -> &BTreeMap<ItemName, TreePath> {
        &self.agents
    }

    /// The `native/<tool>/` directories.
    #[must_use]
    pub(super) const fn native(&self) -> &BTreeMap<ToolName, NativeLayout> {
        &self.native
    }

    /// The files installed under the pack's install root, relative to the pack root.
    #[must_use]
    pub(super) fn root_files(&self) -> &[TreePath] {
        &self.root
    }

    /// Whether the pack has a skill directory called `name`.
    #[must_use]
    pub(super) fn has_skill(&self, name: &ItemName) -> bool {
        self.skills.contains_key(name)
    }

    /// Whether the pack has an agent called `name`.
    #[must_use]
    pub(super) fn has_agent(&self, name: &ItemName) -> bool {
        self.agents.contains_key(name)
    }

    /// Whether the pack has no skill called `name` and the walk did not refuse the directory it
    /// would be in.
    #[must_use]
    pub(super) fn lacks_skill(&self, name: &ItemName) -> bool {
        !self.has_skill(name)
            && !self.is_refused(&TreePath::from_parts(&[SKILLS_DIR, name.as_str()]))
    }

    /// Whether the pack has no agent called `name` and the walk did not refuse the file it would
    /// be in.
    #[must_use]
    pub(super) fn lacks_agent(&self, name: &ItemName) -> bool {
        let file = format!("{name}.md");
        !self.has_agent(name) && !self.is_refused(&TreePath::from_parts(&[AGENTS_DIR, &file]))
    }

    /// Whether the walk refused `path` or a directory above it.
    #[must_use]
    pub(super) fn is_refused(&self, path: &TreePath) -> bool {
        self.refused.covers(Path::new(path.as_str()))
    }

    /// Whether a skill, an agent or a tool directory may exist that the layout does not list,
    /// because the walk or a name rule refused it. A rule that asks "does the pack hold at least
    /// one?" cannot answer while this is true.
    #[must_use]
    pub(super) const fn may_hold_unseen_items(&self) -> bool {
        self.unseen_items
    }
}

/// Builds a [`PackLayout`] one file at a time, remembering which directories were refused already.
struct Sorter<'a> {
    dir: &'a Path,
    layout: PackLayout,
    diagnostics: Diagnostics,
    refused: BTreeSet<TreePath>,
}

impl<'a> Sorter<'a> {
    fn new(dir: &'a Path, walked: &Walked) -> Self {
        let layout = PackLayout {
            unseen_items: walked.refused().iter().any(|path| holds_items(path)),
            refused: Walked::new(Vec::new(), Vec::new(), walked.refused().to_vec()),
            ..PackLayout::default()
        };
        Self {
            dir,
            layout,
            diagnostics: Diagnostics::new(),
            refused: BTreeSet::new(),
        }
    }

    fn sort(&mut self, file: &TreePath) {
        let segments: Vec<&str> = file.as_str().split('/').collect();
        match segments.as_slice() {
            [MANIFEST_FILE] => {}
            [reserved, ..] if RESERVED_DIRECTORIES.contains(reserved) => {
                self.refuse_once(&TreePath::from_parts(&[reserved]), PackError::Reserved);
            }
            [SKILLS_DIR, _] => self.refuse_once(file, PackError::OnlySkillDirectories),
            [SKILLS_DIR, name, rest @ ..] => self.sort_skill_file(name, rest),
            [AGENTS_DIR, entry] => self.sort_agent(entry, file),
            [AGENTS_DIR, directory, ..] => {
                self.refuse_once(
                    &TreePath::from_parts(&[AGENTS_DIR, directory]),
                    PackError::OnlyAgentFiles,
                );
            }
            [NATIVE_DIR, _] => self.refuse_once(file, PackError::OnlyToolDirectories),
            [NATIVE_DIR, tool, rest @ ..] => self.sort_native_file(tool, rest, file),
            _ => self.layout.root.push(file.clone()),
        }
    }

    fn sort_skill_file(&mut self, name: &str, rest: &[&str]) {
        let key = TreePath::from_parts(&[SKILLS_DIR, name]);
        let name = match ItemName::new(name) {
            Ok(name) => name,
            Err(error) => {
                self.refuse_name(&key, error);
                return;
            }
        };
        if !rest.is_empty() {
            let inside = TreePath::from_parts(rest);
            self.layout.skills.entry(name).or_default().push(inside);
        }
    }

    fn sort_agent(&mut self, entry: &str, file: &TreePath) {
        let Some(stem) = entry.strip_suffix(".md") else {
            self.refuse_once(file, PackError::OnlyAgentFiles);
            return;
        };
        match ItemName::new(stem) {
            Ok(name) => {
                self.layout.agents.insert(name, file.clone());
            }
            Err(error) => self.refuse_name(file, error),
        }
    }

    fn sort_native_file(&mut self, tool: &str, rest: &[&str], file: &TreePath) {
        let key = TreePath::from_parts(&[NATIVE_DIR, tool]);
        let tool = match ToolName::new(tool) {
            Ok(tool) => tool,
            Err(error) => {
                self.refuse_name(&key, error);
                return;
            }
        };
        let entry = self.layout.native.entry(tool).or_default();
        if rest == [NATIVE_HOOKS_FILE] {
            entry.hooks = Some(file.clone());
        } else if !rest.is_empty() {
            entry.files.push(TreePath::from_parts(rest));
        }
    }

    /// Records the refusal of an invalid skill, agent or tool name at `relative`, once.
    fn refuse_name(&mut self, relative: &TreePath, error: InvalidName) {
        self.layout.unseen_items = true;
        self.refuse_once(relative, PackError::Name(error));
    }

    /// Records a refusal at `<dir>/<relative>`, once per `relative`.
    fn refuse_once(&mut self, relative: &TreePath, error: PackError) {
        if self.refused.insert(relative.clone()) {
            let location = Location::file(self.dir.join(relative.as_str()));
            self.diagnostics.push(Diagnostic::new(location, error));
        }
    }

    fn finish(mut self, files: &[TreePath]) -> (PackLayout, Diagnostics) {
        let without_skill_md: Vec<TreePath> = self
            .layout
            .skills
            .iter()
            .filter(|(_, inside)| !inside.iter().any(|file| file.as_str() == SKILL_FILE))
            .map(|(name, _)| TreePath::from_parts(&[SKILLS_DIR, name.as_str()]))
            .filter(|dir| !self.layout.is_refused(&skill_file_of(dir)))
            .collect();
        for relative in without_skill_md {
            self.refuse_once(&relative, PackError::MissingSkillMd);
        }
        self.layout.all = files.to_vec();
        (self.layout, self.diagnostics)
    }
}

/// The `SKILL.md` of the skill directory `dir`.
fn skill_file_of(dir: &TreePath) -> TreePath {
    TreePath::from_parts(&[dir.as_str(), SKILL_FILE])
}

/// Whether the refused entry `path` is, or lies below, a place a skill, an agent or a tool
/// directory would be. The walked directory itself (the empty path) is such a place.
fn holds_items(path: &Path) -> bool {
    path.components().next().is_none_or(|top| {
        [SKILLS_DIR, AGENTS_DIR, NATIVE_DIR]
            .iter()
            .any(|dir| top.as_os_str() == *dir)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::pack::testing::{DIR, paths, refused_walk};
    use std::error::Error;
    use std::path::PathBuf;

    type TestResult = Result<(), Box<dyn Error>>;

    fn classify(texts: &[&str]) -> Result<(PackLayout, Vec<String>), String> {
        classify_refused(texts, &[])
    }

    fn classify_refused(
        texts: &[&str],
        refused: &[&str],
    ) -> Result<(PackLayout, Vec<String>), String> {
        match PackLayout::classify(Path::new(DIR), &refused_walk(texts, refused)) {
            Ok((layout, diagnostics)) => Ok((
                layout,
                diagnostics
                    .into_sorted()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            )),
            Err(not_a_pack) => Err(not_a_pack.to_string()),
        }
    }

    fn layout_of(texts: &[&str]) -> Result<(PackLayout, Vec<String>), Box<dyn Error>> {
        Ok(classify(texts)?)
    }

    fn strs(files: &[TreePath]) -> Vec<&str> {
        files.iter().map(TreePath::as_str).collect()
    }

    fn name(text: &str) -> Result<ItemName, Box<dyn Error>> {
        Ok(ItemName::new(text)?)
    }

    #[test]
    fn classify_sorts_files_into_skills_agents_native_and_root() -> TestResult {
        let (layout, shown) = layout_of(&[
            "README.md",
            "agents/b.md",
            "native/claude/hooks.toml",
            "native/claude/x.json",
            "pack.toml",
            "scripts/s.lua",
            "skills/a/SKILL.md",
            "skills/a/references/r.md",
        ])?;
        assert!(shown.is_empty(), "{shown:?}");
        let skill = layout.skills().get(&name("a")?).ok_or("skill a")?;
        assert_eq!(strs(skill), ["SKILL.md", "references/r.md"]);
        let agent = layout.agents().get(&name("b")?).ok_or("agent b")?;
        assert_eq!(agent.as_str(), "agents/b.md");
        let native = layout
            .native()
            .get(&ToolName::new("claude")?)
            .ok_or("native claude")?;
        assert_eq!(strs(native.files()), ["x.json"]);
        assert_eq!(
            native.hooks().map(TreePath::as_str),
            Some("native/claude/hooks.toml")
        );
        assert_eq!(strs(layout.root_files()), ["README.md", "scripts/s.lua"]);
        assert!(!layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_refuses_a_directory_without_pack_toml() {
        let refusal = classify(&["skills/a/SKILL.md"]).err();
        assert_eq!(refusal, Some(format!("{DIR}: not a pack (no pack.toml)")));
    }

    #[test]
    fn classify_does_not_say_not_a_pack_when_the_walk_refused_pack_toml() -> TestResult {
        let (layout, shown) = classify_refused(&["skills/a/SKILL.md"], &["pack.toml"])?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(strs(layout.files()), ["skills/a/SKILL.md"]);
        Ok(())
    }

    #[test]
    fn classify_does_not_say_not_a_pack_when_the_pack_directory_could_not_be_listed() -> TestResult
    {
        let (layout, shown) = classify_refused(&[], &[""])?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(layout.files().is_empty());
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_says_not_a_pack_when_the_walk_refused_only_unrelated_entries() {
        let refusal = classify_refused(&["skills/a/SKILL.md"], &["scripts"]).err();
        assert_eq!(refusal, Some(format!("{DIR}: not a pack (no pack.toml)")));
    }

    #[test]
    fn classify_may_hold_unseen_items_under_any_refused_item_directory() -> TestResult {
        for refused in ["skills", "skills/a", "agents/b.md", "native/claude/x"] {
            let (layout, _) = classify_refused(&["pack.toml"], &[refused])?;
            assert!(layout.may_hold_unseen_items(), "{refused}");
        }
        let (layout, _) = classify_refused(&["pack.toml"], &["scripts/x", "README.md"])?;
        assert!(!layout.may_hold_unseen_items());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn classify_sees_a_refused_non_utf8_entry_under_an_item_directory() -> TestResult {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad = PathBuf::from("skills").join(OsStr::from_bytes(&[0x66, 0xFF]));
        let walk = Walked::new(paths(&["pack.toml"]), Vec::new(), vec![bad]);
        let (layout, diagnostics) =
            PackLayout::classify(Path::new(DIR), &walk).map_err(|refusal| refusal.to_string())?;
        assert!(diagnostics.is_empty());
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_reports_each_reserved_directory_once() -> TestResult {
        let (_, shown) = layout_of(&[
            "pack.toml",
            "commands/a.md",
            "commands/b/c.md",
            "rules/x.md",
        ])?;
        assert_eq!(
            shown,
            [
                format!("{DIR}/commands: reserved, not supported in api 0"),
                format!("{DIR}/rules: reserved, not supported in api 0"),
            ]
        );
        Ok(())
    }

    #[test]
    fn classify_refuses_a_skill_directory_without_skill_md() -> TestResult {
        let (_, shown) = layout_of(&["pack.toml", "skills/a/references/r.md"])?;
        assert_eq!(shown, [format!("{DIR}/skills/a: missing SKILL.md")]);
        Ok(())
    }

    #[test]
    fn classify_does_not_call_a_refused_skill_md_missing() -> TestResult {
        let (layout, shown) = classify_refused(
            &["pack.toml", "skills/a/references/r.md"],
            &["skills/a/SKILL.md"],
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_refuses_files_directly_in_skills() -> TestResult {
        let (_, shown) = layout_of(&["pack.toml", "skills/README.md"])?;
        assert_eq!(
            shown,
            [format!(
                "{DIR}/skills/README.md: only skill directories belong in skills/"
            )]
        );
        Ok(())
    }

    #[test]
    fn classify_refuses_invalid_skill_directory_names() -> TestResult {
        let (layout, shown) = layout_of(&["pack.toml", "skills/My_Skill/notes.md"])?;
        assert_eq!(
            shown,
            [format!(
                "{DIR}/skills/My_Skill: \"My_Skill\" is not a valid name"
            )]
        );
        assert!(layout.skills().is_empty());
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_refuses_non_md_files_and_subdirectories_in_agents() -> TestResult {
        let (_, shown) = layout_of(&[
            "pack.toml",
            "agents/notes.txt",
            "agents/x/y.md",
            "agents/x/z.md",
        ])?;
        assert_eq!(
            shown,
            [
                format!("{DIR}/agents/notes.txt: agents/ holds only <name>.md files"),
                format!("{DIR}/agents/x: agents/ holds only <name>.md files"),
            ]
        );
        Ok(())
    }

    #[test]
    fn classify_refuses_invalid_agent_stems() -> TestResult {
        let (layout, shown) = layout_of(&["pack.toml", "agents/Coder.md"])?;
        assert_eq!(
            shown,
            [format!(
                "{DIR}/agents/Coder.md: \"Coder\" is not a valid name"
            )]
        );
        assert!(layout.agents().is_empty());
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_refuses_files_directly_in_native() -> TestResult {
        let (_, shown) = layout_of(&["pack.toml", "native/README.md"])?;
        assert_eq!(
            shown,
            [format!(
                "{DIR}/native/README.md: only tool directories belong in native/"
            )]
        );
        Ok(())
    }

    #[test]
    fn classify_refuses_invalid_native_tool_names() -> TestResult {
        let (layout, shown) = layout_of(&["pack.toml", "native/Claude/x.json"])?;
        assert_eq!(
            shown,
            [format!(
                "{DIR}/native/Claude: \"Claude\" is not a valid name"
            )]
        );
        assert!(layout.native().is_empty());
        assert!(layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn classify_keeps_a_skill_and_an_agent_with_the_same_name() -> TestResult {
        let (layout, shown) = layout_of(&["pack.toml", "skills/x/SKILL.md", "agents/x.md"])?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(layout.has_skill(&name("x")?));
        assert!(layout.has_agent(&name("x")?));
        Ok(())
    }

    #[test]
    fn classify_lists_every_walked_file() -> TestResult {
        let (layout, _) = layout_of(&["README.md", "native/claude/hooks.toml", "pack.toml"])?;
        assert_eq!(
            strs(layout.files()),
            ["README.md", "native/claude/hooks.toml", "pack.toml"]
        );
        Ok(())
    }

    #[test]
    fn a_refused_entry_is_not_known_to_be_missing() -> TestResult {
        let (layout, _) = classify_refused(
            &["pack.toml"],
            &["skills/a", "agents/b.md", "hooks", "link"],
        )?;
        assert!(!layout.lacks_skill(&name("a")?));
        assert!(!layout.lacks_agent(&name("b")?));
        assert!(layout.lacks_skill(&name("c")?));
        assert!(layout.lacks_agent(&name("c")?));
        let refused = |text: &str| layout.is_refused(&TreePath::from_parts(&[text]));
        assert!(refused("hooks/x.lua"));
        assert!(refused("link"));
        assert!(!refused("hooks2/x.lua"));
        assert!(!refused("linked"));
        Ok(())
    }

    #[test]
    fn a_walk_refusal_in_an_item_directory_may_hide_an_item() -> TestResult {
        for refused in ["skills/a", "agents/b.md", "native", "skills"] {
            let (layout, _) = classify_refused(&["pack.toml"], &[refused])?;
            assert!(layout.may_hold_unseen_items(), "{refused}");
        }
        let (layout, _) = classify_refused(&["pack.toml"], &["scripts/link", "link"])?;
        assert!(!layout.may_hold_unseen_items());
        Ok(())
    }

    #[test]
    fn native_hooks_file_is_the_hooks_toml_of_the_tool_directory() -> TestResult {
        let claude = ToolName::new("claude")?;
        assert_eq!(
            native_hooks_file(&claude).as_str(),
            "native/claude/hooks.toml"
        );
        Ok(())
    }
}
