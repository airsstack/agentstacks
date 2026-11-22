//! The port to the engine that runs a tool extension, with a fake for tests.
//!
//! # Responsibilities
//!
//! - [`Adapter`]: open one extension, dispatch its `values` handler, dispatch its `plan` handler.
//!
//! # Non-responsibilities
//!
//! No logic and no adapter: running Lua belongs to the `host` module. The scripted extension that
//! tests build lives here but is compiled for tests only.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

#[cfg(test)]
mod fake;
mod seam;

#[cfg(test)]
#[doc(inline)]
pub use fake::AdapterCall;
#[cfg(test)]
#[doc(inline)]
pub use fake::FakeAdapter;
#[cfg(test)]
#[doc(inline)]
pub use fake::ScriptStep;
#[cfg(test)]
#[doc(inline)]
pub use fake::ToolScript;
#[doc(inline)]
pub use seam::Adapter;
