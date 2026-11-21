//! What a pack needs from the machine and from other packs.
//!
//! It is separate from `PackManifest` because the `[requires]` rules (repeats, self-requirement,
//! binary names) form one table with its own located refusals, and the hook readers need its
//! binaries.
//!
//! # Responsibilities
//!
//! - [`Requires`] holds the packs a pack depends on, the airsl range it needs, and the binaries it
//!   needs on the machine, each already valid.
//! - `Requires::read` reads the `[requires]` table. A pack name is refused when invalid, when
//!   listed twice, and when it is the pack's own name; a binary is refused when invalid or listed
//!   twice. Every refusal about one entry is located at its indexed key path
//!   (`requires.packs[1]`).
//!
//! # Non-responsibilities
//!
//! Whether a required pack is in the set, whether the pack holds `.lua` files and so needs an
//! airsl range, and whether the ranges of the set intersect belong to the checks over a loaded
//! pack and over a whole set of packs.

use crate::types::format::TomlFields;
use crate::types::manifest::AirslReq;
use crate::types::manifest::error::ManifestError;
use crate::types::name::BinaryName;
use crate::types::name::PackName;
use crate::types::report::ListedTwice;

/// The `[requires]` table of a pack manifest.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::PackManifest;
///
/// let text = r#"
/// [pack]
/// name = "sdlc"
/// version = "1.0.0"
/// api = 0
/// description = "Delivery skills"
///
/// [requires]
/// packs = ["core"]
/// airsl = ">= 0.1.4"
/// binaries = ["git"]
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/sdlc/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let requires = manifest.requires();
/// assert_eq!(requires.packs().len(), 1);
/// assert_eq!(requires.airsl().map(|range| range.written()), Some(">= 0.1.4"));
/// assert_eq!(requires.binaries().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A pack that lists itself is refused at the entry:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::PackManifest;
///
/// let text = r#"
/// [pack]
/// name = "sdlc"
/// version = "1.0.0"
/// api = 0
/// description = "Delivery skills"
///
/// [requires]
/// packs = ["sdlc"]
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/sdlc/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/sdlc/pack.toml: requires.packs[0]: a pack cannot require itself"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Requires {
    packs: Vec<PackName>,
    airsl: Option<AirslReq>,
    binaries: Vec<BinaryName>,
}

impl Requires {
    /// Reads the `[requires]` table behind `fields`, refusing what is wrong in it.
    ///
    /// `own_name` is the pack's own name when it is valid, so a pack listing itself is refused.
    pub(in crate::types) fn read(fields: &mut TomlFields, own_name: Option<&PackName>) -> Self {
        let mut packs: Vec<PackName> = Vec::new();
        for (index, text) in fields.optional_string_list("packs").unwrap_or_default() {
            let item_path = fields.item_path("packs", index);
            match PackName::new(&text) {
                Err(error) => fields.refuse_path(&item_path, ManifestError::from(error)),
                Ok(name) if packs.contains(&name) => {
                    fields.refuse_path(&item_path, ManifestError::from(ListedTwice::text(text)));
                }
                Ok(name) => {
                    if own_name == Some(&name) {
                        fields.refuse_path(&item_path, ManifestError::RequiresItself);
                    }
                    packs.push(name);
                }
            }
        }
        let airsl = fields
            .optional_string("airsl")
            .and_then(|text| match AirslReq::new(&text) {
                Ok(req) => Some(req),
                Err(error) => {
                    fields.refuse("airsl", ManifestError::from(error));
                    None
                }
            });
        let mut binaries: Vec<BinaryName> = Vec::new();
        for (index, text) in fields.optional_string_list("binaries").unwrap_or_default() {
            let item_path = fields.item_path("binaries", index);
            match BinaryName::new(&text) {
                Err(error) => fields.refuse_path(&item_path, ManifestError::from(error)),
                Ok(name) if binaries.contains(&name) => {
                    fields.refuse_path(&item_path, ManifestError::from(ListedTwice::text(text)));
                }
                Ok(name) => binaries.push(name),
            }
        }
        fields.refuse_unknown_keys(&["packs", "airsl", "binaries"]);
        Self {
            packs,
            airsl,
            binaries,
        }
    }

    /// The packs this pack depends on.
    #[must_use]
    pub fn packs(&self) -> &[PackName] {
        &self.packs
    }

    /// The airsl range this pack needs, when it declares one.
    #[must_use]
    pub const fn airsl(&self) -> Option<&AirslReq> {
        self.airsl.as_ref()
    }

    /// The binaries this pack needs on the machine.
    #[must_use]
    pub fn binaries(&self) -> &[BinaryName] {
        &self.binaries
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "packs/sdlc/pack.toml";

    fn read(
        text: &str,
        own: Option<&PackName>,
    ) -> Result<(Requires, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new(FILE);
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "requires", table);
        let requires = Requires::read(&mut fields, own);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((requires, shown))
    }

    #[test]
    fn requires_parses_a_typical_example() -> TestResult {
        let (requires, shown) = read(
            "packs = [\"core\"]\nairsl = \">=0.1.4\"\nbinaries = [\"git\"]\n",
            None,
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(requires.packs(), [PackName::new("core")?]);
        assert_eq!(
            requires.airsl().map(ToString::to_string),
            Some(">=0.1.4".to_owned())
        );
        assert_eq!(requires.binaries(), [BinaryName::new("git")?]);

        let (none, shown) = read("", None)?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(none.packs().is_empty());
        assert!(none.airsl().is_none());
        assert!(none.binaries().is_empty());
        Ok(())
    }

    #[test]
    fn requires_refuses_its_own_name_once_and_a_repeat_as_listed_twice() -> TestResult {
        let own = PackName::new("sdlc")?;
        let (_, shown) = read("packs = [\"sdlc\", \"sdlc\"]", Some(&own))?;
        assert_eq!(
            shown,
            [
                "packs/sdlc/pack.toml: requires.packs[0]: a pack cannot require itself",
                "packs/sdlc/pack.toml: requires.packs[1]: \"sdlc\" listed twice",
            ]
        );
        Ok(())
    }

    #[test]
    fn requires_refuses_each_rule() -> TestResult {
        let own = PackName::new("sdlc")?;
        let cases = [
            (
                "packs = [\"core\", \"core\"]",
                "packs/sdlc/pack.toml: requires.packs[1]: \"core\" listed twice",
            ),
            (
                "packs = [\"sdlc\"]",
                "packs/sdlc/pack.toml: requires.packs[0]: a pack cannot require itself",
            ),
            (
                "packs = [\"Core\"]",
                "packs/sdlc/pack.toml: requires.packs[0]: \"Core\" is not a valid name",
            ),
            (
                "airsl = \"nope\"",
                "packs/sdlc/pack.toml: requires.airsl: \"nope\" is not a version range",
            ),
            (
                "binaries = [\"git\", \"git\"]",
                "packs/sdlc/pack.toml: requires.binaries[1]: \"git\" listed twice",
            ),
            ("foo = 1", "packs/sdlc/pack.toml: requires.foo: unknown key"),
        ];
        for (text, message) in cases {
            let (_, shown) = read(text, Some(&own))?;
            assert_eq!(shown, [message], "{text}");
        }
        Ok(())
    }
}
