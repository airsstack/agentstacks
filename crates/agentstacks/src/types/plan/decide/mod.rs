//! Deciding the step for a path.
//!
//! # Responsibilities
//!
//! - [`proposed`]: the step for a path an extension proposed.
//! - [`unproposed`]: the step for a path the lock records that nothing proposes now.
//!
//! # Non-responsibilities
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod proposed;
pub mod unproposed;

#[doc(inline)]
pub use proposed::ProposedPath;
#[doc(inline)]
pub use proposed::decide_proposed;
#[doc(inline)]
pub use unproposed::decide_unproposed;
