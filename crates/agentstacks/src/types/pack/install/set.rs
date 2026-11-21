//! Which files of a pack are installed, and which of those are rendered.
//!
//! It is separate from the layout because the layout says where a file belongs and never reads the
//! manifest, while installing needs the manifest's `exclude` and `render` lists.
//!
//! # Responsibilities
//!
//! - [`InstallSet::resolve`] matches each `exclude` pattern of the manifest against every walked
//!   file. An excluded file is not installed and not scanned.
//! - The installed files are the root files, the files of each skill, each agent file and the
//!   files of each `native/<tool>/`. `pack.toml` and every `native/<tool>/hooks.toml` are never
//!   installed.
//! - The rendered files are every installed `.md` file and every `render` entry.
//! - It refuses a `render` entry that is not a walked file, a `render` entry that an `exclude`
//!   pattern matches, a `render` entry that is walked but never installed (`pack.toml`, a
//!   `native/<tool>/hooks.toml`), and an `exclude` pattern that matches `pack.toml`, a `SKILL.md`,
//!   an agent file or the file of an own-pack hook `run`, once per pattern and file. A file that is
//!   also listed in `render` gets only the `render` refusal. Each refusal is located at
//!   `pack.toml`, at the key path `render[<index>]` or `exclude[<index>]`, the index being that of
//!   the entry the refusal is about. A `render` entry the walk refused is not reported as
//!   missing.
//!
//! # Non-responsibilities
//!
//! Compiling and validating the patterns and the `render` paths belongs to the manifest. Reading
//! and scanning the rendered files belongs to the pack loader.

use std::collections::BTreeSet;
use std::path::Path;

use crate::types::manifest::PackManifest;
use crate::types::pack::error::PackError;
use crate::types::pack::layout::{MANIFEST_FILE, NATIVE_DIR, PackLayout, SKILL_FILE, SKILLS_DIR};
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tree::TreePath;

/// The excluded, installed and rendered files of a pack, as pack-relative paths.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{FileTree, InstallSet, PackLayout, PackManifest, TreePath, walk};
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-render-exclude");
/// let tree = StdFileTree::new(dir);
/// let (walked, _) = walk(&tree);
/// let (layout, _) = PackLayout::classify(tree.root(), &walked).map_err(|d| d.to_string())?;
/// let manifest = PackManifest::parse(Path::new("pack.toml"), &tree.read_text(Path::new("pack.toml"))?)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// let (install, refusals) = InstallSet::resolve(Path::new("packs/core"), &layout, &manifest, &[]);
/// assert!(refusals.is_empty());
///
/// // `exclude = ["**/*_test.lua"]` keeps the test script out; `pack.toml` is never installed.
/// let installed: Vec<&str> = install.installed().map(TreePath::as_str).collect();
/// assert_eq!(installed, ["enforcement.json", "scripts/a.lua", "skills/a/SKILL.md"]);
/// // Rendered: every installed `.md` file and each `render` entry.
/// let rendered: Vec<&str> = install.rendered().map(TreePath::as_str).collect();
/// assert_eq!(rendered, ["enforcement.json", "skills/a/SKILL.md"]);
/// let test_script = layout
///     .files()
///     .iter()
///     .find(|file| file.as_str() == "scripts/a_test.lua")
///     .ok_or("the fixture holds the test script")?;
/// assert!(install.is_excluded(test_script));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct InstallSet {
    excluded: BTreeSet<TreePath>,
    installed: BTreeSet<TreePath>,
    rendered: BTreeSet<TreePath>,
}

