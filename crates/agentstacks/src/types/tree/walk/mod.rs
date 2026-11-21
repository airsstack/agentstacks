//! The shared directory walker, what it found and the refusals it raises.
//!
//! The walk and the tree hash must see exactly the same files, so both go through this one walker.
//!
//! # Responsibilities
//!
//! - [`walk()`]: lists the regular files of a tree under the directory rules, with every refusal
//!   it met.
//! - [`Walked`]: what the walk found, the files, the ignored dot entries and the refused entries.
//! - [`WalkError`]: the refusals the walk raises, a symlink, a special file and a path that is not
//!   UTF-8.
//!
//! # Non-responsibilities
//!
//! Reading the real disk belongs to the `fs` module, and judging a file's place in a pack belongs
//! to the pack loader.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod error;
pub mod outcome;
pub mod run;

#[doc(inline)]
pub use error::WalkError;
#[doc(inline)]
pub use outcome::Walked;
#[doc(inline)]
pub use run::walk;
