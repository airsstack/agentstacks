//! The hook command line and the text of the hook runtime.
//!
//! # Responsibilities
//!
//! - [`RUNTIME_FILE`], [`RUNTIME_LUA`]: the runtime's file name under the extension root and its
//!   text.
//! - [`HookKind`], [`Blocking`]: the two command-line arguments that say where a hook was declared
//!   and whether it may block.
//! - [`HookModule`]: the module name a hook is required by.
//! - [`HookCommand`]: the `airsl` argument vector that runs a hook through the runtime.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod blocking;
mod command;
mod kind;
mod module;
mod runtime;

#[doc(inline)]
pub use blocking::Blocking;
#[doc(inline)]
pub use command::HookCommand;
#[doc(inline)]
pub use kind::HookKind;
#[doc(inline)]
pub use module::HookModule;
#[doc(inline)]
pub use runtime::RUNTIME_FILE;
#[doc(inline)]
pub use runtime::RUNTIME_LUA;