impl InstallSet {
    /// Resolves the files of `layout` against the `exclude` and `render` lists of `manifest`.
    ///
    /// `protected_runs` are the pack-relative files of the pack's own hook `run`s, which `exclude`
    /// may not remove. Refusals are located under `dir`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::{FileTree, InstallSet, PackLayout, PackManifest, walk};
    ///
    /// let dir = concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/tests/fixtures/packs/exclude-protected-skill-md"
    /// );
    /// let tree = StdFileTree::new(dir);
    /// let (walked, _) = walk(&tree);
    /// let (layout, _) = PackLayout::classify(tree.root(), &walked).map_err(|d| d.to_string())?;
    /// let manifest = PackManifest::parse(Path::new("pack.toml"), &tree.read_text(Path::new("pack.toml"))?)
    ///     .map_err(|refusals| format!("{refusals:?}"))?;
    ///
    /// // `exclude` may never remove a `SKILL.md`.
    /// let (_, refusals) = InstallSet::resolve(Path::new("packs/core"), &layout, &manifest, &[]);
    /// let shown: Vec<String> = refusals.into_sorted().iter().map(ToString::to_string).collect();
    /// assert_eq!(
    ///     shown,
    ///     ["packs/core/pack.toml: exclude[0]: \"**/*.md\" excludes skills/discuss/SKILL.md"]
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn resolve(
        dir: &Path,
        layout: &PackLayout,
        manifest: &PackManifest,
        protected_runs: &[TreePath],
    ) -> (Self, Diagnostics) {
        let walked: BTreeSet<&TreePath> = layout.files().iter().collect();
        let excluded: BTreeSet<TreePath> = walked
            .iter()
            .filter(|file| is_excluded_by(manifest, file))
            .map(|file| (*file).clone())
            .collect();
        let installed: BTreeSet<TreePath> = installable(layout)
            .into_iter()
            .filter(|file| !excluded.contains(file))
            .collect();
        let manifest_file = dir.join(MANIFEST_FILE);
        let mut refusals = Diagnostics::new();

        let mut rendered: BTreeSet<TreePath> = installed
            .iter()
            .filter(|file| file.has_extension("md"))
            .cloned()
            .collect();
        let mut render_listed: BTreeSet<TreePath> = BTreeSet::new();
        for (index, written) in manifest.render().iter().enumerate() {
            let path = TreePath::from_parts(&[written.as_str()]);
            let text = written.as_str().to_owned();
            let (location, error) = if !walked.contains(&path) {
                if layout.is_refused(&path) {
                    render_listed.insert(path);
                    continue;
                }
                (
                    Location::at_key(&manifest_file, format!("render[{index}]")),
                    PackError::DoesNotExist { path: text },
                )
            } else if excluded.contains(&path) {
                (
                    Location::at_key(&manifest_file, format!("render[{index}]")),
                    PackError::AlsoExcluded { path: text },
                )
            } else if !installed.contains(&path) {
                (
                    Location::at_key(&manifest_file, format!("render[{index}]")),
                    PackError::NeverInstalled { path: text },
                )
            } else {
                rendered.insert(path.clone());
                render_listed.insert(path);
                continue;
            };
            render_listed.insert(path);
            refusals.push(Diagnostic::new(location, error));
        }

        let protected = protected_files(layout, &walked, protected_runs);
        for (index, glob) in manifest.exclude().iter().enumerate() {
            for file in &protected {
                if glob.matches(file.as_str()) && !render_listed.contains(file) {
                    refusals.push(Diagnostic::new(
                        Location::at_key(&manifest_file, format!("exclude[{index}]")),
                        PackError::ExcludesProtected {
                            glob: glob.pattern().to_owned(),
                            path: file.to_string(),
                        },
                    ));
                }
            }
        }
        let set = Self {
            excluded,
            installed,
            rendered,
        };
        (set, refusals)
    }

    /// Whether an `exclude` pattern matches the pack-relative file `relative`.
    #[must_use]
    pub fn is_excluded(&self, relative: &TreePath) -> bool {
        self.excluded.contains(relative)
    }

    /// Whether the pack-relative file `relative` is installed.
    #[must_use]
    pub fn is_installed(&self, relative: &TreePath) -> bool {
        self.installed.contains(relative)
    }

    /// Whether the pack-relative file `relative` is rendered.
    #[must_use]
    pub fn is_rendered(&self, relative: &TreePath) -> bool {
        self.rendered.contains(relative)
    }

    /// The installed files, sorted by path bytes.
    pub fn installed(&self) -> impl Iterator<Item = &TreePath> {
        self.installed.iter()
    }

    /// The rendered files, sorted by path bytes.
    pub fn rendered(&self) -> impl Iterator<Item = &TreePath> {
        self.rendered.iter()
    }
}

/// Whether any `exclude` pattern of `manifest` matches the pack-relative file `relative`.
pub(crate) fn is_excluded_by(manifest: &PackManifest, relative: &TreePath) -> bool {
    manifest
        .exclude()
        .iter()
        .any(|glob| glob.matches(relative.as_str()))
}

/// Every file the layout would install before `exclude` is applied, pack-relative.
fn installable(layout: &PackLayout) -> Vec<TreePath> {
    let mut files: Vec<TreePath> = layout.root_files().to_vec();
    for (name, inside) in layout.skills() {
        files.extend(
            inside
                .iter()
                .map(|file| TreePath::from_parts(&[SKILLS_DIR, name.as_str(), file.as_str()])),
        );
    }
    files.extend(layout.agents().values().cloned());
    for (tool, native) in layout.native() {
        files.extend(
            native
                .files()
                .iter()
                .map(|file| TreePath::from_parts(&[NATIVE_DIR, tool.as_str(), file.as_str()])),
        );
    }
    files
}

