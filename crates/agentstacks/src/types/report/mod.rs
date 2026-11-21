//! Where and what of every refusal: a location, the message shapes and the sorted collection a run
//! reports.
//!
//! Every format reports through one diagnostic type, so callers sort and print refusals the same
//! way.
//!
//! # Responsibilities
//!
//! - [`Location`]: a file plus at most one line or key path, displayed the same way everywhere.
//! - [`TypeWord`]: the closed set of type names a wrong-type refusal may use.
//! - [`FieldError`]: the four generic field messages (required, wrong type, empty, unknown key).
//! - [`FormatError`]: the one error type a diagnostic carries, joining every module's errors.
//! - [`Diagnostic`]: one refusal, a [`Location`] paired with a [`FormatError`].
//! - [`Diagnostics`]: the collection of refusals a run found, sorted in one stable order.
//!
//! # Non-responsibilities
//!
//! Finding what to refuse belongs to the parsers and checks; this module only carries the result.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod diagnostic;
pub mod diagnostics;
pub mod field;
pub mod format;
pub mod location;
pub mod word;

#[doc(inline)]
pub use diagnostic::Diagnostic;
#[doc(inline)]
pub use diagnostics::Diagnostics;
#[doc(inline)]
pub use field::FieldError;
#[doc(inline)]
pub use format::FormatError;
#[doc(inline)]
pub use location::Location;
#[doc(inline)]
pub use word::TypeWord;
