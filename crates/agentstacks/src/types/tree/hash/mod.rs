//! The SHA-256 digest of file contents and the deterministic hash of a whole tree.
//!
//! The digest form is parsed once, and the tree hash combines digests in one format the lock pins,
//! so both live together.
//!
//! # Responsibilities
//!
//! - [`ContentHash`]: a `sha256:` digest in the form the lock pins, parsed from text or computed
//!   from bytes.
//! - [`ContentHashError`]: the refusal for text that is not a content hash.
//! - [`tree_hash()`]: the deterministic hash of every file path and content under a tree.
//!
//! # Non-responsibilities
//!
//! Comparing a hash with the lock belongs to the stage that plans and applies an install.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod content;
pub mod error;
pub mod tree;

#[doc(inline)]
pub use content::ContentHash;
#[doc(inline)]
pub use error::ContentHashError;
#[doc(inline)]
pub use tree::tree_hash;
