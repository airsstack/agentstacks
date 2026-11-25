//! The flows of `ext add` and `ext test`: installing a tool extension into a project from a path,
//! a git URL or an official name, and checking one against the built-in conformance pack.
//!
//! The add flow reads `agentstacks.toml`, resolves and loads the extension the way a plan does, and
//! writes the project file with one more `[tools.<name>]` table. Planning and applying what the
//! new tool proposes are the commands' next steps, not this flow's.
//!
//! The test flow opens an extension directory, runs its Lua tests, builds a temporary project from
//! it, the conformance pack and the packs the author named, and plans, applies and plans again
//! there. Nothing is written to the author's project.
//!
//! The code calls the `ext test` command `check`, in `check()`, `types::check` and the report
//! types, so that the word `test` stays free for Rust's own tests.
//!
//! # Responsibilities
//!
//! - [`add()`]: reads the project, resolves and loads the extension, and writes the project file
//!   with the new tool, returning the name of the tool it added.
//! - [`check()`]: runs the eight steps of `ext test` over the ports and returns a [`CheckReport`]
//!   of what each step found, with [`Dir`], [`Checked`], [`FirstPlan`] and [`LuaOutcome`] as its
//!   parts.
//!
//! # Non-responsibilities
//!
//! No rule and no adapter. Which arguments are paths, which options a git URL needs and where the
//! table goes in the file are rules in `types`, and so are the scratch project's file set and the
//! second-plan check; reading the project, `git`, the disk, `airsl` and the temporary directory
//! are ports. For `ext add`, planning, confirming and applying belong to the commands; showing the
//! report of `ext test` does too.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod add;
mod check;

#[doc(inline)]
pub use add::add;
#[doc(inline)]
pub use check::{CheckReport, Checked, Dir, FirstPlan, LuaOutcome, check};
