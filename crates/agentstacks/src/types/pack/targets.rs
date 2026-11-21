//! What a pack's own hooks point at inside the pack.
//!
//! It is separate from the install set because the hooks are read first and the install set needs
//! the files they name, to keep `exclude` from removing them.
//!
//! # Responsibilities
//!
//! - [`HookTargets::collect`] gathers, from `pack.toml` and every `native/<tool>/hooks.toml`, each
//!   `run` that is the pack's own (pack-relative, `{{root}}/…`, or `{{root <this pack>}}/…`) and
//!   each `agents` entry of a `pack.toml` hook.
//! - [`HookTargets::run_paths`] lists the pack-relative files the own runs resolve to, so
//!   `exclude` can be kept from removing them.
//! - [`HookTargets::check`] refuses an own `run` that is not one of the files installed under the
//!   pack's install root (so not `pack.toml`, and nothing under `skills/`, `agents/` or
//!   `native/`), and an `agents` entry that is not an agent of the pack. A `run` of another pack
//!   is not looked at, and neither is a target the walk refused: the author already heard about
//!   it.
//!
//! # Non-responsibilities
//!
//! Whether a `run` is excluded belongs to [`InstallSet`](crate::types::InstallSet): an excluded
//! file exists, so it is refused there and not here. The form of a `run` was checked when the hook
//! was read.

use std::path::Path;

use crate::types::hooks::HookAction;
use crate::types::hooks::NativeHooks;
use crate::types::manifest::PackManifest;
use crate::types::name::ItemName;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::error::PackError;
use crate::types::pack::item::kind::ItemKind;
use crate::types::pack::layout::{MANIFEST_FILE, NATIVE_DIR, NATIVE_HOOKS_FILE, PackLayout};
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tokens::names_own_pack;
use crate::types::tree::TreePath;

/// A `run` of a hook that names a file of the hook's own pack.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnRun {
    /// The pack-relative file the hook is declared in.
    file: TreePath,
    index: usize,
    written: String,
    path: TreePath,
}

/// An `agents` entry of a `pack.toml` hook.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentRef {
    /// The index of the hook in `pack.toml`.
    index: usize,
    /// The index of the entry in the hook's `agents` list.
    position: usize,
    name: ItemName,
}

/// The own-pack runs and agents the hooks of one pack name.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{FileTree, HookTargets, PackLayout, PackManifest, TreePath, walk};
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/hook-run-missing");
/// let tree = StdFileTree::new(dir);
/// let (walked, _) = walk(&tree);
/// let (layout, _) = PackLayout::classify(tree.root(), &walked).map_err(|d| d.to_string())?;
/// let manifest = PackManifest::parse(Path::new("pack.toml"), &tree.read_text(Path::new("pack.toml"))?)
///     .map_err(|refusals| format!("{refusals:?}"))?;
///
/// let targets = HookTargets::collect(manifest.name(), &manifest, &[]);
/// let paths = targets.run_paths();
/// let shown: Vec<&str> = paths.iter().map(TreePath::as_str).collect();
/// assert_eq!(shown, ["hooks/a.lua", "hooks/missing.lua"]);
///
/// // Only the script the pack does not ship is refused.
/// let refusals: Vec<String> = targets
///     .check(Path::new("packs/core"), &layout)
///     .into_sorted()
///     .iter()
///     .map(ToString::to_string)
///     .collect();
/// assert_eq!(
///     refusals,
///     ["packs/core/pack.toml: hooks[1].run: \"hooks/missing.lua\" does not exist"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct HookTargets {
    pack: PackName,
    runs: Vec<OwnRun>,
    agents: Vec<AgentRef>,
}

