//! The file set of the scratch project an `ext test` run installs into.
//!
//! # Responsibilities
//!
//! - [`StagedFile`] is one file of that project: its path from the project root and its bytes.
//! - [`pack_name`] reads the name of a pack the author passed with `--pack`.
//! - [`stage()`] computes the whole project from the extension, the conformance pack and the
//!   author's packs: every file of each tree as the walk lists it, a `native/<tool>/note.md` in
//!   the conformance pack for the tool under test, and the text of `agentstacks.toml`. It refuses
//!   a `--pack` named like another `--pack` or like the conformance pack, and reports every
//!   refusal the walks raise.
//!
//! # Non-responsibilities
//!
//! Opening the trees and writing the files belong to the scratch port and its adapter. Whether
//! the packs are valid, or the extension covers them, is decided later by the plan, which reads
//! this project like any other.

use std::path::Path;

use crate::port::FileTree;
use crate::types::check::conformance::{CONFORMANCE, NAME};
use crate::types::extension::Extension;
use crate::types::name::{InvalidName, PackName, ToolName};
use crate::types::op::{OpPath, PathShape};
use crate::types::pack::{Pack, PackError};
use crate::types::project::{InvalidLocalPath, LocalPath, PROJECT_FILE, Source, render_project};
use crate::types::report::{AlreadyGiven, Diagnostic, Diagnostics, FormatError, Location};
use crate::types::shorthand::Flag;
use crate::types::tree::walk;

/// One file of the scratch project.
///
/// # Examples
///
/// ```
/// use agentstacks::types::check::StagedFile;
/// use agentstacks::types::op::OpPath;
///
/// let file = StagedFile { path: OpPath::new("ext/mytool/adapter.lua")?, bytes: b"return {}\n".to_vec() };
/// assert_eq!(file.path.as_str(), "ext/mytool/adapter.lua");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFile {
    /// Where the file goes, relative to the scratch project.
    pub path: OpPath,
    /// What the file holds.
    pub bytes: Vec<u8>,
}

