//! Where a project-relative path leads on the real disk.
//!
//! # Responsibilities
//!
//! - [`relative_path`] turns a normalised project path's text into a [`PathBuf`] that holds no `.`
//!   component and no empty one.
//! - [`locate`] answers the one inside rule: a path that is itself a symlink is followed, any other
//!   path is placed through its deepest ancestor that exists (the filesystem resolves every `..`
//!   there; only the missing suffix is applied by name), and the result must lie below the
//!   canonical project root. A symlink sitting in a directory outside the root is outside too.
//!   Paths compare by their bytes; nothing is case-folded.
//! - [`link_location`] is where a symlink itself sits, for an operation on the link and not on
//!   what it leads to.
//!
//! # Non-responsibilities
//!
//! What is at the target, and every write, belong to the sibling modules. The lookup and a later
//! use of its answer are separate system calls, so a link swapped in between is not caught here.

use std::io;
use std::path::{Component, Path, PathBuf};

use crate::types::plan::Link;

/// Where a path leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Located {
    /// The canonical target is below the project root.
    Inside {
        /// Whether the path itself is a symlink.
        link: Link,
        /// The canonical target, which may not exist yet.
        target: PathBuf,
    },
    /// The canonical target is not below the project root.
    Outside {
        /// Whether the path itself is a symlink.
        link: Link,
    },
    /// The path is a symlink, at the canonical location `at` below the project root, whose target
    /// does not exist.
    Dangling {
        /// The canonical location of the symlink itself.
        at: PathBuf,
    },
}

/// The components of the `/`-separated `text` that are neither empty nor `.`, as a relative path.
pub(super) fn relative_path(text: &str) -> PathBuf {
    text.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect()
}

