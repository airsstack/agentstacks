//! Reading a pack directory into a checked [`Pack`].
//!
//! It is the one place that reads a pack's files and runs the rules in order, so every other file
//! in this group stays a rule over data already in hand.
//!
//! # Responsibilities
//!
//! - [`Pack::load`] walks the directory through the [`FileTree`] port, then reads `pack.toml` and
//!   checks first whether it has a `requires.agentstacks` that excludes this agentstacks. If it
//!   does, that is the one refusal returned and nothing after it runs, because a pack written for
//!   a later agentstacks may use things this one does not know. Otherwise (the walk did not list
//!   `pack.toml`, it is not UTF-8 or not TOML, or its range is absent or does not parse) the load
//!   goes on: it classifies the files, parses the text of `pack.toml` it already read, reads every
//!   `SKILL.md` and agent file, each `native/<tool>/hooks.toml`, and every rendered file, and runs
//!   each rule over them. It returns the [`Pack`] when the run found no refusal, else every
//!   refusal of the run: sorted by file, then the refusals without a line in the order found, then
//!   by line. A range that does not parse is one of those refusals.
//! - Past that first check, no refusal stops the run early except that a directory without a
//!   readable `pack.toml` is not a pack. A refusal of the walk (a symlink, a special file, a path
//!   that is not UTF-8, an I/O failure) is reported together with every other refusal, and the
//!   files the walk could list are still read and checked. A directory without `pack.toml` ends the
//!   run with the one refusal that it is not a pack, besides the walk's own refusals; a `pack.toml`
//!   the walk refused is not reported as absent.
//! - A refusal never causes a second one about the same thing. A skill, agent or tool directory
//!   the walk or a name rule refused, and a `native/<tool>/hooks.toml` that failed to read, may
//!   have been the pack's only item, so "nothing to install" is not reported then. A file the walk
//!   refused is not reported as missing from `render`, a hook `run`, a `SKILL.md` or a token.
//! - When `pack.toml` is refused, the layout rules, the frontmatter and the bodies of `SKILL.md`
//!   and agent files (which `exclude` can never remove) and the rule that neutral content must not
//!   reference `native/` are still checked. Everything that needs the manifest (native hooks,
//!   `exclude`, `render`, hook targets, the airsl rule, nothing to install, the other bodies, the
//!   pack-local token rules) is skipped, because without `exclude` it would report on files the
//!   manifest may exclude.
//! - Every walked file is read once, rendered or not, so a file that cannot be read is refused at
//!   its own path with the I/O error, as an extension's files are, and never twice: a file that
//!   `exclude` removes, a script and a data file are read for that check alone.
//! - A text that is read but never rendered (`pack.toml`, `SKILL.md`, an agent file, a
//!   `native/<tool>/hooks.toml`) that is not UTF-8 is refused as `<file>: not valid UTF-8`; a
//!   rendered file is refused as `<file>: not valid UTF-8, cannot render`.
//! - Each text is scanned once: frontmatter values by the frontmatter parsers, hook fields by the
//!   manifest parsers, Markdown bodies (after the closing `---` for `SKILL.md` and agent files,
//!   from line 1 for every other `.md`, native ones included) and `render` files here. Every token
//!   found is kept on the pack.
//! - [`Pack::unused_native`] lists the `native/<tool>/` directories no given tool claims, so a
//!   mistyped tool name can be reported.
//!
//! # Non-responsibilities
//!
//! A pack's relation to other packs (required packs, targets in other packs, airsl ranges) belongs
//! to the checks over a whole set of packs. Reading the real disk belongs to the adapter of the
//! [`FileTree`] port that the caller passes in.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use crate::port::FileTree;
use crate::types::VERSION;
use crate::types::frontmatter::AgentMeta;
use crate::types::frontmatter::BodyStart;
use crate::types::frontmatter::SkillMeta;
use crate::types::hooks::NativeHooks;
use crate::types::manifest::PackManifest;
use crate::types::manifest::requires::agentstacks_too_new;
use crate::types::name::ItemName;
use crate::types::name::ToolName;
use crate::types::pack::install::{
    InstallSet, check_airsl_declared, check_something_to_install, is_excluded_by,
};
use crate::types::pack::item::loaded::{PackAgent, PackSkill};
use crate::types::pack::layout::{MANIFEST_FILE, NATIVE_DIR, PackLayout, SKILL_FILE, SKILLS_DIR};
use crate::types::pack::local::{check_local_tokens, check_native_references};
use crate::types::pack::native::NativeDir;
use crate::types::pack::scan::{rendered_text, scan_body};
use crate::types::pack::targets::HookTargets;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tokens::LocatedToken;
use crate::types::tree::TreePath;
use crate::types::tree::Walked;
use crate::types::tree::walk;

/// A pack directory that passed every check.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{Pack, TreePath};
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-tokens");
/// let pack = Pack::load(&StdFileTree::new(dir)).map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(pack.manifest().name().as_str(), "core");
/// assert_eq!(pack.skills().len(), 1);
/// assert_eq!(pack.agents().len(), 1);
/// let rendered: Vec<&str> = pack.rendered().iter().map(TreePath::as_str).collect();
/// assert_eq!(rendered, ["agents/b.md", "skills/a/SKILL.md"]);
/// assert!(pack.unused_native(&[]).is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct Pack {
    dir: PathBuf,
    manifest: PackManifest,
    skills: Vec<PackSkill>,
    agents: Vec<PackAgent>,
    native: Vec<NativeDir>,
    root_files: Vec<TreePath>,
    rendered: Vec<TreePath>,
    tokens: Vec<LocatedToken>,
    ignored: Vec<PathBuf>,
}

