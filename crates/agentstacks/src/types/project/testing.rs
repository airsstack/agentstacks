//! Shared helper for the tests of the project group.
//!
//! # Responsibilities
//!
//! - [`read_fields`] reads one raw source entry on its own, the way the project and lock readers
//!   read it as part of a larger file, so several tests can drive the source rules directly.
//!
//! # Non-responsibilities
//!
//! It is compiled for tests only and holds no rule.

use std::path::Path;

use toml::Table;

use crate::types::format::TomlFields;
use crate::types::project::PinRule;
use crate::types::project::SourceFields;
use crate::types::report::Diagnostics;

/// Reads the source keys of `entry`, the table at `key_path` in `file`, under `rule`: the fields,
/// or every refusal found.
pub(super) fn read_fields(
    entry: &Table,
    file: &Path,
    key_path: &str,
    rule: PinRule,
) -> Result<SourceFields, Diagnostics> {
    let mut fields = TomlFields::new(file, key_path, entry.clone());
    let source = SourceFields::read_into(&mut fields, key_path, rule);
    source.ok_or_else(|| fields.into_diagnostics())
}
