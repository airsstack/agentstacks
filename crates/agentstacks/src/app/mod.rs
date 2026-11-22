//! The use cases: flows generic over the ports, each calling ports in order and handing the
//! results to the rules in `types`.
//!
//! # Responsibilities
//!
//! - [`plan`]: building a reviewable plan.
//! - [`apply`]: applying a plan that was shown, as the only writer.
//! - [`doctor`]: reporting drift against the lock.
//! - [`update`]: finding newer tags for git sources.
//!
//! # Non-responsibilities
//!
//! No rule and no adapter: a decision belongs to `types`, and reaching the disk, a process or the
//! network belongs to an adapter behind a port. Nothing here calls `std::fs`, `std::process` or
//! `std::net`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod apply;
pub mod doctor;
pub mod plan;
pub mod update;
