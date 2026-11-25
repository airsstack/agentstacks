//! The project directory on the real disk, behind the [`Workspace`] port.
//!
//! # Responsibilities
//!
//! - [`StdWorkspace`] keeps the canonical project root and answers every [`Workspace`] method by
//!   resolving the path (`resolve`), reading its state (`state`), and, for the writes, staging
//!   (`stage`) and installing (`swap`). It decides nothing about a step: the rules in `types` take
//!   the [`Resolution`] it returns.
//! - A write re-resolves its path when it runs, so a path that now leads outside the project is
//!   refused with an error displayed as `resolves outside the project`, and nothing outside the
//!   project is written. The resolution and the write are separate system calls, so a link swapped
//!   in between them is not prevented.
//!
//! # Non-responsibilities
//!
//! What to remove and what to lock after a failed write belongs to apply; this adapter hands back
//! the platform's error.

use std::io;
use std::path::{Path, PathBuf};

use crate::fs::disk::StdFileTree;
use crate::fs::workspace::resolve::{Located, link_location, locate, relative_path};
use crate::fs::workspace::stage::{discard_temp, stage_bytes, temp_of};
use crate::fs::workspace::state::disk_state;
use crate::fs::workspace::swap::{delete_file, swap_onto, write_lock, write_project};
use crate::port::Workspace;
use crate::types::op::OpPath;
use crate::types::plan::{DiskState, Link, Resolution};
use crate::types::project::{LocalPath, PROJECT_FILE};
use crate::types::tree::utf8_text;

/// The project file that pins what was applied.
const LOCK_FILE: &str = "agentstacks.lock";

/// A project directory on the real disk.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdWorkspace;
/// use agentstacks::port::Workspace;
///
/// let dir = tempfile::tempdir()?;
/// std::fs::write(dir.path().join("agentstacks.toml"), "tools = []\n")?;
///
/// let workspace = StdWorkspace::open(dir.path())?;
/// assert_eq!(workspace.project_text()?, "tools = []\n");
/// assert_eq!(workspace.lock_text()?, None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct StdWorkspace {
    root: PathBuf,
}

