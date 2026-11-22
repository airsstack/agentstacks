//! The five commands, one module each, and what they share.
//!
//! # Responsibilities
//!
//! - `plan`, `apply`, `doctor`, `update` and `init` each run one subcommand: they call the library
//!   flow, print through [`show`](crate::show) and return an [`Outcome`](crate::exit::Outcome).
//! - `confirm` is the show, ask and apply sequence that `apply`, `init` and `update` share, and
//!   `followup` adds the lines `init` and `update` print after they rewrite `agentstacks.toml`.
//! - [`Ports`] holds the four ports a command works through, and [`Console`] the terminal flag and
//!   the three streams.
//!
//! # Non-responsibilities
//!
//! Every command is generic over its ports and streams, so none opens a file, a process or a
//! stream itself: [`wire`](crate::wire) and `main` pass the real ones. A rule belongs to the
//! library, and the clap grammar to [`cli`](crate::cli).

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

mod apply;
mod confirm;
mod console;
mod doctor;
mod followup;
mod init;
mod plan;
mod ports;
#[cfg(test)]
mod testing;
mod update;

pub(crate) use apply::run as apply;
pub(crate) use console::Console;
pub(crate) use doctor::run as doctor;
pub(crate) use init::run as init;
pub(crate) use plan::run as plan;
pub(crate) use ports::Ports;
pub(crate) use update::run as update;
