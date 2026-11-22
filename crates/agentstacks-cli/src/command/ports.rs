//! The four ports a command works through.
//!
//! # Responsibilities
//!
//! - [`Ports`] holds the workspace, git, binary and adapter a command passes to the library
//!   flows. `wire` builds it from the real adapters and a test builds it from the library fakes.
//!
//! # Non-responsibilities
//!
//! Choosing which adapter is real belongs to `wire`.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

/// A workspace, a git, a binary lookup and an extension adapter.
#[derive(Debug)]
pub(crate) struct Ports<W, G, B, A> {
    /// The project's files, `agentstacks.toml` and `agentstacks.lock`.
    pub(crate) workspace: W,
    /// The remotes of git sources.
    pub(crate) git: G,
    /// The binaries packs and extensions need, and `airsl`.
    pub(crate) binary: B,
    /// The engine that runs an extension.
    pub(crate) adapter: A,
}
