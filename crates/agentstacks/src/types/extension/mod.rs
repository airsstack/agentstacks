//! A tool extension's directory read into one checked extension: its `extension.toml`, its entry file and the hash of its tree.
//!
//! An extension is the unit that turns packs into one tool's files, so everything that can be
//! refused about its directory is found here, in one run, before the extension is held as valid.
//!
//! # Responsibilities
//!
//! - [`Extension`]: the loaded and checked extension, built by [`Extension::load`] from a [`FileTree`](crate::types::FileTree).
//! - [`ExtensionManifest`]: the parsed `extension.toml`, checked field by field.
//! - [`EntryPath`]: the relative `.lua` path of the adapter, which stays inside the extension.
//! - [`ExtensionError`]: the refusals of an extension's directory and manifest with a wording of their own.
//!
//! # Non-responsibilities
//!
//! What the adapter exports and how it runs belongs to the extension host. Comparing the tree
//! hash with the lock belongs to the plan and apply stage, not to the lock's own types, and
//! checking an extension against the `agentstacks.toml` entry that named it belongs to the set
//! checks.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod entry;
pub mod error;
pub mod loaded;
pub mod manifest;

#[doc(inline)]
pub use entry::EntryPath;
#[doc(inline)]
pub use error::ExtensionError;
#[doc(inline)]
pub use loaded::Extension;
#[doc(inline)]
pub use manifest::ExtensionManifest;
