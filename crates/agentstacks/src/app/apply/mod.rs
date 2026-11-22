//! Applying a [`Plan`](crate::types::plan::Plan) that was shown, and the only path to a write.
//!
//! The order of the steps is a flow, so it lives here and each rule it consults lives in
//! `types::plan`. The flow takes the project as a [`Workspace`](crate::port::Workspace) and the plan
//! as a value, so it cannot plan again or reach git, a binary or an extension.
//!
//! # Responsibilities
//!
//! - `stage`: write every new file to a temporary file beside its target, then check that no
//!   target changed or now leads outside the project since the plan, removing the temporary files
//!   when anything is refused.
//! - `swap`: rename the temporary files onto their targets, then delete, stopping at the first
//!   failure and removing the temporary files left.
//! - `lock`: write `agentstacks.lock` last, in full after every step landed and holding only the
//!   steps that landed after a failure.
//! - `flow`: [`apply`], the five steps in order.
//!
//! # Non-responsibilities
//!
//! No planning and no rule: what counts as a change, what the lock records and how a refusal
//! reads belong to `types`. Nothing here calls `std::fs`; the disk is reached through the
//! workspace port.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

mod flow;
mod lock;
mod stage;
mod swap;
#[cfg(test)]
mod testing;

#[doc(inline)]
pub use flow::apply;