impl Pack {
    /// Loads the pack in the directory `tree` is rooted at.
    ///
    /// # Errors
    ///
    /// Every refusal the directory contains; the [`Diagnostics`] hand them back sorted: by file
    /// path, then the refusals without a line in the order found, then by line. A
    /// `requires.agentstacks` range that excludes this agentstacks is the only refusal returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::Pack;
    ///
    /// # fn main() -> Result<(), String> {
    /// let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs");
    ///
    /// let pack = Pack::load(&StdFileTree::new(format!("{fixtures}/valid-hooks")))
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    /// assert_eq!(pack.manifest().name().as_str(), "core");
    ///
    /// let refused = Pack::load(&StdFileTree::new(format!("{fixtures}/no-pack-toml")))
    ///     .err()
    ///     .ok_or("a directory without pack.toml is not a pack")?;
    /// let shown: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
    /// assert!(shown[0].ends_with("no-pack-toml: not a pack (no pack.toml)"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn load<T: FileTree>(tree: &T) -> Result<Self, Diagnostics> {
        let (mut run, walked, layout) = Run::begin(tree)?;
        let skills = run.read_skills(&layout);
        let agents = run.read_agents(&layout);
        let Some(manifest) = run.read_manifest(&layout) else {
            run.read_unread(&layout);
            run.diagnostics
                .extend(check_native_references(&run.dir, &run.tokens));
            return Err(run.diagnostics);
        };
        run.tokens.extend(manifest.tokens().iter().cloned());

        let (mut native_hooks, hooks_refused) = run.read_native_hooks(&layout, &manifest);
        let targets = {
            let pairs: Vec<(&ToolName, &NativeHooks)> = native_hooks.iter().collect();
            HookTargets::collect(manifest.name(), &manifest, &pairs)
        };
        run.diagnostics.extend(targets.check(&run.dir, &layout));
        let (install, refusals) =
            InstallSet::resolve(&run.dir, &layout, &manifest, &targets.run_paths());
        run.diagnostics.extend(refusals);

        let native: Vec<NativeDir> = layout
            .native()
            .iter()
            .map(|(tool, files)| {
                let installed = files
                    .files()
                    .iter()
                    .filter(|file| {
                        let path =
                            TreePath::from_parts(&[NATIVE_DIR, tool.as_str(), file.as_str()]);
                        install.is_installed(&path)
                    })
                    .cloned()
                    .collect();
                NativeDir::new(tool.clone(), installed, native_hooks.remove(tool))
            })
            .collect();
        if let Err(refusal) = check_airsl_declared(&run.dir, &manifest, &install) {
            run.diagnostics.push(refusal);
        }
        if !layout.may_hold_unseen_items()
            && hooks_refused == 0
            && let Err(refusal) = check_something_to_install(&run.dir, &layout, &manifest, &native)
        {
            run.diagnostics.push(refusal);
        }
        run.scan_rendered_files(&layout, &install);
        run.read_unread(&layout);
        run.diagnostics
            .extend(check_native_references(&run.dir, &run.tokens));
        run.diagnostics.extend(check_local_tokens(
            &run.dir,
            manifest.name(),
            &layout,
            &run.tokens,
        ));

        if !run.diagnostics.is_empty() {
            return Err(run.diagnostics);
        }
        let skills = skills
            .into_iter()
            .map(|(name, meta)| {
                let inside = layout.skills().get(&name).map_or(&[][..], Vec::as_slice);
                PackSkill::from_installed(meta, &install, inside)
            })
            .collect();
        let agents = agents
            .into_iter()
            .map(|(meta, file)| PackAgent::new(meta, file))
            .collect();
        Ok(Self {
            dir: run.dir,
            skills,
            agents,
            native,
            root_files: layout
                .root_files()
                .iter()
                .filter(|file| install.is_installed(file))
                .cloned()
                .collect(),
            rendered: layout
                .files()
                .iter()
                .filter(|file| install.is_rendered(file))
                .cloned()
                .collect(),
            tokens: run.tokens,
            ignored: walked.ignored().to_vec(),
            manifest,
        })
    }

    /// The directory the pack was loaded from, as shown in messages.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The pack's `pack.toml`.
    #[must_use]
    pub const fn manifest(&self) -> &PackManifest {
        &self.manifest
    }

    /// The skills, in name order.
    #[must_use]
    pub fn skills(&self) -> &[PackSkill] {
        &self.skills
    }

    /// The agents, in name order.
    #[must_use]
    pub fn agents(&self) -> &[PackAgent] {
        &self.agents
    }

    /// The `native/<tool>/` directories, in tool name order.
    #[must_use]
    pub fn native(&self) -> &[NativeDir] {
        &self.native
    }

    /// The files installed under the pack's install root, relative to the pack root.
    #[must_use]
    pub fn root_files(&self) -> &[TreePath] {
        &self.root_files
    }

    /// Every file that is rendered, relative to the pack root.
    #[must_use]
    pub fn rendered(&self) -> &[TreePath] {
        &self.rendered
    }

    /// Every token found in the pack, wherever it came from.
    #[must_use]
    pub fn tokens(&self) -> &[LocatedToken] {
        &self.tokens
    }

    /// The dot entries the walk ignored, relative to the pack root.
    #[must_use]
    pub fn ignored(&self) -> &[PathBuf] {
        &self.ignored
    }

    /// The `native/<tool>/` directories whose tool is not in `tools`.
    #[must_use]
    pub fn unused_native<'a>(&'a self, tools: &[ToolName]) -> Vec<&'a ToolName> {
        self.native
            .iter()
            .map(NativeDir::tool)
            .filter(|tool| !tools.contains(tool))
            .collect()
    }
}

