//! The `[options.<name>]` declarations of a pack: their type, default and the values they hold.
//!
//! An option is declared once in `pack.toml`, so its declaration, its three types and the values
//! it can hold sit together and the manifest reads them as one unit.
//!
//! # Responsibilities
//!
//! - [`OptionDecl`]: one `[options.<name>]` declaration: type, default and description.
//! - [`OptionType`]: the three option types, `boolean`, `integer` and `string`.
//! - [`OptionValue`]: a value an option can hold.
//!
//! # Non-responsibilities
//!
//! Typing the values a project file gives an option against its declaration needs the loaded
//! packs, so the checks over those packs own it.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod decl;
pub mod kind;
pub mod value;

#[doc(inline)]
pub use decl::OptionDecl;
#[doc(inline)]
pub use kind::OptionType;
#[doc(inline)]
pub use value::OptionValue;