/// `base` with each `..` of `parts` removing the component before it, by name.
fn lexical(base: PathBuf, parts: &[Component<'_>]) -> PathBuf {
    let mut path = base;
    for part in parts {
        match part {
            Component::ParentDir => {
                path.pop();
            }
            other => path.push(other),
        }
    }
    path
}

/// The canonical form of `root` joined with `relative`.
///
/// The longest prefix of `relative` that exists is canonicalised by the filesystem, so a `..` in
/// it climbs out of the real directory a symlink leads to. Only the components below that prefix,
/// which do not exist, are applied by name.
fn through_deepest_ancestor(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let parts: Vec<Component<'_>> = relative.components().collect();
    for kept in (0..=parts.len()).rev() {
        let candidate = root.join(parts[..kept].iter().collect::<PathBuf>());
        match candidate.canonicalize() {
            Ok(canonical) => return Ok(lexical(canonical, &parts[kept..])),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::from(io::ErrorKind::NotFound))
}

/// Where the symlink `relative` below the canonical `root` sits: its canonical parent directory
/// joined with its own name.
///
/// # Errors
///
/// The platform's error from canonicalising the parent directory.
pub(super) fn link_location(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let full = root.join(relative);
    let parent = full.parent().unwrap_or(root).canonicalize()?;
    Ok(match full.file_name() {
        Some(name) => parent.join(name),
        None => parent,
    })
}

/// Where `relative` below the canonical `root` leads.
///
/// # Errors
///
/// The platform's error from looking at the path, other than `NotFound`.
pub(super) fn locate(root: &Path, relative: &Path) -> io::Result<Located> {
    let full = root.join(relative);
    let is_link = match std::fs::symlink_metadata(&full) {
        Ok(meta) => meta.file_type().is_symlink(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    let (link, target) = if is_link {
        let at = link_location(root, relative)?;
        if !at.starts_with(root) {
            return Ok(Located::Outside {
                link: Link::Symlink,
            });
        }
        match full.canonicalize() {
            Ok(target) => (Link::Symlink, target),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Located::Dangling { at });
            }
            Err(error) => return Err(error),
        }
    } else {
        (Link::Direct, through_deepest_ancestor(root, relative)?)
    };
    Ok(if target.starts_with(root) {
        Located::Inside { link, target }
    } else {
        Located::Outside { link }
    })
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A project directory and its canonical root.
    fn project() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().canonicalize()?;
        Ok((dir, root))
    }

    #[test]
    fn relative_path_drops_dot_and_empty_segments() {
        assert_eq!(relative_path("a//./b"), Path::new("a/b"));
        assert_eq!(relative_path("."), PathBuf::new());
        assert_eq!(relative_path("a/../b"), Path::new("a/../b"));
    }

    #[test]
    fn a_plain_file_resolves_to_itself() -> TestResult {
        let (_dir, root) = project()?;
        std::fs::create_dir(root.join("a"))?;
        std::fs::write(root.join("a/b.md"), "x")?;
        assert_eq!(
            locate(&root, Path::new("a/b.md"))?,
            Located::Inside {
                link: Link::Direct,
                target: root.join("a/b.md")
            }
        );
        Ok(())
    }

    #[test]
    fn a_missing_path_resolves_through_its_deepest_existing_ancestor() -> TestResult {
        let (_dir, root) = project()?;
        assert_eq!(
            locate(&root, Path::new("new/dir/x.md"))?,
            Located::Inside {
                link: Link::Direct,
                target: root.join("new/dir/x.md")
            }
        );
        Ok(())
    }

    #[test]
    fn a_missing_path_with_a_parent_component_is_resolved_by_name() -> TestResult {
        let (_dir, root) = project()?;
        assert_eq!(
            locate(&root, Path::new("a/../b/x.md"))?,
            Located::Inside {
                link: Link::Direct,
                target: root.join("b/x.md")
            }
        );
        Ok(())
    }

    #[test]
    fn an_inside_symlink_is_followed() -> TestResult {
        let (_dir, root) = project()?;
        std::fs::write(root.join("real.md"), "x")?;
        symlink("real.md", root.join("link.md"))?;
        assert_eq!(
            locate(&root, Path::new("link.md"))?,
            Located::Inside {
                link: Link::Symlink,
                target: root.join("real.md")
            }
        );
        Ok(())
    }

    #[test]
    fn escape_is_refused_for_a_symlinked_file() -> TestResult {
        let (_dir, root) = project()?;
        let (_other, elsewhere) = project()?;
        std::fs::write(elsewhere.join("hosts"), "x")?;
        symlink(elsewhere.join("hosts"), root.join("link"))?;
        assert_eq!(
            locate(&root, Path::new("link"))?,
            Located::Outside {
                link: Link::Symlink
            }
        );
        Ok(())
    }

    #[test]
    fn escape_is_refused_through_a_symlinked_directory() -> TestResult {
        let (_dir, root) = project()?;
        let (_other, elsewhere) = project()?;
        std::fs::write(elsewhere.join("x.md"), "x")?;
        symlink(&elsewhere, root.join("d"))?;
        assert_eq!(
            locate(&root, Path::new("d/x.md"))?,
            Located::Outside { link: Link::Direct }
        );
        assert_eq!(
            locate(&root, Path::new("d/new.md"))?,
            Located::Outside { link: Link::Direct }
        );
        Ok(())
    }

    #[test]
    fn a_parent_component_is_resolved_through_the_real_directory() -> TestResult {
        let (_dir, root) = project()?;
        let (_other, elsewhere) = project()?;
        std::fs::create_dir(elsewhere.join("sub"))?;
        std::fs::create_dir(elsewhere.join("packs"))?;
        symlink(elsewhere.join("sub"), root.join("d"))?;
        for path in ["d/..", "d/../packs", "d/../missing", "d/../missing/x.md"] {
            assert_eq!(
                locate(&root, Path::new(path))?,
                Located::Outside { link: Link::Direct },
                "{path}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_parent_component_through_a_real_inside_directory_stays_inside() -> TestResult {
        let (_dir, root) = project()?;
        std::fs::create_dir_all(root.join("d/e"))?;
        std::fs::create_dir(root.join("packs"))?;
        assert_eq!(
            locate(&root, Path::new("d/e/../../packs"))?,
            Located::Inside {
                link: Link::Direct,
                target: root.join("packs")
            }
        );
        Ok(())
    }

    #[test]
    fn a_missing_suffix_that_climbs_above_the_root_is_outside() -> TestResult {
        let (_dir, root) = project()?;
        assert_eq!(
            locate(&root, Path::new("a/../../b"))?,
            Located::Outside { link: Link::Direct }
        );
        Ok(())
    }

    #[test]
    fn a_symlink_reached_through_an_outside_directory_is_outside() -> TestResult {
        let (_dir, root) = project()?;
        let (_other, elsewhere) = project()?;
        std::fs::create_dir(elsewhere.join("sub"))?;
        symlink(elsewhere.join("nowhere"), elsewhere.join("ghost"))?;
        symlink(elsewhere.join("sub"), root.join("d"))?;
        assert_eq!(
            locate(&root, Path::new("d/../ghost"))?,
            Located::Outside {
                link: Link::Symlink
            }
        );
        Ok(())
    }

    #[test]
    fn a_dangling_symlink_is_reported_dangling() -> TestResult {
        let (_dir, root) = project()?;
        symlink(root.join("missing"), root.join("gone"))?;
        assert_eq!(
            locate(&root, Path::new("gone"))?,
            Located::Dangling {
                at: root.join("gone")
            }
        );
        Ok(())
    }

    #[test]
    fn a_symlinked_root_is_canonicalised_once() -> TestResult {
        let (_dir, real) = project()?;
        std::fs::write(real.join("a.md"), "x")?;
        let holder = tempfile::tempdir()?;
        let through = holder.path().join("project");
        symlink(&real, &through)?;
        let root = through.canonicalize()?;
        assert_eq!(
            locate(&root, Path::new("a.md"))?,
            Located::Inside {
                link: Link::Direct,
                target: real.join("a.md")
            }
        );
        Ok(())
    }
}
