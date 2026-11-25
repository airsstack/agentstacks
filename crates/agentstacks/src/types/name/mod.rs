//! Names that parse on construction: pack, tool, item, option, binary and environment names.
//!
//! A held name is always valid, so no other module re-checks the naming rules.
//!
//! # Responsibilities
//!
//! - [`PackName`], [`ToolName`], [`ItemName`]: `kebab-case` names of packs, tools and pack items.
//! - [`OptionName`]: a `snake_case` pack option name.
//! - [`BinaryName`]: the name of a binary a pack requires or a hook may run, and [`AIRSL`], the
//!   name of `airsl`.
//! - [`EnvName`]: an environment variable name a hook may read.
//! - [`InvalidName`], [`InvalidEnvName`], [`InvalidOptionName`]: the refusal each constructor
//!   returns, holding the refused value.
//!
//! # Non-responsibilities
//!
//! Whether a named pack, item or option exists belongs to the checks over a whole pack.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod binary;
pub mod env;
pub mod invalid;
pub mod item;
mod kebab;
pub mod option;
pub mod pack;
pub mod tool;

#[doc(inline)]
pub use binary::AIRSL;
#[doc(inline)]
pub use binary::BinaryName;
#[doc(inline)]
pub use env::EnvName;
#[doc(inline)]
pub use invalid::InvalidEnvName;
#[doc(inline)]
pub use invalid::InvalidName;
#[doc(inline)]
pub use invalid::InvalidOptionName;
#[doc(inline)]
pub use item::ItemName;
#[doc(inline)]
pub use option::OptionName;
#[doc(inline)]
pub use pack::PackName;
#[doc(inline)]
pub use tool::ToolName;
