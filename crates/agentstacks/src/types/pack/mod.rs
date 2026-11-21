//! A pack directory read into one checked pack: its layout, its content and every rule that needs the files.
//!
//! A pack is the unit the rest of agentstacks installs, so everything that can be refused about one
//! directory is found here, in one run, before anything reads the pack as valid.
//!
//! # Responsibilities
//!
//! - [`Pack`]: the loaded and checked pack, built by [`Pack::load`] from a [`FileTree`](crate::types::FileTree).
//! - [`PackSkill`]: a skill's checked frontmatter with the files installed from its directory.
//! - [`PackAgent`]: an agent's checked frontmatter with the pack-relative path of its file.
//! - [`NativeDir`]: one `native/<tool>/` directory, its installed files and its parsed hooks.
//! - [`PackError`]: the refusals that depend on a pack's files.
//! - [`ItemKind`]: the kind of item a token or hook points at, a skill or an agent.
//!
//! The loader's building blocks stay inside the crate: `layout` (where each walked file belongs,
//! with the layout refusals), `install` (which files are installed and rendered, and the checks on
//! that), `scan` (a rendered file's text into located tokens), `local` (the token rules that need
//! only the pack), `targets` (what the pack's own hooks point at) and `key` (the key paths
//! refusals are located at).
//!
//! # Non-responsibilities
//!
//! The messages of the files themselves (`pack.toml` fields, frontmatter, token syntax) belong to
//! their own groups. Checks across several packs or against `agentstacks.toml` belong to the checks
//! over the loaded packs, and installing a pack belongs to the executor.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod error;
mod install;
pub mod item;
mod key;
mod layout;
pub mod load;
mod local;
pub mod native;
mod scan;
mod targets;
#[cfg(test)]
mod testing;

#[doc(inline)]
pub use error::PackError;
#[doc(inline)]
pub use item::ItemKind;
#[doc(inline)]
pub use item::PackAgent;
#[doc(inline)]
pub use item::PackSkill;
#[doc(inline)]
pub use load::Pack;
#[doc(inline)]
pub use native::NativeDir;

pub(in crate::types) use key::KeyPath;
pub(in crate::types) use layout::{MANIFEST_FILE, native_hooks_file};
