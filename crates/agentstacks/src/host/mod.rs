//! The adapter that runs a tool extension's Lua adapter in the airsl sandbox with zero grants, and
//! the only place Lua runs, so that an extension can only propose operations.
//!
//! It translates between the adapter port and the sandbox and decides nothing.
//!
//! This module currently holds no items. It is an export-only module with no executable logic,
//! so it carries no inline tests.
