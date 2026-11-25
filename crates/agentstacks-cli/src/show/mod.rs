//! Text renderers: library values in, the text the commands print out.
//!
//! # Responsibilities
//!
//! - [`diff_lines`] is the unified diff of an updated shared file.
//! - [`doctor_text`] is the doctor report.
//! - [`update_text`] is the update list, and [`has_newer`] says whether it moves the `api` or names
//!   a newer tag.
//! - [`plan_text`] is the plan, [`nothing_to_change`] says when it has nothing to apply, and
//!   [`written_and_deleted`] counts what applying it writes and deletes.
//!
//! # Non-responsibilities
//!
//! Running a flow, reading a file and writing to a stream belong to the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

mod diff;
mod doctor;
mod plan;
mod update;

pub(crate) use diff::diff_lines;
pub(crate) use doctor::doctor_text;
pub(crate) use plan::{nothing_to_change, plan_text, written_and_deleted};
pub(crate) use update::{has_newer, update_text};
