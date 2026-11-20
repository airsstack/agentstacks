//! The agentstacks core library.
//!
//! agentstacks sets up agentic coding tools from one tool-neutral copy of skills, agents,
//! commands, hooks and rules. This crate is the home of the logic behind that. It is a library,
//! separate from the command-line front end, so the same core can be driven by the CLI, by tests,
//! and by other callers.
//!
//! # Responsibilities
//!
//! - [`model`] is the home of the vocabulary: packs, tool extensions, operations and the lock.
//! - [`validation`] is the home of the checks on every operation an extension proposes.
//! - [`plan`] is the home of the view that shows every operation before anything is applied.
//! - [`executor`] is the home of the only writer to disk.
//! - [`lock`] is the home of the pinned versions and file hashes in `agentstacks.lock`.
//! - [`extension_host`] is the home of the runner for Lua tool extensions under airsl with zero
//!   grants.
//!
//! Every module currently holds no items.
//!
//! # Non-responsibilities
//!
//! Argument parsing and terminal output belong to the command-line front end, not to this crate.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod executor;
pub mod extension_host;
pub mod lock;
pub mod model;
pub mod plan;
pub mod validation;
