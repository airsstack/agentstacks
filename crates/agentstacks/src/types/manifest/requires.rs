//! What a pack needs from the machine and from other packs.
//!
//! It is separate from `PackManifest` because the `[requires]` rules (repeats, self-requirement,
//! binary names) form one table with its own located refusals, and the hook readers need its
//! binaries.
//!
//! # Responsibilities
//!
//! - [`Requires`] holds the packs a pack depends on, the agentstacks and airsl ranges it needs, and
//!   the binaries it needs on the machine, each already valid.
//! - `agentstacks_too_new` reads only `requires.agentstacks` out of a manifest's text and
//!   answers the refusal of a range that excludes a given agentstacks version, so a loader can
//!   report it alone before anything else in the file is checked.
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

use std::path::Path;

use semver::Version;

use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::manifest::VersionRange;
use crate::types::manifest::error::ManifestError;
use crate::types::name::BinaryName;
use crate::types::name::PackName;
use crate::types::report::Diagnostic;
use crate::types::report::ListedTwice;
use crate::types::report::Location;

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
/// api = 1
/// description = "Delivery skills"
///
/// [requires]
/// packs = ["core"]
/// airsl = ">= 0.1.4"
/// agentstacks = ">=1.0"
/// binaries = ["git"]
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/sdlc/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let requires = manifest.requires();
/// assert_eq!(requires.packs().len(), 1);
/// assert_eq!(requires.airsl().map(|range| range.written()), Some(">= 0.1.4"));
/// assert_eq!(requires.agentstacks().map(|range| range.written()), Some(">=1.0"));
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
/// api = 1
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
    agentstacks: Option<VersionRange>,
    airsl: Option<VersionRange>,
    binaries: Vec<BinaryName>,
}

/// The refusal of a `requires.agentstacks` in `text` that excludes the `running` agentstacks, or
/// `None` when `text` is not valid TOML, has no such key, holds a value that is not a range, or
/// holds a range that matches `running`. Those other cases are refused by the manifest parser.
pub(in crate::types) fn agentstacks_too_new(
    file: &Path,
    text: &str,
    running: &Version,
) -> Option<Diagnostic> {
    let table = parse_toml_document(file, text).ok()?;
    let written = table.get("requires")?.get("agentstacks")?.as_str()?;
    let range = VersionRange::new(written).ok()?;
    if range.as_version_req().matches(running) {
        return None;
    }
    Some(Diagnostic::new(
        Location::at_key(file, "requires.agentstacks"),
        ManifestError::TooNew {
            range: Box::new(range),
            version: running.clone(),
        },
    ))
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
        let airsl = VersionRange::read::<ManifestError>(fields, "airsl");
        let agentstacks = VersionRange::read::<ManifestError>(fields, "agentstacks");
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
        fields.refuse_unknown_keys(&["packs", "agentstacks", "airsl", "binaries"]);
        Self {
            packs,
            agentstacks,
            airsl,
            binaries,
        }
    }

    /// The packs this pack depends on.
    #[must_use]
    pub fn packs(&self) -> &[PackName] {
        &self.packs
    }

    /// The agentstacks range this pack needs, when it declares one; absent means any version.
    #[must_use]
    pub const fn agentstacks(&self) -> Option<&VersionRange> {
        self.agentstacks.as_ref()
    }

    /// The airsl range this pack needs, when it declares one.
    #[must_use]
    pub const fn airsl(&self) -> Option<&VersionRange> {
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

    fn excluded(text: &str, running: &Version) -> Option<String> {
        agentstacks_too_new(Path::new(FILE), text, running).map(|found| found.to_string())
    }

    #[test]
    fn agentstacks_too_new_words_the_refusal() {
        assert_eq!(
            excluded("[requires]\nagentstacks = \">=1.3\"\n", &Version::new(1, 0, 0)),
            Some(
                "packs/sdlc/pack.toml: requires.agentstacks: \">=1.3\" does not match agentstacks 1.0.0 (install a newer agentstacks)"
                    .to_owned()
            )
        );
    }

    #[test]
    fn agentstacks_too_new_is_silent_unless_a_valid_range_excludes() {
        let running = Version::new(1, 0, 0);
        for text in [
            "[requires]\nagentstacks = \">=1.0\"\n",
            "[requires]\nagentstacks = \"*\"\n",
            "[requires]\nairsl = \">=99\"\n",
            "[requires]\nagentstacks = \"nope\"\n",
            "[requires]\nagentstacks = 3\n",
            "requires = \"x\"\n",
            "[pack]\nname = \"a\"\n",
            "agentstacks = \">=99\"\na = [1,",
            "",
        ] {
            assert_eq!(excluded(text, &running), None, "{text:?}");
        }
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

    #[test]
    fn requires_agentstacks_absent_accepts() -> TestResult {
        let (requires, shown) = read("airsl = \">=0.1.4\"\n", None)?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(requires.agentstacks().is_none());

        let (requires, shown) = read("agentstacks = \">=1.0\"\n", None)?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(
            requires.agentstacks().map(ToString::to_string),
            Some(">=1.0".to_owned())
        );
        Ok(())
    }

    #[test]
    fn agentstacks_range_messages_match_airsl() -> TestResult {
        for text in ["", "   ", "nope"] {
            let (_, shown) = read(&format!("agentstacks = \"{text}\""), None)?;
            assert_eq!(
                shown,
                [format!(
                    "packs/sdlc/pack.toml: requires.agentstacks: \"{text}\" is not a version range"
                )],
                "{text:?}"
            );
        }
        let (_, shown) = read("agentstacks = 1", None)?;
        assert_eq!(
            shown,
            ["packs/sdlc/pack.toml: requires.agentstacks: expected string, got integer"]
        );
        Ok(())
    }
}
