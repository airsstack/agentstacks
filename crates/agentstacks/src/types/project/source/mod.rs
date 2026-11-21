//! Where a pack or tool comes from, the checked keys that describe it and its refusals.
//!
//! The project file and the lock both name a source, so one set of types reads and refuses it for
//! both.
//!
//! # Responsibilities
//!
//! - [`Source`]: a git repository with a pin, or a directory in the project.
//! - `SourceFields` (crate-internal): the checked source keys (`git`, `path`, `tag`, `rev`,
//!   `subdir`) of one entry, read under a pin rule.
//! - [`SourceError`]: the refusals of the source values, wrapping the refusals of their parts.
//!
//! # Non-responsibilities
//!
//! Fetching a source, and rules that need fetched content, belong to the stages after parsing.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod error;
mod fields;
pub mod kind;

#[doc(inline)]
pub use error::SourceError;
pub(in crate::types) use fields::SourceFields;
#[doc(inline)]
pub use kind::Source;
