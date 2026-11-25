//! The use cases: flows generic over the ports, each calling ports in order and handing the
//! results to the rules in `types`.
//!
//! # Responsibilities
//!
//! - [`plan`]: building a reviewable plan.
//! - [`apply`]: applying a plan that was shown, as the only writer.
//! - [`init`]: writing a new `agentstacks.toml` from the arguments of `init`.
//! - [`ext`]: the flows of `ext add`, which adds a tool extension to an existing
//!   `agentstacks.toml`, and `ext test`, which checks an extension in a temporary project.
//! - [`doctor`]: reporting drift against the lock.
//! - [`update`]: finding newer tags for git sources.
//! - `newest`: the newest release tag of the official repository, shared by `init` and `ext add`.
//! - `local`: a directory of the project opened as a source, shared by `init` and `ext add`.
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
pub mod ext;
pub mod init;
mod local;
mod newest;
pub mod plan;
pub mod update;
