//! The agentstacks core library.
//!
//! agentstacks sets up agentic coding tools from one tool-neutral copy of skills, agents,
//! commands, hooks and rules. This crate is the home of the logic behind that. It is a library,
//! separate from the command-line front end, so the same core can be driven by the CLI, by tests,
//! and by other callers.
//!
//! # Responsibilities
//!
//! - [`types`] is the home of the rules: domain data and the pure decisions over it, from the
//!   pack, extension, project and lock formats to [`types::op`] (an operation and its validation),
//!   [`types::json`] (editing a JSON document's text at one key path), [`types::plan`] (the step
//!   for each path), [`types::resolve`] (reconciling the lock with the project file and choosing
//!   tags) and [`types::protocol`] (the hook command line and the text of the hook runtime).
//! - [`port`] is the home of the traits the flows and the rules read the outside through, each with
//!   a fake for tests.
//! - [`app`] is the home of the flows: the use cases, generic over ports, that call ports in order
//!   and hand the results to the rules in `types`.
//! - [`fs`](mod@fs) is the home of the adapter that reads the real disk, and the only place that
//!   calls `std::fs`.
//! - [`process`] is the home of the adapter that drives the `git` CLI and looks up binaries on
//!   `PATH`, and the only place that calls `std::process`.
//! - [`host`] is the home of the adapter that runs a tool extension's Lua adapter in the airsl
//!   sandbox with zero grants, and the only place Lua runs.
//!
//! # Dependency direction
//!
//! ```text
//! types  -> port::{FileTree, EntryKind} only
//! port   -> types
//! app    -> port, types
//! fs, process, host -> port, types
//! process -> fs::cache only, among the adapters
//! ```
//!
//! `types`, `port` and `app` never name an adapter and never call `std::fs`, `std::process` or
//! `std::net`; the isolation test in `tests/isolation.rs` enforces both. An adapter makes no
//! decision: it translates between a port and the outside.
//!
//! # Non-responsibilities
//!
//! Argument parsing and terminal output belong to the command-line front end, not to this crate.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod app;
pub mod fs;
pub mod host;
pub mod port;
pub mod process;
pub mod types;
