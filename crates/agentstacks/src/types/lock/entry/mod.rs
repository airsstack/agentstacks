//! The entries a lock holds: what an install resolved, and what it wrote into the project.
//!
//! A resolved tool or pack and a written file record different facts, so they live in separate
//! files under one group.
//!
//! # Responsibilities
//!
//! - [`LockedEntry`], [`LockedSource`]: one locked tool or pack and where it came from.
//! - [`FileEntry`]: one file an install wrote.
//! - [`EntryKey`]: what identifies one entry, so the duplicate refusal can name it.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod key;
pub mod locked;
pub mod written;

#[doc(inline)]
pub use key::EntryKey;
#[doc(inline)]
pub use locked::LockedEntry;
#[doc(inline)]
pub use locked::LockedSource;
#[doc(inline)]
pub use written::FileEntry;