/// The read of `pack.toml`, or `None` when the walk did not list it as a regular file. The load
/// reads it here once, before any other file, so the range check and the manifest parse see the
/// same text.
fn read_manifest_text<T: FileTree>(tree: &T, walked: &Walked) -> Option<io::Result<String>> {
    walked
        .files()
        .iter()
        .any(|path| path.as_str() == MANIFEST_FILE)
        .then(|| tree.read_text(Path::new(MANIFEST_FILE)))
}

/// One load: the tree being read and what the run has found so far.
struct Run<'a, T: FileTree> {
    tree: &'a T,
    dir: PathBuf,
    diagnostics: Diagnostics,
    tokens: Vec<LocatedToken>,
    /// The walked files a read was attempted on, whether it succeeded or not.
    read: BTreeSet<TreePath>,
    /// The read of `pack.toml` made before the run started, held until the manifest is parsed.
    manifest_text: Option<io::Result<String>>,
}

impl<'a, T: FileTree> Run<'a, T> {
    fn new(tree: &'a T, manifest_text: Option<io::Result<String>>) -> Self {
        Self {
            tree,
            dir: tree.root().to_path_buf(),
            diagnostics: Diagnostics::new(),
            tokens: Vec::new(),
            read: BTreeSet::new(),
            manifest_text,
        }
    }

    /// Walks the tree and starts a run holding the walk's refusals and the classified layout, or
    /// ends the load before anything else is read: with the one refusal of a `requires.agentstacks`
    /// that excludes this agentstacks, or with the walk's refusals and the one that the directory
    /// is not a pack.
    fn begin(tree: &'a T) -> Result<(Self, Walked, PackLayout), Diagnostics> {
        let (walked, walk_refusals) = walk(tree);
        let manifest_text = read_manifest_text(tree, &walked);
        let too_new = manifest_text
            .as_ref()
            .and_then(|read| read.as_deref().ok())
            .and_then(|text| agentstacks_too_new(&tree.root().join(MANIFEST_FILE), text, &VERSION));
        if let Some(only) = too_new {
            return Err(Diagnostics::from(only));
        }
        let mut run = Self::new(tree, manifest_text);
        run.diagnostics.extend(walk_refusals);
        match PackLayout::classify(&run.dir, &walked) {
            Ok((layout, refusals)) => {
                run.diagnostics.extend(refusals);
                Ok((run, walked, layout))
            }
            Err(not_a_pack) => {
                run.diagnostics.push(not_a_pack);
                Err(run.diagnostics)
            }
        }
    }

    /// Reads the file `relative` as text that is never rendered; a refusal is recorded and gives
    /// `None`.
    fn read_text(&mut self, relative: &TreePath) -> Option<(PathBuf, String)> {
        self.read.insert(relative.clone());
        let file = self.dir.join(relative.as_str());
        match self.tree.read_text(Path::new(relative.as_str())) {
            Ok(text) => Some((file, text)),
            Err(error) => {
                self.diagnostics
                    .push(Diagnostic::new(Location::file(&file), error));
                None
            }
        }
    }

