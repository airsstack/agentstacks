//! Editing the text of a JSON document at one key path.
//!
//! The text goes in and the edited text, or one refusal, comes out. Nothing here reads a file or
//! sees a Lua value. [`edit()`] is the entry point.
//!
//! # Responsibilities
//!
//! - [`edit()`]: sets or removes one key of a JSON text and prints the document back with the
//!   indent, line endings and trailing newline of the text.
//! - [`PathElem`]: one element of the key path, a key or the type word of a non-string.
//! - [`JsonEditError`]: every refusal of a `json.edit` call, and [`MAX_DEPTH`], how deep a written
//!   value may nest.
//!
//! # Non-responsibilities
//!
//! Reading the call's arguments and converting the Lua value that is written belong to the host
//! that calls [`edit()`].
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod edit;
mod elem;
pub mod error;
mod indent;
mod print;

#[doc(inline)]
pub use edit::edit;
#[doc(inline)]
pub use elem::PathElem;
#[doc(inline)]
pub use error::{JsonEditError, MAX_DEPTH};
