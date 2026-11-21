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
//! - [`PackLayout`]: where each walked file of a pack belongs, with the layout refusals.
//! - [`NativeLayout`]: the files the layout found under one `native/<tool>/` directory.
//! - [`InstallSet`]: which files of a pack are installed and which of those are rendered.
//! - [`check_airsl_declared()`]: refuses a pack with `.lua` files and no `requires.airsl`.
//! - [`check_something_to_install()`]: refuses a pack that installs no skill, agent or hook.
//! - [`rendered_text()`]: the text of a rendered file's bytes, or the refusal that they are not UTF-8.
//! - [`scan_body()`]: scans text into located tokens, with each refusal located at its line.
//! - [`check_native_references()`]: refuses a `root` token into `native/` from neutral content.
//! - [`check_local_tokens()`]: refuses a token that names this pack's own item when the pack does not ship it.
//! - [`HookTargets`]: the files and agents a pack's own hooks point at, with the check that they exist.
//! - [`PackError`]: the refusals that depend on a pack's files.
//! - [`ItemKind`]: the kind of item a token or hook points at, a skill or an agent.
//!
//! # Non-responsibilities
//!
//! The messages of the files themselves (`pack.toml` fields, frontmatter, token syntax) belong to
//! their own groups. Checks across several packs or against `agentstacks.toml` belong to the checks
//! over the loaded packs, and installing a pack belongs to the executor.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod error;
pub mod install;
pub mod item;
pub mod layout;
pub mod load;
pub mod local;
pub mod native;
pub mod scan;
pub mod targets;
#[cfg(test)]
mod testing;

#[doc(inline)]
pub use error::PackError;
#[doc(inline)]
pub use install::InstallSet;
#[doc(inline)]
pub use install::check_airsl_declared;
#[doc(inline)]
pub use install::check_something_to_install;
#[doc(inline)]
pub use item::ItemKind;
#[doc(inline)]
pub use item::PackAgent;
#[doc(inline)]
pub use item::PackSkill;
#[doc(inline)]
pub use layout::NativeLayout;
#[doc(inline)]
pub use layout::PackLayout;
#[doc(inline)]
pub use scan::rendered_text;
#[doc(inline)]
pub use scan::scan_body;
#[doc(inline)]
pub use targets::HookTargets;

pub(crate) use layout::{MANIFEST_FILE, NATIVE_DIR, NATIVE_HOOKS_FILE};
#[doc(inline)]
pub use load::Pack;
#[doc(inline)]
pub use local::check_local_tokens;
#[doc(inline)]
pub use local::check_native_references;
#[doc(inline)]
pub use native::NativeDir;
