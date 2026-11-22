//! The adapter that drives the `git` CLI and looks up binaries on `PATH`, and the only place that
//! calls `std::process`.
//!
//! It translates between the ports and the processes it starts and decides nothing. It is the one
//! adapter allowed to use another adapter's item, `fs::cache`.
//!
//! # Responsibilities
//!
//! - [`GitCli`]: the `Git` port on the `git` command line. Every disk step of the cache is
//!   `fs::Cache`, so the git adapter never calls `std::fs`; only the `PATH` lookup reads file metadata.
//! - [`Snapshot`]: the files of one commit, read into memory and served as a `FileTree`.
//! - [`SystemBinary`]: the `Binary` port on `PATH`, and the answer of `airsl --version`.
//!
//! # Non-responsibilities
//!
//! No rule about what the output means: reading tag lines, tree records and the airsl version,
//! and the cache layout, belong to `types`. Locating a refusal at `agentstacks.toml` belongs to
//! the flows.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod binary;
pub mod git;

#[doc(inline)]
pub use binary::SystemBinary;
#[doc(inline)]
pub use git::GitCli;
#[doc(inline)]
pub use git::Snapshot;