impl StdWorkspace {
    /// The workspace of the project directory `root`, canonicalised once.
    ///
    /// # Errors
    ///
    /// The platform's error when `root` cannot be canonicalised, such as a directory that does
    /// not exist.
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            root: root.as_ref().canonicalize()?,
        })
    }

    /// The canonical project directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The error for a write to a path that leads outside the project.
    fn outside() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, "resolves outside the project")
    }

    /// The canonical target `path` leads to, refusing one outside the project, one that is the
    /// project directory itself, or one that is a symlink to nowhere.
    fn target_of(&self, path: &OpPath) -> io::Result<PathBuf> {
        match locate(&self.root, &relative_path(path.as_str()))? {
            Located::Inside { target, .. } if target == self.root => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resolves to the project directory",
            )),
            Located::Inside { target, .. } => Ok(target),
            Located::Outside { .. } => Err(Self::outside()),
            Located::Dangling { .. } => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "symlink leads nowhere",
            )),
        }
    }

    /// The text of the project file `name`, or `None` when it does not exist.
    fn read_project_file(&self, name: &str) -> io::Result<Option<String>> {
        match std::fs::read(self.root.join(name)) {
            Ok(bytes) => utf8_text(bytes).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl Workspace for StdWorkspace {
    type Tree = StdFileTree;

    fn project_text(&self) -> io::Result<String> {
        utf8_text(std::fs::read(self.root.join(PROJECT_FILE))?)
    }

    fn lock_text(&self) -> io::Result<Option<String>> {
        self.read_project_file(LOCK_FILE)
    }

    fn resolve(&self, path: &OpPath) -> io::Result<Resolution> {
        Ok(match locate(&self.root, &relative_path(path.as_str()))? {
            Located::Outside { link } => Resolution::Outside { link },
            Located::Dangling { .. } => Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::NotRegular,
            },
            Located::Inside { link, target } => Resolution::Inside {
                link,
                state: disk_state(&target)?,
            },
        })
    }

    fn source_tree(&self, dir: &LocalPath) -> io::Result<Option<Self::Tree>> {
        let relative = relative_path(dir.as_str());
        Ok(match locate(&self.root, &relative)? {
            Located::Outside { .. } => None,
            Located::Inside { target, .. } | Located::Dangling { at: target } => {
                Some(StdFileTree::labelled(target, dir.as_str()))
            }
        })
    }

    fn stage(&self, path: &OpPath, bytes: &[u8]) -> io::Result<()> {
        stage_bytes(&temp_of(&self.target_of(path)?), bytes)
    }

    fn discard(&self, path: &OpPath) -> io::Result<()> {
        match locate(&self.root, &relative_path(path.as_str()))? {
            Located::Inside { target, .. } if target != self.root => {
                discard_temp(&temp_of(&target))
            }
            Located::Inside { .. } | Located::Outside { .. } | Located::Dangling { .. } => Ok(()),
        }
    }

    fn swap(&self, path: &OpPath) -> io::Result<()> {
        let target = self.target_of(path)?;
        swap_onto(&temp_of(&target), &target)
    }

    fn delete(&self, path: &OpPath) -> io::Result<()> {
        let relative = relative_path(path.as_str());
        match locate(&self.root, &relative)? {
            Located::Inside {
                link: Link::Symlink,
                ..
            } => delete_file(&link_location(&self.root, &relative)?),
            Located::Inside { target, .. } => delete_file(&target),
            Located::Dangling { at } => delete_file(&at),
            Located::Outside { .. } => Err(Self::outside()),
        }
    }

    fn write_project(&self, text: &str) -> io::Result<()> {
        write_project(&self.root.join(PROJECT_FILE), text)
    }

    fn write_lock(&self, text: &str) -> io::Result<()> {
        write_lock(&self.root.join(LOCK_FILE), text)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use super::*;
    use crate::port::{FakeWorkspace, FileTree};
    use crate::types::pack::Pack;
    use crate::types::tree::ContentHash;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A temp project and the workspace over it.
    fn project() -> Result<(tempfile::TempDir, StdWorkspace), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let workspace = StdWorkspace::open(dir.path())?;
        Ok((dir, workspace))
    }

    fn p(text: &str) -> Result<OpPath, Box<dyn std::error::Error>> {
        Ok(OpPath::new(text)?)
    }

    fn mode_of(path: &Path) -> io::Result<u32> {
        Ok(std::fs::metadata(path)?.permissions().mode() & 0o7777)
    }

    #[test]
    fn open_refuses_a_missing_directory() -> TestResult {
        let dir = tempfile::tempdir()?;
        let error = StdWorkspace::open(dir.path().join("none"))
            .err()
            .ok_or("expected a failure")?;
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn a_workspace_opened_through_a_symlink_resolves_inside() -> TestResult {
        let (dir, _) = project()?;
        std::fs::write(dir.path().join("a.md"), b"x")?;
        let holder = tempfile::tempdir()?;
        symlink(dir.path(), holder.path().join("project"))?;
        let workspace = StdWorkspace::open(holder.path().join("project"))?;
        assert!(matches!(
            workspace.resolve(&p("a.md")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::File(_)
            }
        ));
        Ok(())
    }

    #[test]
    fn resolve_reports_absent() -> TestResult {
        let (_dir, ws) = project()?;
        assert_eq!(
            ws.resolve(&p("none.md")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::Absent
            }
        );
        Ok(())
    }

    #[test]
    fn resolve_reports_a_file_with_hash_and_mode() -> TestResult {
        let (dir, ws) = project()?;
        let file = dir.path().join("a.sh");
        std::fs::write(&file, b"a\n")?;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
        let Resolution::Inside {
            link: Link::Direct,
            state: DiskState::File(found),
        } = ws.resolve(&p("a.sh")?)?
        else {
            return Err("expected an inside regular file".into());
        };
        assert_eq!(found.hash(), &ContentHash::of_bytes(b"a\n"));
        assert_eq!(found.mode(), 0o755);
        Ok(())
    }

    #[test]
    fn resolve_reports_a_directory_as_not_regular() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::create_dir(dir.path().join("d"))?;
        assert_eq!(
            ws.resolve(&p("d")?)?,
            Resolution::Inside {
                link: Link::Direct,
                state: DiskState::NotRegular
            }
        );
        Ok(())
    }

    #[test]
    fn resolve_reports_a_dangling_link_as_not_regular() -> TestResult {
        let (dir, ws) = project()?;
        symlink(dir.path().join("missing"), dir.path().join("gone"))?;
        assert_eq!(
            ws.resolve(&p("gone")?)?,
            Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::NotRegular
            }
        );
        Ok(())
    }

    #[test]
    fn resolve_reads_through_an_inside_link() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::write(dir.path().join("real.md"), b"real")?;
        symlink("real.md", dir.path().join("link.md"))?;
        let Resolution::Inside {
            link: Link::Symlink,
            state: DiskState::File(found),
        } = ws.resolve(&p("link.md")?)?
        else {
            return Err("expected a symlink to an inside regular file".into());
        };
        assert_eq!(found.bytes(), b"real");
        Ok(())
    }

    #[test]
    fn resolve_marks_a_file_replaced_by_a_symlink() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::write(dir.path().join("other.md"), b"other")?;
        std::fs::write(dir.path().join("locked.md"), b"locked")?;
        std::fs::remove_file(dir.path().join("locked.md"))?;
        symlink("other.md", dir.path().join("locked.md"))?;
        assert_eq!(ws.resolve(&p("locked.md")?)?.link(), Link::Symlink);
        Ok(())
    }

    #[test]
    fn resolve_refuses_an_escape() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        std::fs::write(outside.path().join("hosts"), b"x")?;
        symlink(outside.path().join("hosts"), dir.path().join("link"))?;
        assert_eq!(
            ws.resolve(&p("link")?)?,
            Resolution::Outside {
                link: Link::Symlink
            }
        );
        Ok(())
    }

    #[test]
    fn lock_text_is_none_without_a_lock() -> TestResult {
        let (dir, ws) = project()?;
        assert_eq!(ws.lock_text()?, None);
        std::fs::write(dir.path().join("agentstacks.lock"), "lock = 1\n")?;
        assert_eq!(ws.lock_text()?.as_deref(), Some("lock = 1\n"));
        Ok(())
    }

    #[test]
    fn project_text_reads_the_file_and_reports_a_missing_one() -> TestResult {
        let (dir, ws) = project()?;
        let missing = ws.project_text().err().ok_or("expected a failure")?;
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert_eq!(
            missing.to_string(),
            "No such file or directory (os error 2)"
        );
        std::fs::write(dir.path().join("agentstacks.toml"), "tools = []\n")?;
        assert_eq!(ws.project_text()?, "tools = []\n");
        Ok(())
    }

    #[test]
    fn project_text_refuses_bytes_that_are_not_utf8() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::write(dir.path().join("agentstacks.toml"), [0x66, 0xFF])?;
        let error = ws.project_text().err().ok_or("expected a failure")?;
        assert_eq!(error.to_string(), "not valid UTF-8");
        Ok(())
    }

    #[test]
    fn source_tree_inside_reads_the_directory() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::create_dir_all(dir.path().join("packs/core"))?;
        std::fs::write(dir.path().join("packs/core/pack.toml"), "name = \"core\"\n")?;
        let tree = ws
            .source_tree(&LocalPath::new("packs/core")?)?
            .ok_or("expected a tree")?;
        assert_eq!(tree.read_text(Path::new("pack.toml"))?, "name = \"core\"\n");
        assert_eq!(tree.root(), Path::new("packs/core"));

        let outside = tempfile::tempdir()?;
        symlink(outside.path(), dir.path().join("away"))?;
        assert!(ws.source_tree(&LocalPath::new("away")?)?.is_none());
        Ok(())
    }

    #[test]
    fn path_source_refusal_names_the_path_as_written() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::create_dir_all(dir.path().join("packs/core"))?;
        std::fs::write(dir.path().join("packs/core/pack.toml"), "[pack]\n")?;
        let tree = ws
            .source_tree(&LocalPath::new("packs/core")?)?
            .ok_or("expected a tree")?;
        let refusals = Pack::load(&tree).err().ok_or("expected a refusal")?;
        let shown: Vec<String> = refusals
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(!shown.is_empty());
        assert!(
            shown
                .iter()
                .all(|line| line.starts_with("packs/core/pack.toml:")),
            "{shown:?}"
        );
        let tempdir = dir.path().display().to_string();
        let canonical = ws.root().display().to_string();
        assert!(
            shown
                .iter()
                .all(|line| !line.contains(&tempdir) && !line.contains(&canonical)),
            "{shown:?}"
        );
        Ok(())
    }

    #[test]
    fn stage_through_an_inside_link_writes_beside_the_link_target() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::create_dir(dir.path().join("real"))?;
        std::fs::write(dir.path().join("real/x.md"), b"old")?;
        symlink("real/x.md", dir.path().join("link.md"))?;
        ws.stage(&p("link.md")?, b"new")?;
        assert_eq!(
            std::fs::read(dir.path().join("real/.x.md.agentstacks-tmp"))?,
            b"new"
        );
        assert!(!dir.path().join(".link.md.agentstacks-tmp").exists());
        Ok(())
    }

    #[test]
    fn stage_refuses_a_path_that_now_resolves_outside() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        symlink(outside.path(), dir.path().join("d"))?;
        let error = ws
            .stage(&p("d/x.md")?, b"x")
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(error.to_string(), "resolves outside the project");
        assert_eq!(std::fs::read_dir(outside.path())?.count(), 0);
        Ok(())
    }

    #[test]
    fn swap_writes_through_an_inside_symlink() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::write(dir.path().join("real.md"), b"old")?;
        symlink("real.md", dir.path().join("link.md"))?;
        ws.stage(&p("link.md")?, b"B")?;
        ws.swap(&p("link.md")?)?;
        assert!(
            std::fs::symlink_metadata(dir.path().join("link.md"))?
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(dir.path().join("real.md"))?, b"B");
        Ok(())
    }

    #[test]
    fn swap_refuses_a_path_that_now_resolves_outside() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        symlink(outside.path(), dir.path().join("d"))?;
        let error = ws.swap(&p("d/x.md")?).err().ok_or("expected a refusal")?;
        assert_eq!(error.to_string(), "resolves outside the project");
        Ok(())
    }

    #[test]
    fn discard_of_an_unstaged_path_is_not_an_error() -> TestResult {
        let (_dir, ws) = project()?;
        ws.discard(&p("a/b.md")?)?;
        Ok(())
    }

    #[test]
    fn delete_refuses_a_path_that_now_resolves_outside() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        std::fs::write(outside.path().join("x.md"), b"keep")?;
        symlink(outside.path(), dir.path().join("d"))?;
        let error = ws.delete(&p("d/x.md")?).err().ok_or("expected a refusal")?;
        assert_eq!(error.to_string(), "resolves outside the project");
        assert!(outside.path().join("x.md").exists());
        Ok(())
    }

    #[test]
    fn source_tree_refuses_a_parent_component_that_climbs_out_through_a_link() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        std::fs::create_dir(outside.path().join("sub"))?;
        std::fs::create_dir(outside.path().join("packs"))?;
        std::fs::write(
            outside.path().join("packs/pack.toml"),
            "name = \"OUTSIDE\"\n",
        )?;
        symlink(outside.path().join("sub"), dir.path().join("d"))?;
        assert!(ws.source_tree(&LocalPath::new("d/../packs")?)?.is_none());
        assert!(ws.source_tree(&LocalPath::new("d/..")?)?.is_none());
        Ok(())
    }

    #[test]
    fn source_tree_reads_a_dangling_link_as_an_empty_place() -> TestResult {
        let (dir, ws) = project()?;
        symlink(dir.path().join("missing"), dir.path().join("gone"))?;
        let tree = ws
            .source_tree(&LocalPath::new("gone")?)?
            .ok_or("expected a tree")?;
        let error = tree
            .read_text(Path::new("pack.toml"))
            .err()
            .ok_or("expected a failure")?;
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn a_link_to_the_project_root_is_never_staged_swapped_or_discarded() -> TestResult {
        let holder = tempfile::tempdir()?;
        std::fs::create_dir(holder.path().join("project"))?;
        let ws = StdWorkspace::open(holder.path().join("project"))?;
        symlink(".", ws.root().join("x.md"))?;
        assert_eq!(
            ws.resolve(&p("x.md")?)?,
            Resolution::Inside {
                link: Link::Symlink,
                state: DiskState::NotRegular
            }
        );
        let stage = ws
            .stage(&p("x.md")?, b"x")
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(stage.kind(), io::ErrorKind::InvalidInput);
        assert!(ws.swap(&p("x.md")?).is_err());
        ws.discard(&p("x.md")?)?;
        let names: Vec<_> = std::fs::read_dir(holder.path())?
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<io::Result<_>>()?;
        assert_eq!(names, vec![std::ffi::OsString::from("project")]);
        Ok(())
    }

    #[test]
    fn a_dangling_link_is_refused_by_stage_and_swap_with_not_found() -> TestResult {
        let (dir, ws) = project()?;
        symlink(dir.path().join("missing"), dir.path().join("gone"))?;
        let stage = ws
            .stage(&p("gone")?, b"x")
            .err()
            .ok_or("expected a refusal")?;
        assert_eq!(stage.kind(), io::ErrorKind::NotFound);
        assert_eq!(stage.to_string(), "symlink leads nowhere");
        let swap = ws.swap(&p("gone")?).err().ok_or("expected a refusal")?;
        assert_eq!(swap.kind(), io::ErrorKind::NotFound);
        assert!(!dir.path().join("missing").exists());
        Ok(())
    }

    #[test]
    fn delete_removes_a_link_and_never_its_target() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::write(dir.path().join("user.md"), b"keep")?;
        symlink("user.md", dir.path().join("managed.md"))?;
        ws.delete(&p("managed.md")?)?;
        assert_eq!(std::fs::read(dir.path().join("user.md"))?, b"keep");
        assert!(std::fs::symlink_metadata(dir.path().join("managed.md")).is_err());
        Ok(())
    }

    #[test]
    fn delete_removes_a_dangling_link() -> TestResult {
        let (dir, ws) = project()?;
        symlink(dir.path().join("missing"), dir.path().join("gone"))?;
        ws.delete(&p("gone")?)?;
        assert!(std::fs::symlink_metadata(dir.path().join("gone")).is_err());
        Ok(())
    }

    #[test]
    fn delete_refuses_a_link_that_now_resolves_outside_by_its_directory() -> TestResult {
        let (dir, ws) = project()?;
        let outside = tempfile::tempdir()?;
        std::fs::write(outside.path().join("x.md"), b"keep")?;
        symlink("x.md", outside.path().join("l.md"))?;
        symlink(outside.path(), dir.path().join("d"))?;
        let error = ws.delete(&p("d/l.md")?).err().ok_or("expected a refusal")?;
        assert_eq!(error.to_string(), "resolves outside the project");
        assert!(outside.path().join("l.md").exists());
        Ok(())
    }

    #[test]
    fn write_project_writes_the_project_through_the_workspace() -> TestResult {
        let (dir, ws) = project()?;
        ws.write_project("tools = []\n")?;
        assert_eq!(ws.project_text()?, "tools = []\n");
        assert!(
            !dir.path()
                .join(".agentstacks.toml.agentstacks-tmp")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn write_lock_writes_the_lock_through_the_workspace() -> TestResult {
        let (dir, ws) = project()?;
        ws.write_lock("a = 1\n")?;
        assert_eq!(ws.lock_text()?.as_deref(), Some("a = 1\n"));
        assert!(
            !dir.path()
                .join(".agentstacks.lock.agentstacks-tmp")
                .exists()
        );
        Ok(())
    }

    #[test]
    fn staged_then_swapped_matches_the_fake() -> TestResult {
        let (dir, ws) = project()?;
        std::fs::create_dir(dir.path().join(".claude"))?;
        let old = dir.path().join(".claude/old");
        std::fs::write(&old, b"old")?;
        std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o755))?;
        let fake = FakeWorkspace::new().file_with_mode(&p(".claude/old")?, b"old", 0o755);

        ws.stage(&p(".claude/old")?, b"new")?;
        ws.stage(&p(".claude/fresh")?, b"fresh")?;
        ws.swap(&p(".claude/old")?)?;
        ws.swap(&p(".claude/fresh")?)?;
        fake.stage(&p(".claude/old")?, b"new")?;
        fake.stage(&p(".claude/fresh")?, b"fresh")?;
        fake.swap(&p(".claude/old")?)?;
        fake.swap(&p(".claude/fresh")?)?;

        for name in [".claude/old", ".claude/fresh"] {
            let on_disk = dir.path().join(name);
            assert_eq!(Some(std::fs::read(&on_disk)?), fake.file_bytes(&p(name)?));
        }
        assert_eq!(mode_of(&old)?, 0o755);
        assert_eq!(fake.mode(&p(".claude/old")?), Some(0o755));
        let fresh = mode_of(&dir.path().join(".claude/fresh"))?;
        let plain = dir.path().join("plain");
        std::fs::write(&plain, b"x")?;
        assert_eq!(fresh, mode_of(&plain)?);
        assert_eq!(
            fake.mode(&p(".claude/fresh")?),
            Some(FakeWorkspace::NEW_MODE)
        );
        assert!(fake.temps().is_empty());
        assert!(!dir.path().join(".claude/.old.agentstacks-tmp").exists());
        Ok(())
    }
}
