//! The shared directory walker and the SHA-256 hashes of file contents and of whole trees.
//!
//! The walk and the tree hash must see exactly the same files, so both go through one walker and
//! one path type.
//!
//! # Responsibilities
//!
//! - [`walk()`]: lists the regular files of a tree under the directory rules, with every refusal it met.
//! - [`Walked`]: what the walk found, the files, the ignored dot entries and the refused entries.
//! - [`WalkError`]: the refusals the walk raises, a symlink, a special file and a path that is not UTF-8.
//! - [`TreePath`]: a `/`-joined UTF-8 path relative to the walked directory, ordered by its bytes.
//! - [`ContentHash`]: a `sha256:` digest in the form the lock pins, parsed from text or computed from bytes.
//! - [`ContentHashError`]: the refusal for text that is not a content hash.
//! - [`tree_hash()`]: the deterministic hash of every file path and content under a tree.
//! - `hash_walked`: the same hash over a walk the caller already holds, each file read once.
//!
//! # Non-responsibilities
//!
//! Reading the real disk belongs to the `fs` module, which answers the port the walk reads
//! through. Judging a file's place in a pack belongs to the pack loader, and comparing a tree hash
//! with the lock belongs to the stage that plans and applies an install.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod hash;
pub mod path;
pub mod walk;

#[doc(inline)]
pub use hash::ContentHash;
#[doc(inline)]
pub use hash::ContentHashError;
#[doc(inline)]
pub use hash::tree_hash;
#[doc(inline)]
pub use path::TreePath;
#[doc(inline)]
pub use walk::WalkError;
#[doc(inline)]
pub use walk::Walked;
#[doc(inline)]
pub use walk::walk;

pub(crate) use hash::hash_walked;
