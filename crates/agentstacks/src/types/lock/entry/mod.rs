//! The entries a lock holds: what an install resolved, and what it wrote into the project.
//!
//! A resolved tool or pack and a written file or merge record different facts, so they live in
//! separate files under one group.
//!
//! # Responsibilities
//!
//! - [`LockedEntry`], [`LockedSource`]: one locked tool or pack and where it came from.
//! - [`FileEntry`], [`MergeEntry`]: one file an install wrote, and one merge it made into a file
//!   it does not own.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod locked;
pub mod written;

#[doc(inline)]
pub use locked::LockedEntry;
#[doc(inline)]
pub use locked::LockedSource;
#[doc(inline)]
pub use written::FileEntry;
#[doc(inline)]
pub use written::MergeEntry;
