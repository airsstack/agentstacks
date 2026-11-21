//! Tokens in pack text: the vocabulary, scanning text into located tokens and rendering values over
//! them.
//!
//! Token syntax is one rule set shared by every text a pack holds, so it is read in one place.
//!
//! # Responsibilities
//!
//! - [`Token`], [`TokenKind`], [`TokenTarget`]: one `{{...}}` found in text, its kind and what it
//!   points at.
//! - [`Var`], [`ItemRef`]: the fixed variables and the `[pack/]item` references a target may hold.
//! - [`TokenError`]: the grammar refusals, each with its exact message.
//! - [`LocatedToken`], [`TextOrigin`]: a token with where it was found, and how a text's position
//!   is reported.
//! - [`scan()`]: the one function that reads text into located tokens and the [`Diagnostics`](crate::types::Diagnostics) of the refusals.
//! - [`names_own_pack()`]: the one rule for whether a pack named in a token or hook `run` is the pack it sits in.
//! - [`TokenValues`], [`render()`]: the replacement text per token, and the splice of it into text.
//!
//! # Non-responsibilities
//!
//! Where a token may appear and what it may point at belong to the checks over a whole pack and a
//! whole set of packs.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod error;
pub mod kind;
pub mod located;
pub mod origin;
pub mod own;
pub mod parsed;
pub mod reference;
pub mod render;
pub mod scan;
pub mod target;
pub mod values;
pub mod var;

#[doc(inline)]
pub use error::TokenError;
#[doc(inline)]
pub use kind::TokenKind;
#[doc(inline)]
pub use located::LocatedToken;
#[doc(inline)]
pub use origin::TextOrigin;
#[doc(inline)]
pub use own::names_own_pack;
#[doc(inline)]
pub use parsed::Token;
#[doc(inline)]
pub use reference::ItemRef;
#[doc(inline)]
pub use render::render;
#[doc(inline)]
pub use scan::scan;
#[doc(inline)]
pub use target::TokenTarget;
#[doc(inline)]
pub use values::TokenValues;
#[doc(inline)]
pub use var::Var;
