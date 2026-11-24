//! The checks over a whole project: the packs and extensions it lists, taken together.
//!
//! A pack or extension can be valid alone and still disagree with the rest of the project, so the
//! rules that need several of them live apart from the loaders that read one.
//!
//! # Responsibilities
//!
//! - [`check()`]: validates a project file with what loading each of its entries returned, and
//!   returns a [`ValidSet`] or every refusal.
//! - [`ValidSet`]: a project that passed every check, with its loaded packs and extensions, the
//!   resolved option values and the airsl range. It is the only input later stages accept.
//! - [`AirslRange`]: the `requires.airsl` ranges of the packs and the versions they admit
//!   together.
//! - [`SetError`]: the refusals with a wording of their own that these checks raise.
//! - `fetched`, `graph`, `targets`, `runs`, `options` and `interval` hold the crate-internal
//!   checks that [`check()`] runs, one rule each.
//!
//! # Non-responsibilities
//!
//! Loading a pack or an extension belongs to their own groups, and fetching their sources to the
//! stage that plans an install. Probing the installed airsl is left to that stage too; this group
//! only tests a given version against the ranges.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod airsl;
pub mod check;
pub mod error;
mod fetched;
mod graph;
mod interval;
mod options;
mod runs;
mod targets;
#[cfg(test)]
mod testing;
pub mod valid;

#[doc(inline)]
pub use airsl::AirslRange;
#[doc(inline)]
pub use check::check;
#[doc(inline)]
pub use error::SetError;
#[doc(inline)]
pub use valid::ValidSet;
