//! The values that name a point in a git repository: its URL, a tag and a commit id.
//!
//! A git source is fixed by these three values, so each is checked once here and never as a bare
//! string.
//!
//! # Responsibilities
//!
//! - [`GitUrl`]: a repository URL that cannot be taken for a command-line option.
//! - [`GitTag`]: a non-empty git tag name.
//! - [`CommitId`]: a full-length lowercase hex commit id.
//!
//! # Non-responsibilities
//!
//! Whether a URL, tag or commit exists is decided when it is fetched.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod commit;
pub mod tag;
pub mod url;

#[doc(inline)]
pub use commit::CommitId;
#[doc(inline)]
pub use tag::GitTag;
#[doc(inline)]
pub use url::GitUrl;