/// A refusal of the scratch project's file set.
#[derive(Debug, thiserror::Error)]
pub enum StageError {
    /// A `--pack` is named like an earlier `--pack` or like the built-in conformance pack.
    #[error(transparent)]
    AlreadyGiven(#[from] AlreadyGiven),
    /// A path of the scratch project is not a valid project path.
    #[error(transparent)]
    Path(#[from] PathShape),
    /// A scratch project directory is not a valid local path.
    #[error(transparent)]
    Source(#[from] InvalidLocalPath),
    /// The conformance pack's name is not a valid pack name.
    #[error(transparent)]
    Name(#[from] InvalidName),
}

/// A directory a `--pack` gives, with the name its `pack.toml` carries.
#[derive(Debug, Clone)]
pub struct GivenPack<T> {
    /// The directory as the author wrote it after `--pack`.
    pub label: String,
    /// The name its `pack.toml` carries.
    pub name: PackName,
    /// The files of the directory.
    pub tree: T,
}

/// The name of the pack `tree` holds, which the author wrote as `label` after `--pack`.
///
/// # Errors
///
/// `--pack "<label>": not a pack (no pack.toml)` for a directory without `pack.toml`, otherwise
/// every refusal of [`Pack::load`], which includes an unreadable `pack.toml` and a missing or
/// invalid `pack.name`.
pub fn pack_name<T: FileTree>(tree: &T, label: &str) -> Result<PackName, Diagnostics> {
    match Pack::load(tree) {
        Ok(pack) => Ok(pack.manifest().name().clone()),
        Err(refusals) => {
            let mut flagged = Diagnostics::new();
            flagged.extend(
                refusals
                    .into_iter()
                    .map(|refusal| flag_not_a_pack(refusal, label)),
            );
            Err(flagged)
        }
    }
}

/// The place of a `--pack` the author wrote as `label`.
fn pack_place(label: &str) -> Location {
    Location::verbatim(pack_option(label))
}

/// A `--pack` the author wrote as `label`, as the refusals show it.
fn pack_option(label: &str) -> String {
    format!("{} \"{label}\"", Flag::Pack)
}

/// `refusal` with the `--pack` the author wrote as its place, when it says the directory is not a
/// pack.
fn flag_not_a_pack(refusal: Diagnostic, label: &str) -> Diagnostic {
    if matches!(refusal.error(), FormatError::Pack(PackError::NotAPack)) {
        Diagnostic::new(pack_place(label), PackError::NotAPack)
    } else {
        refusal
    }
}

/// The files of the scratch project, sorted by path.
///
/// `extension`'s tree goes under `ext/<name>/`, the conformance pack under `packs/conformance/`
/// with a native note for the tool, each of `packs` under `packs/<name>/`, and `agentstacks.toml`
/// lists them all.
///
/// # Errors
///
/// A `--pack` named `conformance`, two `--pack` with one name, a symlink or special file or
/// unreadable entry in a tree, and a path that is not a valid project path; every one found.
pub fn stage<T: FileTree, P: FileTree>(
    extension: &Extension,
    extension_tree: &T,
    packs: &[GivenPack<P>],
) -> Result<Vec<StagedFile>, Diagnostics> {
    let tool = extension.manifest().name();
    let mut staging = Staging::default();
    staging.check_names(packs);

    staging.tree(extension_tree, &format!("ext/{tool}"));
    for (path, bytes) in CONFORMANCE {
        staging.file(&format!("packs/{NAME}/{path}"), bytes.to_vec());
    }
    staging.file(
        &format!("packs/{NAME}/native/{tool}/note.md"),
        format!("Native note for {tool}.\n").into_bytes(),
    );
    for pack in packs {
        staging.tree(&pack.tree, &format!("packs/{}", pack.name));
    }
    staging.project(tool, packs);
    staging.finish()
}

/// The files and the refusals found so far.
#[derive(Default)]
struct Staging {
    files: Vec<StagedFile>,
    refusals: Diagnostics,
}

impl Staging {
    /// Refuses a `--pack` that is named like the conformance pack or like an earlier `--pack`.
    fn check_names<T>(&mut self, packs: &[GivenPack<T>]) {
        let mut seen: Vec<(&PackName, &str)> = Vec::new();
        for GivenPack { label, name, .. } in packs {
            let first = if name.as_str() == NAME {
                Some("the built-in conformance pack".to_owned())
            } else {
                seen.iter()
                    .find(|(earlier, _)| *earlier == name)
                    .map(|(_, first)| pack_option(first))
            };
            match first {
                Some(first) => self.refusals.push(Diagnostic::new(
                    pack_place(label),
                    StageError::from(AlreadyGiven::new("pack", name.as_str(), first)),
                )),
                None => seen.push((name, label)),
            }
        }
    }

    /// Adds `bytes` at `path`, or refuses a path that is not a project path.
    fn file(&mut self, path: &str, bytes: Vec<u8>) {
        match OpPath::new(path) {
            Ok(path) => self.files.push(StagedFile { path, bytes }),
            Err(shape) => self.refusals.push(Diagnostic::new(
                Location::verbatim(path),
                StageError::Path(shape),
            )),
        }
    }

    /// Adds every file the walk lists in `tree` under `prefix`.
    fn tree<T: FileTree>(&mut self, tree: &T, prefix: &str) {
        let (walked, walk_refusals) = walk(tree);
        self.refusals.extend(walk_refusals);
        for path in walked.files() {
            let relative = Path::new(path.as_str());
            match tree.read_file(relative) {
                Ok(bytes) => self.file(&format!("{prefix}/{path}"), bytes),
                Err(error) => self.refusals.push(Diagnostic::new(
                    Location::file(tree.root().join(relative)),
                    error,
                )),
            }
        }
    }

    /// Adds `agentstacks.toml`, listing the tool, the conformance pack and each given pack.
    fn project<T>(&mut self, tool: &ToolName, packs: &[GivenPack<T>]) {
        match project_text(tool, packs) {
            Ok(text) => self.file(PROJECT_FILE, text.into_bytes()),
            Err(error) => self
                .refusals
                .push(Diagnostic::new(Location::file(PROJECT_FILE), error)),
        }
    }

    fn finish(mut self) -> Result<Vec<StagedFile>, Diagnostics> {
        self.files.sort_by(|a, b| a.path.cmp(&b.path));
        self.refusals.into_result(self.files)
    }
}

/// The text of the scratch project's `agentstacks.toml`.
fn project_text<T>(tool: &ToolName, packs: &[GivenPack<T>]) -> Result<String, StageError> {
    let source = |dir: &str| LocalPath::new(dir).map(Source::Path);
    let tools = [(tool.clone(), source(&format!("ext/{tool}"))?)];
    let mut listed = vec![(PackName::new(NAME)?, source(&format!("packs/{NAME}"))?)];
    for pack in packs {
        listed.push((pack.name.clone(), source(&format!("packs/{}", pack.name))?));
    }
    Ok(render_project(&tools, &listed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const MANIFEST: &str = "[extension]\n\
        name        = \"mytool\"\n\
        version     = \"0.1.0\"\n\
        api         = 1\n\
        description = \"A tool\"\n\
        entry       = \"adapter.lua\"\n";

    const PACK_TOML: &str = "[pack]\n\
        name        = \"team\"\n\
        version     = \"0.1.0\"\n\
        api         = 1\n\
        description = \"A team pack\"\n";

    const SKILL: &str = "---\nname: a\ndescription: d\n---\n\nbody\n";

    fn ext_tree() -> FakeFileTree {
        FakeFileTree::new("ext-src")
            .file("extension.toml", MANIFEST.as_bytes())
            .file("adapter.lua", b"return {}\n")
            .file(".git/config", b"hidden\n")
    }

    fn pack_tree(root: &str) -> FakeFileTree {
        FakeFileTree::new(root)
            .file("pack.toml", PACK_TOML.as_bytes())
            .file("skills/a/SKILL.md", SKILL.as_bytes())
    }

    fn extension(tree: &FakeFileTree) -> Result<Extension, Box<dyn std::error::Error>> {
        Ok(Extension::load(tree).map_err(|refusals| format!("{refusals:?}"))?)
    }

    fn given(label: &str, name: &str, root: &str) -> Result<GivenPack<FakeFileTree>, InvalidName> {
        Ok(GivenPack {
            label: label.to_owned(),
            name: PackName::new(name)?,
            tree: pack_tree(root),
        })
    }

    fn shown(refusals: Diagnostics) -> Vec<String> {
        refusals
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn staged_paths(files: &[StagedFile]) -> Vec<&str> {
        files.iter().map(|file| file.path.as_str()).collect()
    }

    fn staged(files: &[StagedFile], path: &str) -> Option<String> {
        files
            .iter()
            .find(|file| file.path.as_str() == path)
            .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
    }

    #[test]
    fn stage_lists_every_file_and_project_text() -> TestResult {
        let tree = ext_tree();
        let files = stage(
            &extension(&tree)?,
            &tree,
            &[given("../team", "team", "team-src")?],
        )
        .map_err(|refusals| format!("{refusals:?}"))?;

        let mut expected: Vec<String> = vec![
            "agentstacks.toml".to_owned(),
            "ext/mytool/adapter.lua".to_owned(),
            "ext/mytool/extension.toml".to_owned(),
            "packs/conformance/native/mytool/note.md".to_owned(),
            "packs/team/pack.toml".to_owned(),
            "packs/team/skills/a/SKILL.md".to_owned(),
        ];
        expected.extend(
            CONFORMANCE
                .iter()
                .map(|(path, _)| format!("packs/conformance/{path}")),
        );
        expected.sort();
        assert_eq!(staged_paths(&files), expected);

        assert_eq!(
            staged(&files, "agentstacks.toml").as_deref(),
            Some(
                "api = 1\n\n[tools.mytool]\npath = \"ext/mytool\"\n\n\
                 [packs.conformance]\npath = \"packs/conformance\"\n\n\
                 [packs.team]\npath = \"packs/team\"\n"
            )
        );
        assert_eq!(
            staged(&files, "ext/mytool/adapter.lua").as_deref(),
            Some("return {}\n")
        );
        assert_eq!(
            staged(&files, "packs/team/skills/a/SKILL.md").as_deref(),
            Some(SKILL)
        );
        Ok(())
    }

    #[test]
    fn stage_without_packs_lists_the_extension_and_the_conformance_pack() -> TestResult {
        let tree = ext_tree();
        let files = stage::<_, FakeFileTree>(&extension(&tree)?, &tree, &[])
            .map_err(|refusals| format!("{refusals:?}"))?;
        assert!(staged_paths(&files).contains(&"packs/conformance/pack.toml"));
        assert!(
            staged_paths(&files)
                .iter()
                .all(|path| !path.starts_with("packs/team"))
        );
        assert_eq!(
            staged(&files, "agentstacks.toml").as_deref(),
            Some(
                "api = 1\n\n[tools.mytool]\npath = \"ext/mytool\"\n\n\
                 [packs.conformance]\npath = \"packs/conformance\"\n"
            )
        );
        Ok(())
    }

    #[test]
    fn native_note_for_the_tool() -> TestResult {
        let tree = ext_tree();
        let files = stage::<_, FakeFileTree>(&extension(&tree)?, &tree, &[])
            .map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(
            staged(&files, "packs/conformance/native/mytool/note.md").as_deref(),
            Some("Native note for mytool.\n")
        );
        Ok(())
    }

    #[test]
    fn duplicate_pack_names() -> TestResult {
        let tree = ext_tree();
        let packs = [given("../a", "x", "a-src")?, given("../b", "x", "b-src")?];
        let refusals = stage(&extension(&tree)?, &tree, &packs)
            .err()
            .ok_or("a duplicate name must be refused")?;
        assert_eq!(
            shown(refusals),
            ["--pack \"../b\": pack \"x\" is already given by --pack \"../a\""]
        );
        Ok(())
    }

    #[test]
    fn conformance_name_reserved() -> TestResult {
        let tree = ext_tree();
        let packs = [given("./mine", "conformance", "mine-src")?];
        let refusals = stage(&extension(&tree)?, &tree, &packs)
            .err()
            .ok_or("the reserved name must be refused")?;
        assert_eq!(
            shown(refusals),
            [
                "--pack \"./mine\": pack \"conformance\" is already given by the built-in conformance pack"
            ]
        );
        Ok(())
    }

    #[test]
    fn walk_refusal_is_staging_refusal() -> TestResult {
        let tree = ext_tree();
        let linked = pack_tree("../team").symlink("skills/link");
        let packs = [GivenPack {
            label: "../team".to_owned(),
            name: PackName::new("team")?,
            tree: linked,
        }];
        let refusals = stage(&extension(&tree)?, &tree, &packs)
            .err()
            .ok_or("a symlink must be refused")?;
        assert_eq!(
            shown(refusals),
            ["../team/skills/link: symlinks are not allowed in a pack"]
        );
        Ok(())
    }

    #[test]
    fn unreadable_file_is_staging_refusal() -> TestResult {
        let tree = ext_tree();
        let packs = [GivenPack {
            label: "../team".to_owned(),
            name: PackName::new("team")?,
            tree: pack_tree("../team").unreadable("notes.md"),
        }];
        let refusals = stage(&extension(&tree)?, &tree, &packs)
            .err()
            .ok_or("an unreadable file must be refused")?;
        assert_eq!(
            shown(refusals),
            [format!("../team/notes.md: {}", FakeFileTree::denied())]
        );
        Ok(())
    }

    #[test]
    fn pack_name_reads_the_manifest_name() -> TestResult {
        let name = pack_name(&pack_tree("../team"), "../team")
            .map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(name.as_str(), "team");
        Ok(())
    }

    #[test]
    fn pack_name_without_pack_toml() -> TestResult {
        let tree = FakeFileTree::new("../empty").file("skills/a/SKILL.md", b"x\n");
        let refusals = pack_name(&tree, "../empty")
            .err()
            .ok_or("a directory without pack.toml must be refused")?;
        assert_eq!(
            shown(refusals),
            ["--pack \"../empty\": not a pack (no pack.toml)"]
        );
        Ok(())
    }

    #[test]
    fn pack_name_with_invalid_name() -> TestResult {
        let bad = PACK_TOML.replace("\"team\"", "\"Not Valid\"");
        let tree = pack_tree("../bad").file("pack.toml", bad.as_bytes());
        let refusals = pack_name(&tree, "../bad")
            .err()
            .ok_or("an invalid name must be refused")?;
        let own = Pack::load(&tree)
            .err()
            .ok_or("Pack::load must refuse it too")?;
        let shown = shown(refusals);
        assert_eq!(shown, self::shown(own));
        assert!(
            shown
                .iter()
                .any(|line| line.starts_with("../bad/pack.toml: pack.name: ")),
            "{shown:?}"
        );
        Ok(())
    }
}
