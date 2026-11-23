//! The execution harness: spawning hooks and scripts the way the Claude Code
//! runtime would, and judging what they produced.
//!
//! Responsibilities: [`default_payload`], [`merge`], [`substitute_project`],
//! [`Project`], [`overlay_into`], [`base_env`], [`Captured`], [`run`],
//! [`run_shell`], [`run_handler`], [`Observed`], [`observe`], [`Verdict`],
//! [`Mismatch`], [`judge`], [`commands_for`], [`resolve_hook`], [`TModule`].

mod environment;
mod hooks;
mod module;
mod payload;
mod project;
mod semantics;
mod spawn;
mod verdict;

pub use environment::base_env;
pub use hooks::{commands_for, resolve as resolve_hook};
pub use module::TModule;
pub use payload::{default_payload, merge, substitute_project};
pub(crate) use project::copy_tree;
pub use project::{Project, overlay_into};
pub use semantics::{Observed, observe};
pub use spawn::{Captured, DEFAULT_TIMEOUT, run, run_handler, run_shell};
pub use verdict::{Mismatch, Verdict, judge};