impl HookTargets {
    /// Gathers the targets of the hooks of the pack `pack`: its `manifest` hooks and the hooks of
    /// each tool in `native`.
    #[must_use]
    pub fn collect(
        pack: &PackName,
        manifest: &PackManifest,
        native: &[(&ToolName, &NativeHooks)],
    ) -> Self {
        let mut runs = Vec::new();
        let mut agents = Vec::new();
        for (index, hook) in manifest.hooks().iter().enumerate() {
            let file = TreePath::from_parts(&[MANIFEST_FILE]);
            collect_run(&mut runs, pack, &file, index, hook.action());
            for (position, name) in hook.agents().unwrap_or_default().iter().enumerate() {
                agents.push(AgentRef {
                    index,
                    position,
                    name: name.clone(),
                });
            }
        }
        for (tool, hooks) in native {
            let file = TreePath::from_parts(&[NATIVE_DIR, tool.as_str(), NATIVE_HOOKS_FILE]);
            for (index, hook) in hooks.hooks().iter().enumerate() {
                collect_run(&mut runs, pack, &file, index, hook.action());
            }
        }
        Self {
            pack: pack.clone(),
            runs,
            agents,
        }
    }

    /// The pack-relative files the own runs resolve to, in declaration order.
    #[must_use]
    pub fn run_paths(&self) -> Vec<TreePath> {
        self.runs.iter().map(|run| run.path.clone()).collect()
    }

    /// Refuses each own `run` that is not a root file of `layout` and each `agents` entry that is
    /// not an agent of `layout`, located under the pack directory `dir`. A refusal about an
    /// `agents` entry is located at that entry, `hooks[n].agents[i]`. A target the walk refused is
    /// not refused again.
    #[must_use]
    pub fn check(&self, dir: &Path, layout: &PackLayout) -> Diagnostics {
        let mut refusals = Diagnostics::new();
        for run in &self.runs {
            let exists = layout.root_files().contains(&run.path);
            if !exists && !layout.is_refused(&run.path) {
                refusals.push(Diagnostic::new(
                    Location::at_key(
                        dir.join(run.file.as_str()),
                        format!("hooks[{}].run", run.index),
                    ),
                    PackError::DoesNotExist {
                        path: run.written.clone(),
                    },
                ));
            }
        }
        for agent in &self.agents {
            if layout.lacks_agent(&agent.name) {
                refusals.push(Diagnostic::new(
                    Location::at_key(
                        dir.join(MANIFEST_FILE),
                        format!("hooks[{}].agents[{}]", agent.index, agent.position),
                    ),
                    PackError::NoSuchItem {
                        pack: self.pack.clone(),
                        kind: ItemKind::Agent,
                        item: agent.name.to_string(),
                    },
                ));
            }
        }
        refusals
    }
}

