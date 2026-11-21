//! Hooks written in a tool's own event vocabulary: one hook and the whole `hooks.toml` of a tool.
//!
//! A native hook is kept opaque here, because only that tool's extension knows its events.
//!
//! # Responsibilities
//!
//! - [`NativeHook`]: a hook of a tool's own event vocabulary, kept opaque.
//! - [`NativeHooks`]: a whole `native/<tool>/hooks.toml`.
//!
//! # Non-responsibilities
//!
//! Whether a tool knows a native event belongs to that tool's extension.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod file;
pub mod hook;

#[doc(inline)]
pub use file::NativeHooks;
#[doc(inline)]
pub use hook::NativeHook;
