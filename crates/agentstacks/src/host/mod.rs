//! The adapter that runs a tool extension's Lua adapter in the airsl sandbox with zero grants, and
//! the only place Lua runs, so that an extension can only propose operations.
//!
//! It translates between the adapter port and the sandbox and decides nothing.
//!
//! # Responsibilities
//!
//! - [`AirslAdapter`] implements the adapter port, and [`Session`] is one extension's engine from
//!   running its entry file to its last `plan` call.
//! - Reading every `.lua` file of an extension into memory, so Lua never touches the disk.
//! - Building the sandbox: Lua's pure base, `airsstack.json`, `airsstack.ext` and the
//!   `agentstacks` table, with `print` and `warn` removed.
//! - The `agentstacks` table: `on` registers a handler; `write`, `copy`, `skip` and
//!   `require_binary` propose operations while the `plan` handler runs; `read` serves the project
//!   files the extension declared; `require` loads the extension's own modules from memory; and
//!   `json.edit` edits JSON text.
//! - Reading the arguments of every call strictly, with no coercion; locating a refusal at the
//!   nearest Lua frame; converting Lua values to JSON; and wording a Lua failure for the user.
//! - Building the payloads the handlers are called with and reading the answer of `values`.
//!
//! # Non-responsibilities
//!
//! No rule is decided here: the shape of an operation, a path or a JSON edit belongs to `types`,
//! and the order of the flows belongs to `app`. The host reaches the outside only through the
//! ports, and never writes.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod adapter;
mod args;
mod caller;
mod convert;
mod engine;
mod events;
mod failure;
mod files;
mod json;
mod module;
mod ops;
mod payload;
mod read;
mod require;
mod session;
mod state;
#[cfg(test)]
mod testing;
mod values;

#[doc(inline)]
pub use adapter::AirslAdapter;
#[doc(inline)]
pub use session::Session;