/// Records the run of `action` when it is a lua hook whose script is in `pack` itself.
fn collect_run(
    runs: &mut Vec<OwnRun>,
    pack: &PackName,
    file: &TreePath,
    index: usize,
    action: &HookAction,
) {
    let HookAction::Lua(lua) = action else {
        return;
    };
    let run = lua.run();
    if names_own_pack(run.pack(), pack) {
        runs.push(OwnRun {
            file: file.clone(),
            index,
            written: run.written().to_owned(),
            path: TreePath::from_parts(&[run.path().as_str()]),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::pack::testing::{DIR, layout, manifest, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const REQUIRES: &str = "[requires]\nairsl = \">=0.1.4\"\n";

    fn lua_hook(run: &str) -> String {
        format!("[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"{run}\"\n")
    }

    fn core() -> Result<PackName, Box<dyn Error>> {
        Ok(PackName::new("core")?)
    }

    fn targets_of(hooks: &[String]) -> Result<HookTargets, Box<dyn Error>> {
        let manifest = manifest(&format!("{REQUIRES}{}", hooks.join("")))?;
        Ok(HookTargets::collect(&core()?, &manifest, &[]))
    }

    fn run_paths(targets: &HookTargets) -> Vec<String> {
        targets
            .run_paths()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn own_runs_resolve_in_all_three_forms() -> TestResult {
        let targets = targets_of(&[
            lua_hook("hooks/a.lua"),
            lua_hook("{{root}}/hooks/a.lua"),
            lua_hook("{{root core}}/hooks/a.lua"),
        ])?;
        assert_eq!(
            run_paths(&targets),
            ["hooks/a.lua", "hooks/a.lua", "hooks/a.lua"]
        );
        let found = targets.check(Path::new(DIR), &layout(&["hooks/a.lua", "pack.toml"]));
        assert!(shown(found).is_empty());
        Ok(())
    }

    #[test]
    fn other_pack_runs_are_left_to_the_set_check() -> TestResult {
        let targets = targets_of(&[lua_hook("{{root guideline}}/hooks/x.lua")])?;
        assert!(run_paths(&targets).is_empty());
        assert!(
            targets
                .check(Path::new(DIR), &layout(&["pack.toml"]))
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn missing_own_run_is_refused() -> TestResult {
        let files = layout(&["hooks/a.lua", "pack.toml"]);
        let targets = targets_of(&[lua_hook("hooks/a.lua"), lua_hook("hooks/missing.lua")])?;
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [format!(
                "{DIR}/pack.toml: hooks[1].run: \"hooks/missing.lua\" does not exist"
            )]
        );
        let targets = targets_of(&[lua_hook("{{root}}/hooks/missing.lua")])?;
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [format!(
                "{DIR}/pack.toml: hooks[0].run: \"{{{{root}}}}/hooks/missing.lua\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn a_run_or_agent_the_walk_refused_is_not_refused_again() -> TestResult {
        use crate::types::pack::testing::refused_walk;

        let walk = refused_walk(&["pack.toml"], &["hooks/a.lua", "agents/coder.md"]);
        let (files, _) =
            PackLayout::classify(Path::new(DIR), &walk).map_err(|refusal| refusal.to_string())?;
        let stop = "[[hooks]]\nkind = \"message\"\nevent = \"agent.stop\"\ntext = \"t\"\nagents = [\"coder\", \"codr\"]\n";
        let targets = targets_of(&[
            lua_hook("hooks/a.lua"),
            lua_hook("hooks/b.lua"),
            stop.to_owned(),
        ])?;
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [
                format!("{DIR}/pack.toml: hooks[1].run: \"hooks/b.lua\" does not exist"),
                format!("{DIR}/pack.toml: hooks[2].agents[1]: pack \"core\" has no agent \"codr\""),
            ]
        );
        Ok(())
    }

    #[test]
    fn run_naming_a_skill_file_does_not_exist_under_root() -> TestResult {
        let files = layout(&["pack.toml", "skills/a/SKILL.md", "skills/a/x.lua"]);
        let targets = targets_of(&[lua_hook("skills/a/x.lua")])?;
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [format!(
                "{DIR}/pack.toml: hooks[0].run: \"skills/a/x.lua\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn run_naming_a_native_file_does_not_exist_under_root() -> TestResult {
        let files = layout(&["native/claude/x.lua", "pack.toml"]);
        let targets = targets_of(&[lua_hook("native/claude/x.lua")])?;
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [format!(
                "{DIR}/pack.toml: hooks[0].run: \"native/claude/x.lua\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn native_hook_runs_are_checked_as_own_pack_runs() -> TestResult {
        let pack_manifest = manifest(REQUIRES)?;
        let text =
            "[[hooks]]\nkind = \"lua\"\nevent = \"PreToolUse\"\nrun = \"hooks/missing.lua\"\n";
        let file = Path::new(DIR).join("native/claude/hooks.toml");
        let native = NativeHooks::parse(&file, text, pack_manifest.requires())
            .map_err(|d| format!("{:?}", d.into_sorted()))?;
        let tool = ToolName::new("claude")?;
        let targets = HookTargets::collect(&core()?, &pack_manifest, &[(&tool, &native)]);
        assert_eq!(run_paths(&targets), ["hooks/missing.lua"]);
        assert_eq!(
            shown(targets.check(Path::new(DIR), &layout(&["pack.toml"]))),
            [format!(
                "{DIR}/native/claude/hooks.toml: hooks[0].run: \"hooks/missing.lua\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn hook_agents_must_name_own_agents() -> TestResult {
        let message = "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"t\"\n";
        let stop = "[[hooks]]\nkind = \"message\"\nevent = \"agent.stop\"\ntext = \"t\"\nagents = [\"coder\", \"task-briefer\"]\n";
        let targets = targets_of(&[
            message.to_owned(),
            message.to_owned(),
            message.to_owned(),
            stop.to_owned(),
        ])?;
        let files = layout(&["agents/coder.md", "pack.toml"]);
        assert_eq!(
            shown(targets.check(Path::new(DIR), &files)),
            [format!(
                "{DIR}/pack.toml: hooks[3].agents[1]: pack \"core\" has no agent \"task-briefer\""
            )]
        );
        Ok(())
    }
}
