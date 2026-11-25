//! Helpers the tool-extension test binaries (`claude.rs`, `opencode.rs`) share.
//!
//! - [`airsl`]: the `airsl` binary check that skips a test, or fails it when the binary is required.
//! - [`tree`]: where a tool's extension and fixtures live, and the temporary project a plan runs in.
//! - [`plan`]: running the real plan flow and reading its steps and skips as plain text.
//! - [`hook`]: building a hook module and running its command.
//!
//! Every function here that depends on the tool takes its name (`"claude"`, `"opencode"`); each test
//! binary wraps those in its own one-argument helpers.

pub mod airsl;
pub mod hook;
pub mod plan;
pub mod tree;

pub(crate) use airsl::skip_without_airsl;
pub(crate) use hook::{module, run_piped};
pub(crate) use plan::{expected, planned, proposed, run_plan, skips};
pub(crate) use tree::{copy_tree, extension_dir, fixture, project};
