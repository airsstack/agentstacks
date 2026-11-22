//! The result of an extension's `values` handler, read and used.
//!
//! The extension says where each skill is installed, how an agent and a var are written, and
//! under which root its files live. The core checks that answer once and then looks tokens up in
//! it.
//!
//! # Responsibilities
//!
//! - [`ValuesResult`] and [`SkillValues`]: the answer, read from the table the host received
//!   and refused key by key when it is wrong.
//! - [`TokenValue`]: what the answer gives for a token, looked up from the answer.
//!
//! # Non-responsibilities
//!
//! Running the handler, and a handler that returned nothing or something that is not a table,
//! belong to the extension host.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod lookup;
pub mod read;

#[doc(inline)]
pub use lookup::TokenValue;
#[doc(inline)]
pub use read::SkillValues;
#[doc(inline)]
pub use read::ValuesResult;
