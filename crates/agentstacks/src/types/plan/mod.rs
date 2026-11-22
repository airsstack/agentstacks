//! The step for each path, decided from the operation, the lock and the state of the disk.
//!
//! # Responsibilities
//!
//! - [`StepError`]: every refusal of a file step.
//! - [`DiskFile`], [`DiskState`], [`Link`] and [`Resolution`]: the facts about a path on disk.
//! - [`Action`] and [`Step`]: what happens to a path.
//! - [`ReadOutcome`] and [`Reads`]: what a tool's reads returned, and which paths are shared.
//! - [`decide_proposed`] and [`decide_unproposed`]: the step for a proposed path and for a locked
//!   path nothing proposes now.
//! - [`keep_shared_writes`]: the proposals a removed tool may still make.
//! - [`lock_contents`], [`Landed`] and [`airsl_pin`]: the new lock, in full or after a failed
//!   swap.
//! - [`Plan`], [`BinaryCheck`], [`Skipped`], [`Note`] and [`AirslPin`]: the whole plan.
//! - [`AirslAnswer`], [`Finding`], [`file_finding`] and [`airsl_finding`]: the drift check against
//!   the lock.
//! - [`race()`](fn@race) and [`Race`]: the check apply makes before it acts.
//!
//! # Non-responsibilities
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod action;
pub mod airsl;
pub mod binary;
pub mod built;
pub mod decide;
pub mod disk;
pub mod drift;
pub mod error;
pub mod finding;
pub mod lock;
pub mod note;
pub mod race;
pub mod reads;
pub mod removed;
pub mod skipped;
pub mod step;

#[doc(inline)]
pub use action::Action;
#[doc(inline)]
pub use airsl::AirslAnswer;
#[doc(inline)]
pub use airsl::Exit;
#[doc(inline)]
pub use binary::BinaryCheck;
#[doc(inline)]
pub use built::Plan;
#[doc(inline)]
pub use decide::ProposedPath;
#[doc(inline)]
pub use decide::decide_proposed;
#[doc(inline)]
pub use decide::decide_unproposed;
#[doc(inline)]
pub use disk::DiskFile;
#[doc(inline)]
pub use disk::DiskState;
#[doc(inline)]
pub use disk::Link;
#[doc(inline)]
pub use disk::Resolution;
#[doc(inline)]
pub use drift::airsl_finding;
#[doc(inline)]
pub use drift::file_finding;
#[doc(inline)]
pub use error::StepError;
#[doc(inline)]
pub use finding::Finding;
#[doc(inline)]
pub use lock::Landed;
#[doc(inline)]
pub use lock::airsl_pin;
#[doc(inline)]
pub use lock::lock_contents;
#[doc(inline)]
pub use note::AirslPin;
#[doc(inline)]
pub use note::Note;
#[doc(inline)]
pub use race::Race;
#[doc(inline)]
pub use race::race;
#[doc(inline)]
pub use reads::ReadOutcome;
#[doc(inline)]
pub use reads::Reads;
#[doc(inline)]
pub use removed::keep_shared_writes;
#[doc(inline)]
pub use skipped::Skipped;
#[doc(inline)]
pub use step::Step;
