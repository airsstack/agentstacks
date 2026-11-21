//! The `pack.toml` of a pack, read into one checked manifest.
//!
//! Every rule that needs only the text of `pack.toml` lives here, apart from the checks that need
//! the pack's files or the other packs, so a manifest can be parsed and refused without a
//! directory.
//!
//! # Responsibilities
//!
//! - [`PackManifest`]: reads the whole file into one value, or reports every refusal of it at once.
//! - [`ManifestError`]: one variant per refusal wording a manifest field can meet.
//! - [`Requires`]: the `[requires]` table: required packs, the airsl range and required binaries.
//! - [`AirslReq`]: a `requires.airsl` range whose operators are all supported, kept as written.
//! - [`PackPath`]: a path that stays inside the pack, used by `render` and by hook scripts.
//! - [`ExcludeGlob`]: a relative glob with its compiled matcher, used by `exclude`.
//! - [`OptionDecl`]: one `[options.<name>]` declaration: type, default and description.
//! - [`OptionType`]: the three option types, `boolean`, `integer` and `string`.
//! - [`OptionValue`]: a value an option can hold.
//! - [`PackVersion`]: a semantic version, used for packs, extensions and the locked airsl.
//! - [`InvalidVersion`], [`InvalidAirslReq`], [`InvalidPackPath`], [`InvalidExcludeGlob`],
//!   [`InvalidOptionType`]: the refusal each of those constructors returns.
//!
//! # Non-responsibilities
//!
//! A `render` file existing, an `exclude` protecting a `SKILL.md` and `requires.airsl` being
//! present when the pack holds Lua need the pack's files, so the pack loader owns them. Rules
//! across several packs belong to the set checks. The `[[hooks]]` tables are read here but defined
//! by [`hooks`](mod@crate::types::hooks).
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod airsl;
pub mod error;
pub mod exclude;
pub mod invalid;
pub mod option;
pub mod pack;
pub mod requires;

#[doc(inline)]
pub use airsl::AirslReq;
#[doc(inline)]
pub use error::ManifestError;
#[doc(inline)]
pub use exclude::ExcludeGlob;
#[doc(inline)]
pub use invalid::InvalidAirslReq;
#[doc(inline)]
pub use invalid::InvalidExcludeGlob;
#[doc(inline)]
pub use invalid::InvalidOptionType;
#[doc(inline)]
pub use invalid::InvalidPackPath;
#[doc(inline)]
pub use invalid::InvalidVersion;
#[doc(inline)]
pub use option::OptionDecl;
#[doc(inline)]
pub use option::OptionType;
#[doc(inline)]
pub use option::OptionValue;
#[doc(inline)]
pub use pack::PackManifest;
#[doc(inline)]
pub use pack::PackPath;
#[doc(inline)]
pub use pack::PackVersion;
#[doc(inline)]
pub use requires::Requires;
