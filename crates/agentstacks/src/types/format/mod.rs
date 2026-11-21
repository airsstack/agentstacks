//! Generic readers for TOML and YAML text: a raw TOML table with typed field access by key path and
//! an own YAML value type.
//!
//! Every manifest and frontmatter reader builds on the same parsing, so every error of a file is
//! reported with its location.
//!
//! # Responsibilities
//!
//! - `document`: parses TOML text into a raw table or one syntax diagnostic.
//! - `fields`: typed reads over that table that record every refusal by key path.
//! - [`YamlValue`]: a generic YAML node that frontmatter checks walk by hand.
//!
//! The items of the two TOML modules are `pub(crate)` because they hand out `toml` types.
//!
//! # Non-responsibilities
//!
//! Which keys a file may hold, and what each means, belongs to the types that read that file.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod document;
pub mod fields;
pub mod yaml;

#[doc(inline)]
pub use yaml::YamlValue;

pub(crate) use document::parse_toml_document;
pub(crate) use fields::TomlFields;
