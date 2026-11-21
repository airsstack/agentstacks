//! The agentstacks core library.
//!
//! agentstacks sets up agentic coding tools from one tool-neutral copy of skills, agents,
//! commands, hooks and rules. This crate is the home of the logic behind that. It is a library,
//! separate from the command-line front end, so the same core can be driven by the CLI, by tests,
//! and by other callers.
//!
//! # Responsibilities
//!
//! - [`types`] is the home of the domain types and the logic over them, reaching the disk only
//!   through the ports it declares.
//! - [`validation`] is the home of the checks on every operation an extension proposes.
//! - [`fs`](mod@fs) is the home of the adapter that reads the real disk for the ports `types`
//!   declares, and the only place that calls `std::fs`.
//! - [`plan`] is the home of the view that shows every operation before anything is applied.
//! - [`executor`] is the home of the only writer to disk.
//! - [`host`] is the home of the runner for Lua tool extensions under airsl with zero
//!   grants.
//!
//! # Non-responsibilities
//!
//! Argument parsing and terminal output belong to the command-line front end, not to this crate.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod executor;
pub mod fs;
pub mod host;
pub mod plan;
pub mod types;
pub mod validation;
