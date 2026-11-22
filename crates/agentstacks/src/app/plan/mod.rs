//! The flow that builds a reviewable plan: every operation an apply would perform, decided and
//! shown in full before anything is written.
//!
//! The flow reads the project and its lock, fetches every source, loads and checks the packs and
//! extensions, runs each tool's extension, checks the binaries it needs, and decides the step for
//! each path. It collects every refusal it meets and returns either all of them or one
//! [`Plan`](crate::types::plan::Plan).
//!
//! # Responsibilities
//!
//! - `read`: reading `agentstacks.toml` and `agentstacks.lock`.
//! - `sources`: reconciling the lock with the project, fetching each source and placing every
//!   fetch failure at its entry.
//! - `load`: loading the fetched trees, running the set check, and noting ignored dot entries and
//!   unused `native/` directories.
//! - `render`: filling a tool's token values into the texts of each pack.
//! - `tools`: running each tool's extension over the rendered packs, in name order, then each
//!   removed tool, and checking what they proposed.
//! - `binaries`: looking up the binaries and the `airsl` version the plan needs.
//! - `steps`: deciding the step of every proposed path and of every locked path nothing proposes.
//! - `flow`: [`plan()`], the ten steps in order, building the plan.
//!
//! # Non-responsibilities
//!
//! No rule and no adapter. Every decision is a call into `types`, and every read of the outside
//! goes through a port: nothing here touches the disk, `git`, `PATH` or Lua. Showing the plan and
//! writing it belong to the commands and the apply flow.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod binaries;
mod flow;
mod load;
mod read;
mod render;
mod sources;
mod steps;
#[cfg(test)]
mod testing;
mod tools;

#[doc(inline)]
pub use flow::plan;
