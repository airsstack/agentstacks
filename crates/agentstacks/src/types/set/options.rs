//! Resolving the option values a project gives one pack.
//!
//! It is separate from the manifest because a declaration lives in `pack.toml` and its value in the
//! project file, so only a check that sees both can resolve them.
//!
//! # Responsibilities
//!
//! - `resolve_options` gives every option a pack declares its value: the value the project file
//!   sets when it has the declared type, else the option's default.
//! - It refuses, all at once, an option with no value and no default, a value of the wrong type,
//!   and a value for an option the pack does not declare. A required option that is not set is
//!   located at the pack's entry in the project file; the other two at the option's key under it.
//!
//! # Non-responsibilities
//!
//! Reading the `options` table out of the project file belongs to
//! [`ProjectFile`](crate::types::ProjectFile), and declaring the options to the pack manifest.
//! Splicing a value into text belongs to token rendering.

use std::collections::BTreeMap;
use std::path::Path;

use toml::Table;

use crate::types::format::TomlFields;
use crate::types::manifest::OptionDecl;
use crate::types::manifest::OptionValue;
use crate::types::name::OptionName;
use crate::types::name::PackName;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;

/// The value of every option of a pack.
pub(crate) type ResolvedOptions = BTreeMap<OptionName, OptionValue>;

/// The values of the options `decls` that the pack listed as `key` declares, given the `set` table
/// of its `[packs.<key>.options]` in the project file at `project_file`.
///
/// # Errors
///
/// Returns every refusal when any option does not resolve.
pub(crate) fn resolve_options(
    project_file: &Path,
    key: &PackName,
    decls: &[(OptionName, OptionDecl)],
    set: &Table,
) -> Result<ResolvedOptions, Vec<Diagnostic>> {
    let at = |path: String| Location::set_level(project_file, Some(path));
    let mut resolved = ResolvedOptions::new();
    let mut refusals = Vec::new();
    for (name, decl) in decls {
        let Some(written) = set.get(name.as_str()) else {
            match decl.default() {
                Some(default) => {
                    resolved.insert(name.clone(), default.clone());
                }
                None => refusals.push(Diagnostic::new(
                    at(format!("packs.{key}")),
                    SetError::OptionRequired {
                        option: name.clone(),
                    },
                )),
            }
            continue;
        };
        match OptionValue::from_toml(written).filter(|v| v.option_type() == decl.option_type()) {
            Some(value) => {
                resolved.insert(name.clone(), value);
            }
            None => refusals.push(Diagnostic::new(
                at(format!("packs.{key}.options.{name}")),
                SetError::OptionType {
                    option: name.clone(),
                    expected: decl.option_type().type_word(),
                    got: TomlFields::type_word(written),
                },
            )),
        }
    }
    for written in set.keys() {
        if !decls.iter().any(|(name, _)| name.as_str() == written) {
            refusals.push(Diagnostic::new(
                at(format!("packs.{key}.options.{written}")),
                SetError::OptionUndeclared {
                    key: written.clone(),
                },
            ));
        }
    }
    if refusals.is_empty() {
        Ok(resolved)
    } else {
        Err(refusals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PROJECT_FILE, PackSpec, name, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    /// The options of `core`, which declares `archive_keep` (integer, default 20), `x` (string,
    /// required) and `label` (string, default `none`), resolved against the project's `set` text.
    fn resolve(set: &str) -> Result<Result<ResolvedOptions, Vec<String>>, Box<dyn Error>> {
        let pack = PackSpec::new("core")
            .integer_option("archive_keep", 20)
            .required_string_option("x")
            .tables("[options.label]\ntype = \"string\"\ndefault = \"none\"\ndescription = \"d\"\n")
            .load()?;
        let table: Table = set.parse()?;
        Ok(resolve_options(
            Path::new(PROJECT_FILE),
            &name("core")?,
            pack.manifest().options(),
            &table,
        )
        .map_err(|refusals| shown(&refusals)))
    }

    fn value(resolved: &ResolvedOptions, option: &str) -> Result<OptionValue, Box<dyn Error>> {
        Ok(resolved
            .get(&OptionName::new(option)?)
            .ok_or("option has no value")?
            .clone())
    }

    #[test]
    fn set_and_default_values_resolve() -> TestResult {
        let resolved = resolve("archive_keep = 50\nx = \"y\"\n")?.map_err(|r| r.join("; "))?;
        assert_eq!(value(&resolved, "archive_keep")?, OptionValue::Integer(50));
        assert_eq!(value(&resolved, "x")?, OptionValue::String("y".to_owned()));
        assert_eq!(
            value(&resolved, "label")?,
            OptionValue::String("none".to_owned())
        );
        let defaulted = resolve("x = \"y\"\n")?.map_err(|r| r.join("; "))?;
        assert_eq!(value(&defaulted, "archive_keep")?, OptionValue::Integer(20));
        Ok(())
    }

    #[test]
    fn each_refusal_reads_as_documented() -> TestResult {
        for (set, expected) in [
            ("", "agentstacks.toml: packs.core: option \"x\" is required"),
            (
                "x = \"y\"\narchive_keep = \"many\"\n",
                "agentstacks.toml: packs.core.options.archive_keep: option archive_keep expects integer, got string",
            ),
            (
                "x = \"y\"\narchive_kep = 1\n",
                "agentstacks.toml: packs.core.options.archive_kep: no option \"archive_kep\"",
            ),
        ] {
            assert_eq!(
                resolve(set)?.err(),
                Some(vec![expected.to_owned()]),
                "{set}"
            );
        }
        Ok(())
    }

    #[test]
    fn type_mismatch_names_the_toml_type() -> TestResult {
        assert_eq!(
            resolve("x = \"y\"\narchive_keep = 1.5\n")?.err(),
            Some(vec![
                "agentstacks.toml: packs.core.options.archive_keep: option archive_keep expects integer, got float"
                    .to_owned()
            ])
        );
        assert_eq!(
            resolve("x = true\n")?.err(),
            Some(vec![
                "agentstacks.toml: packs.core.options.x: option x expects string, got boolean"
                    .to_owned()
            ])
        );
        Ok(())
    }

    #[test]
    fn every_refusal_is_reported_at_once() -> TestResult {
        let found = resolve("archive_keep = \"many\"\nstray = 1\n")?.err();
        assert_eq!(
            found,
            Some(vec![
                "agentstacks.toml: packs.core.options.archive_keep: option archive_keep expects integer, got string".to_owned(),
                "agentstacks.toml: packs.core: option \"x\" is required".to_owned(),
                "agentstacks.toml: packs.core.options.stray: no option \"stray\"".to_owned(),
            ])
        );
        Ok(())
    }
}
