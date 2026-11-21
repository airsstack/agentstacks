//! The whole `pack.toml` and the pack-level values it is made of.
//!
//! The manifest reader and the path and version values it hands out are always used together, so
//! they share one folder.
//!
//! # Responsibilities
//!
//! - [`PackManifest`]: reads the whole file into one value, or reports every refusal of it at once.
//! - [`PackPath`]: a path that stays inside the pack, used by `render` and by hook scripts.
//! - [`PackVersion`]: a semantic version, used for packs, extensions and the locked airsl.
//!
//! # Non-responsibilities
//!
//! Checks that need the pack's files or the other packs belong to the pack loader and the set
//! checks.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod file;
pub mod path;
pub mod version;

#[doc(inline)]
pub use file::PackManifest;
#[doc(inline)]
pub use path::PackPath;
#[doc(inline)]
pub use version::PackVersion;