/// The walked files `exclude` may not remove: `pack.toml`, each `SKILL.md`, each agent file and
/// each own-pack hook `run`.
fn protected_files(
    layout: &PackLayout,
    walked: &BTreeSet<&TreePath>,
    protected_runs: &[TreePath],
) -> BTreeSet<TreePath> {
    let mut files: BTreeSet<TreePath> = BTreeSet::new();
    files.insert(TreePath::from_parts(&[MANIFEST_FILE]));
    for (name, inside) in layout.skills() {
        if inside.iter().any(|file| file.as_str() == SKILL_FILE) {
            files.insert(TreePath::from_parts(&[
                SKILLS_DIR,
                name.as_str(),
                SKILL_FILE,
            ]));
        }
    }
    files.extend(layout.agents().values().cloned());
    files.extend(protected_runs.iter().cloned());
    files.retain(|file| walked.contains(file));
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::pack::testing::{DIR, layout, manifest_with, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn tp(text: &str) -> TreePath {
        TreePath::from_parts(&[text])
    }

    fn resolve(
        files: &[&str],
        top: &str,
        runs: &[&str],
    ) -> Result<(InstallSet, Vec<String>), Box<dyn Error>> {
        let manifest = manifest_with(top, "")?;
        let runs: Vec<TreePath> = runs.iter().map(|run| tp(run)).collect();
        let (set, refusals) = InstallSet::resolve(Path::new(DIR), &layout(files), &manifest, &runs);
        Ok((set, shown(refusals)))
    }

    #[test]
    fn exclude_drops_matching_files_from_install_and_render() -> TestResult {
        let (set, refusals) = resolve(
            &[
                "docs/x.md",
                "pack.toml",
                "scripts/a.lua",
                "scripts/a_test.lua",
                "skills/a/SKILL.md",
                "skills/a/scripts/b_test.lua",
            ],
            "exclude = [\"**/*_test.lua\", \"docs/**\"]",
            &[],
        )?;
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(set.is_excluded(&tp("scripts/a_test.lua")));
        assert!(set.is_excluded(&tp("skills/a/scripts/b_test.lua")));
        assert!(set.is_excluded(&tp("docs/x.md")));
        assert!(set.is_installed(&tp("scripts/a.lua")));
        assert!(!set.is_installed(&tp("scripts/a_test.lua")));
        assert!(!set.is_installed(&tp("skills/a/scripts/b_test.lua")));
        assert!(!set.is_rendered(&tp("docs/x.md")));
        Ok(())
    }

    #[test]
    fn rendered_set_holds_each_installed_md_and_each_render_file() -> TestResult {
        let (set, refusals) = resolve(
            &[
                "README.md",
                "agents/b.md",
                "enforcement.json",
                "native/claude/hooks.toml",
                "native/claude/x.md",
                "pack.toml",
                "scripts/a.lua",
                "skills/a/SKILL.md",
            ],
            "render = [\"enforcement.json\"]",
            &[],
        )?;
        assert!(refusals.is_empty(), "{refusals:?}");
        let rendered: Vec<&str> = set.rendered().map(TreePath::as_str).collect();
        assert_eq!(
            rendered,
            [
                "README.md",
                "agents/b.md",
                "enforcement.json",
                "native/claude/x.md",
                "skills/a/SKILL.md",
            ]
        );
        assert!(!set.is_installed(&tp("pack.toml")));
        assert!(!set.is_installed(&tp("native/claude/hooks.toml")));
        assert!(set.is_installed(&tp("scripts/a.lua")));
        Ok(())
    }

    #[test]
    fn render_entry_must_exist() -> TestResult {
        let (_, refusals) = resolve(&["pack.toml"], "render = [\"enforcment.json\"]", &[])?;
        assert_eq!(
            refusals,
            [format!(
                "{DIR}/pack.toml: render[0]: \"enforcment.json\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn render_entry_must_not_be_excluded() -> TestResult {
        let (_, refusals) = resolve(
            &["pack.toml", "x.json"],
            "render = [\"x.json\"]\nexclude = [\"*.json\"]",
            &[],
        )?;
        assert_eq!(
            refusals,
            [format!(
                "{DIR}/pack.toml: render[0]: \"x.json\" is also excluded"
            )]
        );
        Ok(())
    }

    #[test]
    fn render_entry_must_be_installed() -> TestResult {
        let (set, refusals) = resolve(
            &["native/claude/hooks.toml", "pack.toml", "x.json"],
            "render = [\"x.json\", \"pack.toml\", \"native/claude/hooks.toml\"]",
            &[],
        )?;
        assert_eq!(
            refusals,
            [
                format!("{DIR}/pack.toml: render[1]: \"pack.toml\" is never installed"),
                format!(
                    "{DIR}/pack.toml: render[2]: \"native/claude/hooks.toml\" is never installed"
                ),
            ]
        );
        assert!(!set.is_rendered(&tp("pack.toml")));
        assert!(set.is_rendered(&tp("x.json")));
        Ok(())
    }

    #[test]
    fn an_excluded_never_installed_render_entry_reports_only_the_exclusion() -> TestResult {
        let (_, refusals) = resolve(
            &["pack.toml"],
            "render = [\"pack.toml\"]\nexclude = [\"pack.toml\"]",
            &[],
        )?;
        assert_eq!(
            refusals,
            [format!(
                "{DIR}/pack.toml: render[0]: \"pack.toml\" is also excluded"
            )]
        );
        Ok(())
    }

    #[test]
    fn a_render_entry_the_walk_refused_is_not_reported_missing() -> TestResult {
        use crate::types::pack::testing::refused_walk;

        let walk = refused_walk(&["pack.toml"], &["data.json", "hooks"]);
        let (layout, _) =
            PackLayout::classify(Path::new(DIR), &walk).map_err(|refusal| refusal.to_string())?;
        let manifest = manifest_with(
            "render = [\"data.json\", \"hooks/x.json\", \"gone.json\"]",
            "",
        )?;
        let (_, refusals) = InstallSet::resolve(Path::new(DIR), &layout, &manifest, &[]);
        assert_eq!(
            shown(refusals),
            [format!(
                "{DIR}/pack.toml: render[2]: \"gone.json\" does not exist"
            )]
        );
        Ok(())
    }

    #[test]
    fn exclude_must_not_match_protected_files() -> TestResult {
        let cases: [(&[&str], &str, &[&str], &str); 4] = [
            (
                &["pack.toml", "skills/discuss/SKILL.md"],
                "exclude = [\"pack.toml\"]",
                &[],
                "pack.toml",
            ),
            (
                &["pack.toml", "skills/discuss/SKILL.md"],
                "exclude = [\"**/*.md\"]",
                &[],
                "skills/discuss/SKILL.md",
            ),
            (
                &["agents/coder.md", "pack.toml"],
                "exclude = [\"agents/*\"]",
                &[],
                "agents/coder.md",
            ),
            (
                &["hooks/enforce.lua", "pack.toml"],
                "exclude = [\"hooks/**\"]",
                &["hooks/enforce.lua"],
                "hooks/enforce.lua",
            ),
        ];
        for (files, top, runs, excluded) in cases {
            let glob = top
                .trim_start_matches("exclude = [\"")
                .trim_end_matches("\"]");
            let (_, refusals) = resolve(files, top, runs)?;
            assert_eq!(
                refusals,
                [format!(
                    "{DIR}/pack.toml: exclude[0]: \"{glob}\" excludes {excluded}"
                )]
            );
        }
        Ok(())
    }

    #[test]
    fn a_refusal_carries_the_index_of_its_own_list_entry() -> TestResult {
        let (_, refusals) = resolve(
            &["a.json", "b.json", "pack.toml"],
            "render = [\"a.json\", \"gone.json\", \"b.json\"]\nexclude = [\"*.xml\", \"b.json\", \"pack.toml\"]",
            &[],
        )?;
        assert_eq!(
            refusals,
            [
                format!("{DIR}/pack.toml: render[1]: \"gone.json\" does not exist"),
                format!("{DIR}/pack.toml: render[2]: \"b.json\" is also excluded"),
                format!("{DIR}/pack.toml: exclude[2]: \"pack.toml\" excludes pack.toml"),
            ]
        );
        Ok(())
    }

    #[test]
    fn exclude_matching_a_rendered_protected_file_reports_only_the_render_refusal() -> TestResult {
        let (_, refusals) = resolve(
            &["agents/coder.md", "pack.toml"],
            "render = [\"agents/coder.md\"]\nexclude = [\"agents/*\"]",
            &[],
        )?;
        assert_eq!(
            refusals,
            [format!(
                "{DIR}/pack.toml: render[0]: \"agents/coder.md\" is also excluded"
            )]
        );
        Ok(())
    }
}