    /// Reads the file `relative` as text that is rendered; a refusal is recorded and gives `None`.
    fn read_rendered(&mut self, relative: &TreePath) -> Option<(PathBuf, String)> {
        self.read.insert(relative.clone());
        let file = self.dir.join(relative.as_str());
        let bytes = match self.tree.read_file(Path::new(relative.as_str())) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.diagnostics
                    .push(Diagnostic::new(Location::file(&file), error));
                return None;
            }
        };
        match rendered_text(&file, bytes) {
            Ok(text) => Some((file, text)),
            Err(refusal) => {
                self.diagnostics.push(refusal);
                None
            }
        }
    }

    /// Reads every walked file no other step read, so that a file that cannot be read is refused
    /// here once. The bytes are dropped: nothing about such a file is checked.
    fn read_unread(&mut self, layout: &PackLayout) {
        for relative in layout.files() {
            if self.read.contains(relative) {
                continue;
            }
            if let Err(error) = self.tree.read_file(Path::new(relative.as_str())) {
                let file = self.dir.join(relative.as_str());
                self.diagnostics
                    .push(Diagnostic::new(Location::file(&file), error));
            }
        }
    }

    /// Scans the Markdown body that begins at `start` in `text`, once.
    fn scan_markdown_body(&mut self, file: &Path, text: &str, start: BodyStart) {
        let (found, refusals) = scan_body(file, start.body(text), start.line());
        self.tokens.extend(found);
        self.diagnostics.extend(refusals);
    }

    /// Reads each skill's `SKILL.md`: frontmatter first, then the body when the frontmatter is
    /// accepted.
    fn read_skills(&mut self, layout: &PackLayout) -> Vec<(ItemName, SkillMeta)> {
        let mut skills = Vec::new();
        for (name, inside) in layout.skills() {
            if !inside.iter().any(|file| file.as_str() == SKILL_FILE) {
                continue;
            }
            let path = TreePath::from_parts(&[SKILLS_DIR, name.as_str(), SKILL_FILE]);
            let Some((file, text)) = self.read_text(&path) else {
                continue;
            };
            match SkillMeta::parse(&file, &text, name) {
                Ok(meta) => {
                    self.tokens.extend(meta.tokens().iter().cloned());
                    self.scan_markdown_body(&file, &text, meta.body_start());
                    skills.push((name.clone(), meta));
                }
                Err(refusals) => self.diagnostics.extend(refusals),
            }
        }
        skills
    }

    /// Reads each agent file the same way.
    fn read_agents(&mut self, layout: &PackLayout) -> Vec<(AgentMeta, TreePath)> {
        let mut agents = Vec::new();
        for (name, path) in layout.agents() {
            let Some((file, text)) = self.read_text(path) else {
                continue;
            };
            match AgentMeta::parse(&file, &text, name) {
                Ok(meta) => {
                    self.tokens.extend(meta.tokens().iter().cloned());
                    self.scan_markdown_body(&file, &text, meta.body_start());
                    agents.push((meta, path.clone()));
                }
                Err(refusals) => self.diagnostics.extend(refusals),
            }
        }
        agents
    }

    /// Parses `pack.toml` from the text read when the run began; a failed read is recorded here.
    /// `None` when it is refused, or when the walk already refused it.
    fn read_manifest(&mut self, layout: &PackLayout) -> Option<PackManifest> {
        let path = TreePath::from_parts(&[MANIFEST_FILE]);
        if !layout.files().contains(&path) {
            return None;
        }
        self.read.insert(path);
        let file = self.dir.join(MANIFEST_FILE);
        let text = match self.manifest_text.take()? {
            Ok(text) => text,
            Err(error) => {
                self.diagnostics
                    .push(Diagnostic::new(Location::file(&file), error));
                return None;
            }
        };
        match PackManifest::parse(&file, &text) {
            Ok(manifest) => Some(manifest),
            Err(refusals) => {
                self.diagnostics.extend(refusals);
                None
            }
        }
    }

    /// Parses each `native/<tool>/hooks.toml` the manifest does not exclude, and counts the ones
    /// that were refused.
    fn read_native_hooks(
        &mut self,
        layout: &PackLayout,
        manifest: &PackManifest,
    ) -> (BTreeMap<ToolName, NativeHooks>, usize) {
        let mut parsed = BTreeMap::new();
        let mut refused = 0;
        for (tool, native) in layout.native() {
            let Some(path) = native.hooks() else { continue };
            if is_excluded_by(manifest, path) {
                continue;
            }
            let Some((file, text)) = self.read_text(path) else {
                refused += 1;
                continue;
            };
            match NativeHooks::parse(&file, &text, manifest.requires()) {
                Ok(hooks) => {
                    self.tokens.extend(hooks.tokens().iter().cloned());
                    parsed.insert(tool.clone(), hooks);
                }
                Err(refusals) => {
                    self.diagnostics.extend(refusals);
                    refused += 1;
                }
            }
        }
        (parsed, refused)
    }

    /// Scans, from line 1, every rendered file whose text was not already scanned as a `SKILL.md`
    /// or an agent file.
    fn scan_rendered_files(&mut self, layout: &PackLayout, install: &InstallSet) {
        let scanned: BTreeSet<TreePath> = layout
            .skills()
            .keys()
            .map(|name| TreePath::from_parts(&[SKILLS_DIR, name.as_str(), SKILL_FILE]))
            .chain(layout.agents().values().cloned())
            .collect();
        for path in layout.files() {
            if !install.is_rendered(path) || scanned.contains(path) {
                continue;
            }
            if let Some((file, text)) = self.read_rendered(path) {
                let (found, refusals) = scan_body(&file, &text, NonZeroUsize::MIN);
                self.tokens.extend(found);
                self.diagnostics.extend(refusals);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;
    use crate::types::API;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const ROOT: &str = "packs/core";
    const IDENTITY: &str =
        "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n";
    const DENIED: &str = "permission denied";
    const SYMLINK: &str = "symlinks are not allowed in a pack";
    const NOT_UTF8: &str = "not valid UTF-8";

    /// A Markdown file whose frontmatter is lines 1 to 4 and whose line 6 is `tail`.
    fn doc(name: &str, tail: &str) -> String {
        format!("---\nname: {name}\ndescription: d\n---\n\n{tail}\n")
    }

    fn base() -> FakeFileTree {
        FakeFileTree::new(ROOT)
            .file("pack.toml", IDENTITY.as_bytes())
            .file("skills/a/SKILL.md", doc("a", "body").as_bytes())
    }

    fn manifest_text(top: &str, rest: &str) -> String {
        format!("{top}\n{IDENTITY}{rest}")
    }

    fn with_manifest(top: &str, rest: &str) -> FakeFileTree {
        base().file("pack.toml", manifest_text(top, rest).as_bytes())
    }

    fn loaded(tree: &FakeFileTree) -> Result<Pack, Box<dyn Error>> {
        Pack::load(tree).map_err(|refusals| shown(refusals).join("; ").into())
    }

    fn refusals(tree: &FakeFileTree) -> Result<Vec<String>, Box<dyn Error>> {
        match Pack::load(tree) {
            Ok(_) => Err("the pack loaded, expected refusals".into()),
            Err(refusals) => Ok(shown(refusals)),
        }
    }

    fn shown(refusals: Diagnostics) -> Vec<String> {
        refusals
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn names<'a, I: Iterator<Item = &'a TreePath>>(paths: I) -> Vec<&'a str> {
        paths.map(TreePath::as_str).collect()
    }

    #[test]
    fn load_returns_every_part_of_a_valid_pack() -> TestResult {
        let manifest = manifest_text(
            "",
            "[requires]\nairsl = \">=0.1.4\"\n\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"scripts/s.lua\"\n",
        );
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", manifest.as_bytes())
            .file("skills/a/SKILL.md", doc("a", "body").as_bytes())
            .file("agents/b.md", doc("b", "body").as_bytes())
            .file("native/claude/x.json", b"{}")
            .file("scripts/s.lua", b"print(1)\n");
        let pack = loaded(&tree)?;
        let skill_names: Vec<&str> = pack.skills().iter().map(|s| s.name().as_str()).collect();
        assert_eq!(skill_names, ["a"]);
        let agent_names: Vec<&str> = pack.agents().iter().map(|a| a.name().as_str()).collect();
        assert_eq!(agent_names, ["b"]);
        assert_eq!(pack.native().len(), 1);
        assert_eq!(names(pack.native()[0].files().iter()), ["x.json"]);
        assert_eq!(names(pack.root_files().iter()), ["scripts/s.lua"]);
        assert_eq!(pack.dir(), Path::new(ROOT));
        assert_eq!(pack.manifest().name().as_str(), "core");
        Ok(())
    }

    #[test]
    fn load_keeps_tokens_from_every_source() -> TestResult {
        let manifest = manifest_text(
            "render = [\"data.json\"]",
            "[requires]\nairsl = \">=0.1.4\"\n\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"scripts/s.lua\"\nargs = [\"{{root}}/x\"]\n",
        );
        let native = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"{{skill a}}\"\n";
        let skill = "---\nname: a\ndescription: \"{{agent b}}\"\n---\n\n{{skill a}}\n";
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", manifest.as_bytes())
            .file("skills/a/SKILL.md", skill.as_bytes())
            .file("agents/b.md", doc("b", "body").as_bytes())
            .file("scripts/s.lua", b"x\n")
            .file("data.json", b"{{skill a}}\n")
            .file("native/claude/hooks.toml", native.as_bytes());
        let pack = loaded(&tree)?;
        let at: BTreeSet<String> = pack
            .tokens()
            .iter()
            .map(|t| t.location().to_string())
            .collect();
        let in_skill = pack
            .tokens()
            .iter()
            .filter(|t| {
                t.location()
                    .file_path()
                    .is_some_and(|file| file.ends_with("SKILL.md"))
            })
            .count();
        assert_eq!(in_skill, 2, "each text of the skill is scanned once");
        for expected in [
            format!("{ROOT}/skills/a/SKILL.md: description"),
            format!("{ROOT}/skills/a/SKILL.md:6"),
            format!("{ROOT}/data.json:1"),
            format!("{ROOT}/pack.toml: hooks[0].args[0]"),
            format!("{ROOT}/native/claude/hooks.toml: hooks[0].text"),
        ] {
            assert!(at.contains(&expected), "{expected} not in {at:?}");
        }
        Ok(())
    }

    #[test]
    fn load_scans_each_text_once() -> TestResult {
        let skill = "---\nname: a\ndescription: \"{{skil x}}\"\n---\n\nbody\n";
        let tree = base().file("skills/a/SKILL.md", skill.as_bytes());
        assert_eq!(
            refusals(&tree)?,
            [format!(
                "{ROOT}/skills/a/SKILL.md: description: unknown token kind \"skil\""
            )]
        );
        Ok(())
    }

    #[test]
    fn load_does_not_scan_excluded_files() -> TestResult {
        let tree = with_manifest("exclude = [\"docs/**\"]", "").file("docs/x.md", b"{{bad}}\n");
        let pack = loaded(&tree)?;
        assert!(!names(pack.rendered().iter()).contains(&"docs/x.md"));
        Ok(())
    }

    #[test]
    fn load_does_not_hold_native_md_to_frontmatter_rules() -> TestResult {
        let tree = base().file(
            "native/claude/agents/x.md",
            b"---\nmodel: sonnet\n---\n\nbody\n",
        );
        loaded(&tree)?;
        Ok(())
    }

    #[test]
    fn load_allows_native_content_to_reference_native() -> TestResult {
        let tree = base().file("native/claude/x.md", b"{{root}}/native/claude/x\n");
        loaded(&tree)?;
        Ok(())
    }

    #[test]
    fn load_stops_at_a_missing_pack_toml() -> TestResult {
        let tree =
            FakeFileTree::new(ROOT).file("skills/a/SKILL.md", doc("a", "{{skil x}}").as_bytes());
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}: not a pack (no pack.toml)")]
        );
        Ok(())
    }

    #[test]
    fn load_continues_past_a_refused_pack_toml() -> TestResult {
        let skill = "---\nname: a\ndescription: d\ndisable-model-invocation: true\n---\n\nbody\n";
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", b"a = [1,")
            .file("skills/a/SKILL.md", skill.as_bytes())
            .file("agents/b.md", doc("b", "{{skil x}}").as_bytes());
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}/agents/b.md:6: unknown token kind \"skil\""),
                format!("{ROOT}/pack.toml:1: unclosed array, expected `]`"),
                format!("{ROOT}/skills/a/SKILL.md: unknown key \"disable-model-invocation\""),
            ]
        );
        Ok(())
    }

    #[test]
    fn load_still_checks_native_references_when_pack_toml_is_refused() -> TestResult {
        let tree = FakeFileTree::new(ROOT).file("pack.toml", b"a = [1,").file(
            "skills/a/SKILL.md",
            doc("a", "{{root}}/native/claude/x").as_bytes(),
        );
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}/pack.toml:1: unclosed array, expected `]`"),
                format!("{ROOT}/skills/a/SKILL.md:6: neutral content must not reference native/"),
            ]
        );
        Ok(())
    }

    #[test]
    fn load_skips_the_body_of_a_refused_frontmatter() -> TestResult {
        let skill = "---\nname: [a\ndescription: d\n---\n\n{{skil x}}\n";
        let tree = base().file("skills/a/SKILL.md", skill.as_bytes());
        let found = refusals(&tree)?;
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with(&format!("{ROOT}/skills/a/SKILL.md")));
        assert!(!found[0].contains("unknown token kind"), "{found:?}");
        Ok(())
    }

    #[test]
    fn load_refuses_a_non_utf8_render_file() -> TestResult {
        let tree = with_manifest("render = [\"data.json\"]", "").file("data.json", &[0x66, 0xFF]);
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}/data.json: not valid UTF-8, cannot render")]
        );
        Ok(())
    }

    #[test]
    fn load_refuses_a_non_utf8_text_that_is_never_rendered() -> TestResult {
        let cases: [(FakeFileTree, &str); 4] = [
            (base().file("pack.toml", &[0xFF, 0xFE]), "pack.toml"),
            (
                base().file("skills/a/SKILL.md", &[0xFF, 0xFE]),
                "skills/a/SKILL.md",
            ),
            (base().file("agents/b.md", &[0xFF, 0xFE]), "agents/b.md"),
            (
                base().file("native/claude/hooks.toml", &[0xFF, 0xFE]),
                "native/claude/hooks.toml",
            ),
        ];
        for (tree, file) in cases {
            assert_eq!(
                refusals(&tree)?,
                [format!("{ROOT}/{file}: {NOT_UTF8}")],
                "{file}"
            );
        }
        Ok(())
    }

    #[test]
    fn load_reports_an_unreadable_file_as_io() -> TestResult {
        let cases: [(FakeFileTree, &str); 5] = [
            (base().unreadable("pack.toml"), "pack.toml"),
            (base().unreadable("skills/a/SKILL.md"), "skills/a/SKILL.md"),
            (
                base()
                    .file("agents/b.md", doc("b", "body").as_bytes())
                    .unreadable("agents/b.md"),
                "agents/b.md",
            ),
            (
                with_manifest("render = [\"data.json\"]", "").unreadable("data.json"),
                "data.json",
            ),
            (
                base()
                    .file("native/claude/x.json", b"{}")
                    .unreadable("native/claude/hooks.toml"),
                "native/claude/hooks.toml",
            ),
        ];
        for (tree, file) in cases {
            assert_eq!(
                refusals(&tree)?,
                [format!("{ROOT}/{file}: {DENIED}")],
                "{file}"
            );
        }
        Ok(())
    }

    #[test]
    fn load_keeps_checking_past_walk_refusals() -> TestResult {
        let skill = "---\nname: a\ndescription: d\ndisable-model-invocation: true\n---\n\nbody\n";
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", b"a = [1,")
            .file("skills/a/SKILL.md", skill.as_bytes())
            .file("agents/Bad.md", b"")
            .symlink("link")
            .special("sock");
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}/agents/Bad.md: \"Bad\" is not a valid name"),
                format!("{ROOT}/link: {SYMLINK}"),
                format!("{ROOT}/pack.toml:1: unclosed array, expected `]`"),
                format!("{ROOT}/skills/a/SKILL.md: unknown key \"disable-model-invocation\""),
                format!("{ROOT}/sock: only regular files and directories are allowed"),
            ]
        );
        Ok(())
    }

    #[test]
    fn load_reports_a_walk_refusal_beside_the_other_refusals_of_a_valid_looking_pack() -> TestResult
    {
        assert_eq!(
            refusals(&base().symlink("link"))?,
            [format!("{ROOT}/link: {SYMLINK}")]
        );
        assert_eq!(
            refusals(&base().special("sock"))?,
            [format!(
                "{ROOT}/sock: only regular files and directories are allowed"
            )]
        );
        Ok(())
    }

    #[test]
    fn load_reports_an_unlistable_directory_and_checks_the_rest() -> TestResult {
        let tree = with_manifest("exclude = [\"[\"]", "").unlistable_dir("x");
        let found = refusals(&tree)?;
        assert!(found.contains(&format!("{ROOT}/x: {DENIED}")), "{found:?}");
        assert!(
            found.iter().any(|m| m.contains("is not a valid glob")),
            "{found:?}"
        );
        Ok(())
    }

    fn requiring(range: &str, identity_api: u32, top: &str) -> String {
        format!(
            "{top}\n[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = {identity_api}\ndescription = \"d\"\n\n[requires]\nagentstacks = \"{range}\"\n"
        )
    }

    #[test]
    fn excluding_range_is_the_only_refusal() -> TestResult {
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", requiring(">=99", 2, "zzz = 1").as_bytes())
            .file("skills/a/SKILL.md", doc("a", "{{bogus x}}").as_bytes())
            .symlink("link");
        assert_eq!(
            refusals(&tree)?,
            [format!(
                "{ROOT}/pack.toml: requires.agentstacks: \">=99\" does not match agentstacks {VERSION} (install a newer agentstacks)"
            )]
        );
        Ok(())
    }

    #[test]
    fn invalid_range_is_one_refusal_among_others() -> TestResult {
        let tree = base().file("pack.toml", requiring("nope", 1, "zzz = 1").as_bytes());
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}/pack.toml: zzz: unknown key"),
                format!("{ROOT}/pack.toml: requires.agentstacks: \"nope\" is not a version range"),
            ]
        );
        Ok(())
    }

    #[test]
    fn absent_range_loads() -> TestResult {
        loaded(&base())?;
        Ok(())
    }

    #[test]
    fn matching_range_loads() -> TestResult {
        let tree = base().file("pack.toml", requiring(">=1.0", 1, "").as_bytes());
        let pack = loaded(&tree)?;
        assert_eq!(
            pack.manifest()
                .requires()
                .agentstacks()
                .map(ToString::to_string),
            Some(">=1.0".to_owned())
        );
        Ok(())
    }

    #[test]
    fn unreadable_or_invalid_toml_is_refused_by_the_parse_not_the_range_check() -> TestResult {
        let tree = base().file("pack.toml", b"agentstacks = \">=99\"\na = [1,");
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}/pack.toml:2: unclosed array, expected `]`")]
        );
        let tree = base().file("pack.toml", b"\xff\xfe");
        assert_eq!(refusals(&tree)?, [format!("{ROOT}/pack.toml: {NOT_UTF8}")]);
        let tree = base().unreadable("pack.toml");
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}/pack.toml: {}", FakeFileTree::denied())]
        );
        Ok(())
    }

    #[test]
    fn load_does_not_call_a_pack_toml_the_walk_refused_absent() -> TestResult {
        let tree = FakeFileTree::new(ROOT)
            .symlink("pack.toml")
            .file("skills/a/SKILL.md", doc("a", "body").as_bytes());
        assert_eq!(refusals(&tree)?, [format!("{ROOT}/pack.toml: {SYMLINK}")]);
        Ok(())
    }

    #[test]
    fn load_does_not_call_a_skill_md_the_walk_refused_missing() -> TestResult {
        let tree = base()
            .file("skills/b/references/r.md", b"r\n")
            .symlink("skills/b/SKILL.md");
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}/skills/b/SKILL.md: {SYMLINK}")]
        );
        Ok(())
    }

    #[test]
    fn load_does_not_call_a_run_or_a_render_file_the_walk_refused_missing() -> TestResult {
        let manifest = manifest_text(
            "render = [\"data.json\"]",
            "[requires]\nairsl = \">=0.1.4\"\n\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/a.lua\"\n",
        );
        let tree = base()
            .file("pack.toml", manifest.as_bytes())
            .symlink("hooks/a.lua")
            .symlink("data.json");
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}/data.json: {SYMLINK}"),
                format!("{ROOT}/hooks/a.lua: {SYMLINK}"),
            ]
        );
        Ok(())
    }

    #[test]
    fn load_does_not_call_a_pack_empty_for_the_only_item_it_refused() -> TestResult {
        let hooks_only = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"t\"\n";
        let bare = || {
            FakeFileTree::new(ROOT)
                .file("pack.toml", IDENTITY.as_bytes())
                .file("README.md", b"r\n")
        };
        let cases: [(FakeFileTree, String); 6] = [
            (
                bare().file("skills/My_Skill/SKILL.md", b""),
                "skills/My_Skill: \"My_Skill\" is not a valid name".to_owned(),
            ),
            (
                bare().file("agents/Coder.md", b""),
                "agents/Coder.md: \"Coder\" is not a valid name".to_owned(),
            ),
            (
                bare().file("native/Claude/hooks.toml", hooks_only.as_bytes()),
                "native/Claude: \"Claude\" is not a valid name".to_owned(),
            ),
            (
                bare().file("native/claude/hooks.toml", b"a = [1,"),
                "native/claude/hooks.toml:1: unclosed array, expected `]`".to_owned(),
            ),
            (
                bare().file("native/claude/hooks.toml", &[0xFF]),
                format!("native/claude/hooks.toml: {NOT_UTF8}"),
            ),
            (
                bare().symlink("agents/b.md"),
                format!("agents/b.md: {SYMLINK}"),
            ),
        ];
        for (tree, expected) in cases {
            assert_eq!(refusals(&tree)?, [format!("{ROOT}/{expected}")]);
        }
        Ok(())
    }

    #[test]
    fn load_still_reports_an_empty_pack_when_nothing_was_refused() -> TestResult {
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", IDENTITY.as_bytes())
            .file("README.md", b"r\n")
            .file("commands/x.md", b"x\n");
        assert_eq!(
            refusals(&tree)?,
            [
                format!("{ROOT}: pack declares nothing to install"),
                format!("{ROOT}/commands: reserved, not supported in api {API}"),
            ]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn load_does_not_call_a_pack_empty_for_a_non_utf8_item_directory() -> TestResult {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad = OsStr::from_bytes(&[0x66, 0xFF]);
        let tree = FakeFileTree::new(ROOT)
            .file("pack.toml", IDENTITY.as_bytes())
            .file(
                std::path::Path::new("skills").join(bad).join("SKILL.md"),
                b"",
            );
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}/skills/f\u{FFFD}: path is not valid UTF-8")]
        );
        Ok(())
    }

    #[test]
    fn load_reports_only_the_io_refusal_for_a_pack_directory_that_cannot_be_listed() -> TestResult {
        let tree = FakeFileTree::missing(ROOT);
        assert_eq!(
            refusals(&tree)?,
            [format!("{ROOT}: {}", FakeFileTree::not_found())]
        );
        Ok(())
    }

    #[test]
    fn load_refuses_an_unreadable_file_that_nothing_else_reads() -> TestResult {
        let cases: [(FakeFileTree, &str); 4] = [
            (
                base()
                    .file("scripts/run.sh", b"x")
                    .unreadable("scripts/run.sh"),
                "scripts/run.sh",
            ),
            (
                base()
                    .file("skills/a/scripts/x.sh", b"x")
                    .unreadable("skills/a/scripts/x.sh"),
                "skills/a/scripts/x.sh",
            ),
            (
                base()
                    .file("native/claude/x.json", b"{}")
                    .unreadable("native/claude/x.json"),
                "native/claude/x.json",
            ),
            (
                with_manifest("exclude = [\"data/*\"]", "")
                    .file("data/x.bin", b"x")
                    .unreadable("data/x.bin"),
                "data/x.bin",
            ),
        ];
        for (tree, file) in cases {
            assert_eq!(
                refusals(&tree)?,
                [format!("{ROOT}/{file}: {DENIED}")],
                "{file}"
            );
        }
        Ok(())
    }

    #[test]
    fn load_refuses_an_unreadable_markdown_file_once() -> TestResult {
        let tree = base().file("README.md", b"r\n").unreadable("README.md");
        assert_eq!(refusals(&tree)?, [format!("{ROOT}/README.md: {DENIED}")]);
        Ok(())
    }

    #[test]
    fn load_refuses_an_unreadable_file_beside_a_refused_pack_toml() -> TestResult {
        let tree = base()
            .file("pack.toml", b"a = [1,")
            .file("data/x.bin", b"x")
            .unreadable("data/x.bin");
        let found = refusals(&tree)?;
        assert!(
            found.contains(&format!("{ROOT}/data/x.bin: {DENIED}")),
            "{found:?}"
        );
        assert_eq!(found.len(), 2, "{found:?}");
        Ok(())
    }

    #[test]
    fn load_refuses_a_render_entry_that_is_never_installed() -> TestResult {
        let tree = with_manifest("render = [\"pack.toml\"]", "");
        assert_eq!(
            refusals(&tree)?,
            [format!(
                "{ROOT}/pack.toml: render[0]: \"pack.toml\" is never installed"
            )]
        );
        let native = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"t\"\n";
        let tree = with_manifest("render = [\"native/claude/hooks.toml\"]", "")
            .file("native/claude/hooks.toml", native.as_bytes());
        assert_eq!(
            refusals(&tree)?,
            [format!(
                "{ROOT}/pack.toml: render[0]: \"native/claude/hooks.toml\" is never installed"
            )]
        );
        Ok(())
    }

    #[test]
    fn load_allows_arguments_only_in_skills_name_skill_md() -> TestResult {
        let tree = base()
            .file(
                "skills/a/SKILL.md",
                doc("a", "{{var arguments}}").as_bytes(),
            )
            .file("docs/SKILL.md", b"{{var arguments}}\n");
        assert_eq!(
            refusals(&tree)?,
            [format!(
                "{ROOT}/docs/SKILL.md:1: var \"arguments\" is only valid in SKILL.md"
            )]
        );
        Ok(())
    }

    #[test]
    fn load_ignores_empty_directories() -> TestResult {
        let tree = base()
            .dir("skills/b")
            .dir("agents/x")
            .dir("native/claude")
            .dir("commands");
        let pack = loaded(&tree)?;
        let skill_names: Vec<&str> = pack.skills().iter().map(|s| s.name().as_str()).collect();
        assert_eq!(skill_names, ["a"]);
        assert!(pack.native().is_empty());
        Ok(())
    }

    #[test]
    fn load_keeps_ignored_dot_entries() -> TestResult {
        let pack = loaded(&base().file(".DS_Store", b""))?;
        assert_eq!(pack.ignored(), [PathBuf::from(".DS_Store")]);
        assert!(pack.root_files().is_empty());
        Ok(())
    }

    #[test]
    fn unused_native_lists_unclaimed_tool_dirs() -> TestResult {
        let tree = base()
            .file("native/claude/x.json", b"{}")
            .file("native/cluade/x.json", b"{}");
        let pack = loaded(&tree)?;
        let claimed = [ToolName::new("claude")?];
        let unused: Vec<&str> = pack
            .unused_native(&claimed)
            .into_iter()
            .map(ToolName::as_str)
            .collect();
        assert_eq!(unused, ["cluade"]);
        Ok(())
    }
}
