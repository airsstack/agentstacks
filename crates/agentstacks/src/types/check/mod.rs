//! The pure rules behind `agentstacks ext test`: what the scratch project holds, whether an
//! extension has Lua tests, and whether the check passed.
//!
//! The command checks an extension by installing the built-in conformance pack, and any packs the
//! author names, into a temporary project, planning, applying and planning again. Each decision on
//! the way is a rule over data in hand, so it lives here and the flow that calls it stays a
//! sequence of steps.
//!
//! # Responsibilities
//!
//! - [`conformance`]: the conformance pack, embedded in the binary, and the [`NAME`] it is listed
//!   under.
//! - [`CheckError`]: the refusals of the run itself, `airsl` missing and `airsl test` that could
//!   not start.
//! - [`stage()`]: the files of the scratch project, its `agentstacks.toml` text and the rules for
//!   the packs the author names ([`GivenPack`]), as [`StagedFile`]s and [`StageError`] refusals.
//! - [`has_tests()`] and [`summary()`]: whether a walked extension holds a Lua test file, and which
//!   line of an `airsl test` run to show.
//! - [`second_plan()`]: whether a second plan has nothing to change, with the [`NotRepeatable`]
//!   refusal when it has, and the [`Verdict`] a run ends with.
//!
//! # Non-responsibilities
//!
//! Opening directories, writing the scratch project and running `airsl` belong to the adapters
//! behind the ports. Putting the steps in order and printing them belong to the flow in `app` and
//! to the command line.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod conformance;
pub mod error;
pub mod lua;
pub mod stage;
pub mod verdict;

#[doc(inline)]
pub use conformance::CONFORMANCE;
#[doc(inline)]
pub use conformance::NAME;
#[doc(inline)]
pub use error::CheckError;
#[doc(inline)]
pub use lua::has_tests;
#[doc(inline)]
pub use lua::summary;
#[doc(inline)]
pub use stage::GivenPack;
#[doc(inline)]
pub use stage::StageError;
#[doc(inline)]
pub use stage::StagedFile;
#[doc(inline)]
pub use stage::pack_name;
#[doc(inline)]
pub use stage::stage;
#[doc(inline)]
pub use verdict::NotRepeatable;
#[doc(inline)]
pub use verdict::Verdict;
#[doc(inline)]
pub use verdict::second_plan;
