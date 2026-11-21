//! The refusals pack loading owns.
//!
//! A refusal is a message without its location: the diagnostic that carries it names the file, the
//! line or the key path.
//!
//! It is separate from the loader so the layout, install-set, hook and token rules can all raise the
//! same refusals without depending on the code that reads the files.
//!
//! # Responsibilities
//!
//! - [`PackError`] has a variant per refusal that depends on the files of a pack: the layout, the
//!   `render` and `exclude` lists, what the pack installs, and what its content points at. Each
//!   displays exactly the message the author reads after the location.
//!
//! # Non-responsibilities
//!
//! The messages of the files themselves (`pack.toml` fields, frontmatter, token syntax) belong to
//! the errors of those formats. An invalid name is not redeclared: it wraps
//! [`InvalidName`].

use crate::types::name::InvalidName;
use crate::types::name::PackName;
use crate::types::pack::item::kind::ItemKind;

/// A refusal found while loading a pack directory.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{ItemKind, PackError, PackName};
///
/// assert_eq!(PackError::NotAPack.to_string(), "not a pack (no pack.toml)");
/// assert_eq!(
///     PackError::DoesNotExist { path: "enforcment.json".into() }.to_string(),
///     "\"enforcment.json\" does not exist"
/// );
///
/// let missing = PackError::NoSuchItem {
///     pack: PackName::new("core")?,
///     kind: ItemKind::Agent,
///     item: "codr".to_owned(),
/// };
/// assert_eq!(missing.to_string(), "pack \"core\" has no agent \"codr\"");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    /// The directory has no `pack.toml` at its root.
    #[error("not a pack (no pack.toml)")]
    NotAPack,
    /// A directory the format reserves and does not support in api 0.
    #[error("reserved, not supported in api 0")]
    Reserved,
    /// A skill directory without a `SKILL.md`.
    #[error("missing SKILL.md")]
    MissingSkillMd,
    /// A file directly in `skills/`.
    #[error("only skill directories belong in skills/")]
    OnlySkillDirectories,
    /// Something in `agents/` that is not an `<name>.md` file.
    #[error("agents/ holds only <name>.md files")]
    OnlyAgentFiles,
    /// A file directly in `native/`.
    #[error("only tool directories belong in native/")]
    OnlyToolDirectories,
    /// A name that is not a valid skill, agent or tool name.
    #[error(transparent)]
    Name(#[from] InvalidName),
    /// A `render` entry or a hook `run` that names no file.
    #[error("\"{path}\" does not exist")]
    DoesNotExist {
        /// The path as written.
        path: String,
    },
    /// A `render` entry that names a file the pack never installs, such as `pack.toml`.
    #[error("\"{path}\" is never installed")]
    NeverInstalled {
        /// The path as written.
        path: String,
    },
    /// A `render` entry that an `exclude` pattern matches.
    #[error("\"{path}\" is also excluded")]
    AlsoExcluded {
        /// The path as written.
        path: String,
    },
    /// An `exclude` pattern that matches a file that must stay.
    #[error("\"{glob}\" excludes {path}")]
    ExcludesProtected {
        /// The pattern as written.
        glob: String,
        /// The pack-relative path it matches.
        path: String,
    },
    /// Lua files are installed and `requires.airsl` is missing.
    #[error("required (the pack contains .lua files)")]
    AirslRequired,
    /// No skill, agent or hook is installed.
    #[error("pack declares nothing to install")]
    NothingToInstall,
    /// A file that is rendered but is not UTF-8. A file that is read and never rendered gets the
    /// plain wording of [`FileTree::read_text`](crate::types::FileTree::read_text) instead.
    #[error("not valid UTF-8, cannot render")]
    NotUtf8,
    /// A `root` token in neutral content that reaches into `native/`.
    #[error("neutral content must not reference native/")]
    NativeReference,
    /// `{{var arguments}}` outside a `SKILL.md`.
    #[error("var \"arguments\" is only valid in SKILL.md")]
    ArgumentsOutsideSkill,
    /// A token or hook that names an item the pack does not ship.
    #[error("pack \"{pack}\" has no {kind} \"{item}\"")]
    NoSuchItem {
        /// The pack that was searched.
        pack: PackName,
        /// Whether a skill or an agent was named.
        kind: ItemKind,
        /// The item name as written.
        item: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::ItemName;
    use std::error::Error;

    #[test]
    fn pack_error_messages_read_as_documented() -> Result<(), Box<dyn Error>> {
        let missing = |kind, item: &str| -> Result<PackError, InvalidName> {
            Ok(PackError::NoSuchItem {
                pack: PackName::new("core")?,
                kind,
                item: item.to_owned(),
            })
        };
        let bad_name = ItemName::new("My_Skill")
            .err()
            .ok_or("My_Skill must be refused")?;
        let cases: Vec<(PackError, &str)> = vec![
            (PackError::NotAPack, "not a pack (no pack.toml)"),
            (PackError::Reserved, "reserved, not supported in api 0"),
            (PackError::MissingSkillMd, "missing SKILL.md"),
            (
                PackError::OnlySkillDirectories,
                "only skill directories belong in skills/",
            ),
            (
                PackError::OnlyAgentFiles,
                "agents/ holds only <name>.md files",
            ),
            (
                PackError::OnlyToolDirectories,
                "only tool directories belong in native/",
            ),
            (
                PackError::Name(bad_name),
                "\"My_Skill\" is not a valid name",
            ),
            (
                PackError::DoesNotExist {
                    path: "enforcment.json".into(),
                },
                "\"enforcment.json\" does not exist",
            ),
            (
                PackError::NeverInstalled {
                    path: "pack.toml".into(),
                },
                "\"pack.toml\" is never installed",
            ),
            (
                PackError::AlsoExcluded {
                    path: "x.json".into(),
                },
                "\"x.json\" is also excluded",
            ),
            (
                PackError::ExcludesProtected {
                    glob: "**/*.md".into(),
                    path: "skills/discuss/SKILL.md".into(),
                },
                "\"**/*.md\" excludes skills/discuss/SKILL.md",
            ),
            (
                PackError::AirslRequired,
                "required (the pack contains .lua files)",
            ),
            (
                PackError::NothingToInstall,
                "pack declares nothing to install",
            ),
            (PackError::NotUtf8, "not valid UTF-8, cannot render"),
            (
                PackError::NativeReference,
                "neutral content must not reference native/",
            ),
            (
                PackError::ArgumentsOutsideSkill,
                "var \"arguments\" is only valid in SKILL.md",
            ),
            (
                missing(ItemKind::Skill, "coder")?,
                "pack \"core\" has no skill \"coder\"",
            ),
            (
                missing(ItemKind::Agent, "codr")?,
                "pack \"core\" has no agent \"codr\"",
            ),
            (
                missing(ItemKind::Agent, "task-briefer")?,
                "pack \"core\" has no agent \"task-briefer\"",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
        Ok(())
    }
}
