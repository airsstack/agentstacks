//! One `native/<tool>/` directory of a pack.
//!
//! It is its own type because a pack's content for one tool has a shape of its own, files plus
//! hooks, that the extension for that tool reads apart from the neutral content.
//!
//! # Responsibilities
//!
//! - [`NativeDir`] holds the tool's name, the files installed from the directory (relative to it,
//!   `hooks.toml` not among them) and the tool's parsed hooks when a `hooks.toml` exists and is not
//!   excluded.
//! - [`NativeDir::hook_count`] counts the hooks, so a pack that declares only native hooks still
//!   counts as installing something.
//!
//! # Non-responsibilities
//!
//! Finding the directory in the pack, applying `exclude` and parsing `hooks.toml` belong to the
//! pack loader. Which tool a directory is for, and whether that tool exists, belong to the tool's
//! extension.

use crate::types::hooks::NativeHooks;
use crate::types::name::ToolName;
use crate::types::tree::TreePath;

/// The installed content of `native/<tool>/`.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::Pack;
///
/// let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-native-hooks");
/// let pack = Pack::load(&StdFileTree::new(dir)).map_err(|refusals| format!("{refusals:?}"))?;
/// let native = pack.native().first().ok_or("the pack has a native directory")?;
/// assert_eq!(native.tool().as_str(), "claude");
/// assert_eq!(native.hook_count(), 1);
/// // hooks.toml is parsed into hooks, never installed as a file.
/// assert!(native.files().is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct NativeDir {
    tool: ToolName,
    files: Vec<TreePath>,
    hooks: Option<NativeHooks>,
}

impl NativeDir {
    /// A directory for `tool` with its installed `files` and its `hooks`, when it has any.
    #[must_use]
    pub const fn new(tool: ToolName, files: Vec<TreePath>, hooks: Option<NativeHooks>) -> Self {
        Self { tool, files, hooks }
    }

    /// The tool the directory is for.
    #[must_use]
    pub const fn tool(&self) -> &ToolName {
        &self.tool
    }

    /// The installed files, relative to `native/<tool>/`.
    #[must_use]
    pub fn files(&self) -> &[TreePath] {
        &self.files
    }

    /// The tool's hooks; `None` when there is no `hooks.toml` or it is excluded.
    #[must_use]
    pub const fn hooks(&self) -> Option<&NativeHooks> {
        self.hooks.as_ref()
    }

    /// How many hooks the directory declares.
    #[must_use]
    pub fn hook_count(&self) -> usize {
        self.hooks.as_ref().map_or(0, |hooks| hooks.hooks().len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manifest::Requires;
    use std::error::Error;
    use std::path::Path;

    #[test]
    fn native_dir_counts_hook_entries() -> Result<(), Box<dyn Error>> {
        let tool = ToolName::new("claude")?;
        assert_eq!(
            NativeDir::new(tool.clone(), Vec::new(), None).hook_count(),
            0
        );

        let text = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"a\"\n\n\
                    [[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"b\"\n";
        let hooks = NativeHooks::parse(
            Path::new("native/claude/hooks.toml"),
            text,
            &Requires::default(),
        )
        .map_err(|d| format!("{:?}", d.into_sorted()))?;
        assert_eq!(
            NativeDir::new(tool, Vec::new(), Some(hooks)).hook_count(),
            2
        );
        Ok(())
    }
}
