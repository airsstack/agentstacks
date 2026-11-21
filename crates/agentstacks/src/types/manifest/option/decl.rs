//! One option a pack declares.
//!
//! It is separate from `PackManifest` because an option's fields check one another (the default must
//! fit the type), a rule of the declaration that the manifest should not carry beside its own
//! fields.
//!
//! # Responsibilities
//!
//! - [`OptionDecl`] holds an option's type, its optional default and its description.
//! - `OptionDecl::read` reads one `[options.<name>]` table: `type` and `description` are
//!   required, `default` is optional and its TOML type must be the declared type.
//! - The default is an [`OptionValue`], already checked against the type.
//!
//! # Non-responsibilities
//!
//! The option's name belongs to the code that reads the `[options]` table. Resolving a value for an
//! option from the project file belongs to the checks over a whole set of packs.

use crate::types::format::TomlFields;
use crate::types::manifest::OptionType;
use crate::types::manifest::OptionValue;
use crate::types::manifest::error::ManifestError;

/// A declared option.
///
/// # Examples
///
/// Declarations are read from the `[options]` table of a
/// [`PackManifest`](crate::types::PackManifest):
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{OptionType, OptionValue, PackManifest};
///
/// let text = r#"
/// [pack]
/// name = "core"
/// version = "1.0.0"
/// api = 0
/// description = "Shared skills"
///
/// [options.retries]
/// type = "integer"
/// default = 3
/// description = "How many times to retry"
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let (name, option) = manifest.options().first().ok_or("one option")?;
/// assert_eq!(name.as_str(), "retries");
/// assert_eq!(option.option_type(), OptionType::Integer);
/// assert_eq!(option.default(), Some(&OptionValue::Integer(3)));
/// assert_eq!(option.description(), "How many times to retry");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A default of another type than the declared one is refused:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::PackManifest;
///
/// let text = r#"
/// [pack]
/// name = "core"
/// version = "1.0.0"
/// api = 0
/// description = "Shared skills"
///
/// [options.retries]
/// type = "integer"
/// default = "three"
/// description = "How many times to retry"
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/pack.toml: options.retries.default: expects integer, got string"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionDecl {
    option_type: OptionType,
    default: Option<OptionValue>,
    description: String,
}

impl OptionDecl {
    /// Reads the `[options.<name>]` table behind `fields`, refusing what is wrong in it.
    ///
    /// Returns the declaration when `type`, `default` and `description` are valid; an unknown key
    /// is refused without withholding it.
    pub(crate) fn read(fields: &mut TomlFields) -> Option<Self> {
        let option_type = fields.required_text("type").and_then(|text| {
            OptionType::parse(&text)
                .map_err(|error| fields.refuse("type", ManifestError::from(error)))
                .ok()
        });
        let written = fields.raw("default").cloned();
        let mut default = None;
        let mut default_fits = true;
        if let Some(value) = &written {
            let got = TomlFields::type_word(value);
            match option_type {
                Some(declared) if got != declared.type_word() => {
                    default_fits = false;
                    fields.refuse(
                        "default",
                        ManifestError::DefaultType {
                            expected: declared.type_word(),
                            got,
                        },
                    );
                }
                _ => default = OptionValue::from_toml(value),
            }
        }
        let description = fields.required_text("description");
        fields.refuse_unknown_keys(&["type", "default", "description"]);
        match (option_type, description) {
            (Some(option_type), Some(description)) if default_fits => Some(Self {
                option_type,
                default,
                description,
            }),
            _ => None,
        }
    }

    /// The declared type.
    #[must_use]
    pub const fn option_type(&self) -> OptionType {
        self.option_type
    }

    /// The default, a value of the declared type, when the option has one.
    #[must_use]
    pub const fn default(&self) -> Option<&OptionValue> {
        self.default.as_ref()
    }

    /// What the option is for.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn read(text: &str) -> Result<(Option<OptionDecl>, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "options.archive_keep", table);
        let decl = OptionDecl::read(&mut fields);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((decl, shown))
    }

    #[test]
    fn option_decl_parses_each_type() -> TestResult {
        let (decl, shown) = read(
            "type = \"integer\"\ndefault = 20\ndescription = \"Discussions kept per project besides the current one\"\n",
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        let decl = decl.ok_or("integer option")?;
        assert_eq!(decl.option_type(), OptionType::Integer);
        assert_eq!(decl.default(), Some(&OptionValue::Integer(20)));
        assert_eq!(
            decl.description(),
            "Discussions kept per project besides the current one"
        );

        let (decl, shown) = read("type = \"boolean\"\ndefault = true\ndescription = \"d\"\n")?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(
            decl.ok_or("boolean option")?.default(),
            Some(&OptionValue::Boolean(true))
        );

        let (decl, shown) = read("type = \"string\"\ndescription = \"d\"\n")?;
        assert!(shown.is_empty(), "{shown:?}");
        let decl = decl.ok_or("string option")?;
        assert_eq!(decl.option_type(), OptionType::String);
        assert_eq!(decl.default(), None);
        Ok(())
    }

    #[test]
    fn option_decl_refuses_each_rule() -> TestResult {
        let cases = [
            (
                "type = \"integer\"\ndefault = \"20\"\ndescription = \"d\"\n",
                "packs/core/pack.toml: options.archive_keep.default: expects integer, got string",
            ),
            (
                "type = \"integer\"\ndefault = 1.5\ndescription = \"d\"\n",
                "packs/core/pack.toml: options.archive_keep.default: expects integer, got float",
            ),
            (
                "type = \"integer\"\ndescription = \"\"\n",
                "packs/core/pack.toml: options.archive_keep.description: empty",
            ),
            (
                "type = \"integer\"\n",
                "packs/core/pack.toml: options.archive_keep.description: required",
            ),
            (
                "description = \"d\"\n",
                "packs/core/pack.toml: options.archive_keep.type: required",
            ),
            (
                "type = \"integer\"\ndescription = \"d\"\nfoo = 1\n",
                "packs/core/pack.toml: options.archive_keep.foo: unknown key",
            ),
            (
                "type = \"number\"\ndescription = \"d\"\n",
                "packs/core/pack.toml: options.archive_keep.type: \"number\" (expected boolean, integer, string)",
            ),
        ];
        for (text, message) in cases {
            let (decl, shown) = read(text)?;
            assert_eq!(shown, [message], "{text}");
            // An unknown key is refused but leaves the three known fields readable.
            assert_eq!(decl.is_some(), message.ends_with("unknown key"), "{text}");
        }
        Ok(())
    }
}
