//! The relative path of a file the walker found.
//!
//! It is its own type so the walk, the tree hash and the pack layout all build and order paths the
//! same way, instead of each joining strings its own way.
//!
//! # Responsibilities
//!
//! - [`TreePath`] is a path relative to the walked directory, joined with `/` and valid UTF-8, so
//!   it reads the same on every platform and orders by its bytes.
//!
//! # Non-responsibilities
//!
//! The walker builds a [`TreePath`] for every file it lists, and the pack layout builds one for
//! every pack-relative path it derives, so the path text is assembled in one place. Building one
//! from a [`Path`] checks for valid UTF-8 but not for relativity: the caller
//! passes a path relative to the walked directory. Locating a refusal at a path belongs to the
//! walker.

use std::fmt;
use std::path::Path;

use crate::types::tree::walk::error::WalkError;

/// A path relative to a walked directory, `/`-separated and valid UTF-8.
///
/// Ordering is the byte order of the text.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::walk;
///
/// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
/// let (walked, _) = walk(&StdFileTree::new(pack));
/// let paths: Vec<String> = walked.files().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     paths,
///     ["hooks/enforce.lua", "pack.toml", "skills/a/SKILL.md"]
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreePath(String);

impl TreePath {
    /// Builds the `/`-joined form of `relative`.
    ///
    /// # Errors
    ///
    /// [`WalkError::NotUtf8`] when a component is not valid UTF-8.
    pub(crate) fn from_relative(relative: &Path) -> Result<Self, WalkError> {
        let mut text = String::new();
        for component in relative.components() {
            let part = component.as_os_str().to_str().ok_or(WalkError::NotUtf8)?;
            if !text.is_empty() {
                text.push('/');
            }
            text.push_str(part);
        }
        Ok(Self(text))
    }

    /// The `/`-joined form of `parts`, each of which may itself hold `/`-separated segments.
    ///
    /// The parts are text, so the result is valid UTF-8 whatever the platform.
    pub(crate) fn from_parts(parts: &[&str]) -> Self {
        Self(parts.join("/"))
    }

    /// Whether the last segment ends in `.<extension>`, compared exactly.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::walk;
    ///
    /// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
    /// let (walked, _) = walk(&StdFileTree::new(pack));
    /// let lua: Vec<&str> = walked
    ///     .files()
    ///     .iter()
    ///     .filter(|path| path.has_extension("lua"))
    ///     .map(|path| path.as_str())
    ///     .collect();
    /// assert_eq!(lua, ["hooks/enforce.lua"]);
    /// ```
    #[must_use]
    pub fn has_extension(&self, extension: &str) -> bool {
        Path::new(&self.0)
            .extension()
            .is_some_and(|found| found == extension)
    }

    /// The path as text, components joined with `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::walk;
    ///
    /// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-hooks");
    /// let (walked, _) = walk(&StdFileTree::new(pack));
    /// let nested = walked.files().iter().map(|path| path.as_str()).last();
    /// assert_eq!(nested, Some("skills/a/SKILL.md"));
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TreePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tree_path(parts: &[&str]) -> Result<TreePath, WalkError> {
        let relative: PathBuf = parts.iter().collect();
        TreePath::from_relative(&relative)
    }

    #[test]
    fn tree_path_joins_components_with_slash() {
        assert_eq!(
            tree_path(&["b", "c.lua"]).map(|p| p.as_str().to_owned()),
            Ok("b/c.lua".to_owned())
        );
        assert_eq!(
            tree_path(&["a.md"]).map(|p| p.as_str().to_owned()),
            Ok("a.md".to_owned())
        );
    }

    #[test]
    fn tree_path_from_parts_joins_parts_with_slash() {
        assert_eq!(
            TreePath::from_parts(&["skills", "a", "SKILL.md"]).as_str(),
            "skills/a/SKILL.md"
        );
        assert_eq!(
            TreePath::from_parts(&["native", "claude", "agents/x.md"]).as_str(),
            "native/claude/agents/x.md"
        );
        assert_eq!(TreePath::from_parts(&["pack.toml"]).as_str(), "pack.toml");
    }

    #[test]
    fn tree_path_from_parts_equals_the_walked_path() -> Result<(), WalkError> {
        let walked = tree_path(&["skills", "a", "SKILL.md"])?;
        assert_eq!(TreePath::from_parts(&["skills", "a", "SKILL.md"]), walked);
        Ok(())
    }

    #[test]
    fn tree_path_has_extension_compares_the_last_segment_exactly() {
        let has =
            |text: &str, extension: &str| TreePath::from_parts(&[text]).has_extension(extension);
        assert!(has("scripts/a.lua", "lua"));
        assert!(has("a.b.lua", "lua"));
        assert!(!has("scripts/a.lua", "md"));
        assert!(!has("a.LUA", "lua"));
        assert!(!has("lua", "lua"));
        assert!(!has("a.lua/b", "lua"));
    }

    #[cfg(unix)]
    #[test]
    fn tree_path_refuses_a_non_utf8_component() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let mut relative = PathBuf::from("a");
        relative.push(OsStr::from_bytes(&[0x66, 0xFF]));
        assert_eq!(
            TreePath::from_relative(&relative).map(|p| p.to_string()),
            Err(WalkError::NotUtf8)
        );
    }

    #[test]
    fn tree_path_orders_by_path_bytes() {
        let mut paths: Vec<TreePath> = ["b/c.lua", "a/b", "a.md", "B.md"]
            .iter()
            .filter_map(|text| tree_path(&text.split('/').collect::<Vec<_>>()).ok())
            .collect();
        paths.sort();
        let sorted: Vec<&str> = paths.iter().map(TreePath::as_str).collect();
        assert_eq!(sorted, ["B.md", "a.md", "a/b", "b/c.lua"]);
    }
}
