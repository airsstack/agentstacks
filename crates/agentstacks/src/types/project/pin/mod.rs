//! What a git source is fixed to, and the rule that says which pin keys it must carry.
//!
//! The project file and the lock pin differently but read through the same code, so the pin and
//! its rule live together.
//!
//! # Responsibilities
//!
//! - [`Pin`]: what a git source is fixed to, a tag or a commit.
//! - `PinRule` (crate-internal): which pin keys a git source must carry, so one reader serves the
//!   project file and the lock.
//!
//! # Non-responsibilities
//!
//! Reading the pin keys out of a table belongs to the source reader, `SourceFields`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod kind;
mod rule;

#[doc(inline)]
pub use kind::Pin;
pub(in crate::types) use rule::PinRule;
