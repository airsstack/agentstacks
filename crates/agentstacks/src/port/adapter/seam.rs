//! The port to the engine that runs a tool extension.
//!
//! Pure trait definition with no executable logic, so it carries no inline tests.
//!
//! # Responsibilities
//!
//! - [`Adapter`] opens one extension, asks it where its files go, and asks it for its operations.
//!   The engine of one extension lives across the three calls, because running the entry file is
//!   what registers the handlers the later calls dispatch to.
//!
//! # Non-responsibilities
//!
//! It validates nothing beyond what each method says. Rendering the payloads, ordering the tools
//! and deciding the step for each operation belong to the flows and the rules; running Lua belongs
//! to the `host` adapter.

use crate::port::tree::FileTree;
use crate::port::workspace::Workspace;
use crate::types::extension::Extension;
use crate::types::op::{PlanInput, Proposals, ValuesInput, ValuesResult};
use crate::types::plan::Reads;
use crate::types::report::Diagnostics;

/// The engine that runs tool extensions.
///
/// A refusal is a [`Diagnostics`] located as the extension's own files and the project file are;
/// any refusal refuses the whole plan.
pub trait Adapter {
    /// One extension's live engine, from [`Adapter::open`] to the last [`Adapter::plan`].
    type Session;

    /// Loads the `.lua` files of `extension` from `files`, the tree the extension was loaded
    /// from, and runs its entry file at the top level, which registers its handlers.
    ///
    /// # Errors
    ///
    /// The refusals found while loading and running the entry file.
    fn open<T: FileTree>(
        &self,
        extension: &Extension,
        files: &T,
    ) -> Result<Self::Session, Diagnostics>;

    /// Dispatches the `values` handler with `input` and returns its checked answer.
    ///
    /// # Errors
    ///
    /// The refusals found in the handler or in the answer it gave.
    fn values(
        &self,
        session: &mut Self::Session,
        input: &ValuesInput,
    ) -> Result<ValuesResult, Diagnostics>;

    /// Dispatches the `plan` handler with `input`.
    ///
    /// Each accepted operation is added to `proposals`, which already holds the writes of the core
    /// and the operations of every earlier tool, so a path proposed twice is refused naming who
    /// proposed it first. `workspace` serves the `read` calls of the handler. The result records
    /// what each `read` returned.
    ///
    /// # Errors
    ///
    /// Every refusal found while the handler ran, in the order found.
    fn plan<W: Workspace>(
        &self,
        session: &mut Self::Session,
        input: &PlanInput,
        workspace: &W,
        proposals: &mut Proposals,
    ) -> Result<Reads, Diagnostics>;
}
