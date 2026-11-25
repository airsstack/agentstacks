//! The subcommands of `ext`, one module each.
//!
//! # Responsibilities
//!
//! - `add` runs `ext add`: it calls the library flow, prints the line `agentstacks.toml: added
//!   tools.<name>` and then plans, shows and applies like `update`.
//! - `check` runs `ext test`: it resolves the directories against the project directory, calls the
//!   library flow and prints the report. The code says `check` where the command line says `test`,
//!   so that the word does not collide with Rust's own tests.
//!
//! # Non-responsibilities
//!
//! The flows and the report text belong to the library and to `show`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

mod add;
mod check;

pub(crate) use add::{AddRequest, run as add};
pub(crate) use check::{CheckRequest, run as check};
