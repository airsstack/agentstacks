//! A project whose packs and extensions all loaded and agree with one another.
//!
//! It is a type of its own so "has passed every whole-set check" is visible in a signature: a stage
//! that takes a [`ValidSet`] cannot be handed a set that skipped one.
//!
//! # Responsibilities
//!
//! - [`ValidSet`] holds the project file, the loaded packs and extensions by their key in it, the
//!   resolved option values of each pack and the airsl range of the set. It can only be built by
//!   [`check`](crate::types::set::check::check), so every later stage that takes one works
//!   from a set that passed every whole-set check.
//!
//! # Non-responsibilities
//!
//! Finding what is wrong with a set belongs to the checks that build it. Planning and applying an
//! install belong to the stages that take it.

use std::collections::BTreeMap;

use crate::types::extension::Extension;
use crate::types::manifest::OptionValue;
use crate::types::name::OptionName;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::Pack;
use crate::types::project::ProjectFile;
use crate::types::set::airsl::AirslRange;

/// A project that passed every whole-set check.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{Extension, OptionName, Pack, PackName, ProjectFile, check_pack_set};
///
/// let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
/// let text = "api = 0\n\
///     [tools.claude]\n\
///     path = \"extensions/claude\"\n\
///     [packs.core]\n\
///     path = \"packs/core\"\n\
///     options = { archive_keep = 50 }\n";
/// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
///
/// let set = check_pack_set(
///     Path::new("agentstacks.toml"),
///     project,
///     |_key, _entry| {
///         Pack::load(&StdFileTree::new(format!("{fixtures}/sets/valid-realistic/packs/core")))
///     },
///     |_key, _source| {
///         Extension::load(&StdFileTree::new(format!("{fixtures}/extensions/valid/minimal")))
///     },
/// )
/// .map_err(|refusals| format!("{refusals:?}"))?;
/// let kept = set.option(&PackName::new("core")?, &OptionName::new("archive_keep")?);
/// assert_eq!(kept.map(ToString::to_string).as_deref(), Some("50"));
/// assert!(set.airsl().is_some());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct ValidSet {
    project: ProjectFile,
    packs: BTreeMap<PackName, Pack>,
    extensions: BTreeMap<ToolName, Extension>,
    options: BTreeMap<PackName, BTreeMap<OptionName, OptionValue>>,
    airsl: Option<AirslRange>,
}

impl ValidSet {
    /// A set of already checked parts.
    pub(crate) const fn new(
        project: ProjectFile,
        packs: BTreeMap<PackName, Pack>,
        extensions: BTreeMap<ToolName, Extension>,
        options: BTreeMap<PackName, BTreeMap<OptionName, OptionValue>>,
        airsl: Option<AirslRange>,
    ) -> Self {
        Self {
            project,
            packs,
            extensions,
            options,
            airsl,
        }
    }

    /// The project file the set was checked for.
    #[must_use]
    pub const fn project(&self) -> &ProjectFile {
        &self.project
    }

    /// The loaded packs, by their key in the project file.
    #[must_use]
    pub const fn packs(&self) -> &BTreeMap<PackName, Pack> {
        &self.packs
    }

    /// The pack listed as `key`.
    #[must_use]
    pub fn pack(&self, key: &PackName) -> Option<&Pack> {
        self.packs.get(key)
    }

    /// The loaded extensions, by their key in the project file.
    #[must_use]
    pub const fn extensions(&self) -> &BTreeMap<ToolName, Extension> {
        &self.extensions
    }

    /// The extension listed as `key`.
    #[must_use]
    pub fn extension(&self, key: &ToolName) -> Option<&Extension> {
        self.extensions.get(key)
    }

    /// The value of every option of the pack listed as `key`: the value the project sets or the
    /// option's default.
    #[must_use]
    pub fn options(&self, key: &PackName) -> Option<&BTreeMap<OptionName, OptionValue>> {
        self.options.get(key)
    }

    /// The value of one option of the pack listed as `key`.
    #[must_use]
    pub fn option(&self, key: &PackName, option: &OptionName) -> Option<&OptionValue> {
        self.options.get(key)?.get(option)
    }

    /// The airsl range the packs need together; `None` when no pack declares one.
    #[must_use]
    pub const fn airsl(&self) -> Option<&AirslRange> {
        self.airsl.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manifest::OptionValue;
    use crate::types::set::testing::{PackSpec, name, run_check};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn core() -> PackSpec {
        PackSpec::new("core")
            .integer_option("archive_keep", 20)
            .tables(
                "[options.style_reinject]\ntype = \"boolean\"\ndefault = true\ndescription = \"d\"\n",
            )
    }

    #[test]
    fn valid_set_answers_option_and_airsl_queries() -> TestResult {
        let set = run_check(
            &[(core().airsl(">=0.1.4"), "archive_keep = 50")],
            &["claude"],
        )?
        .map_err(|refusals| refusals.join("; "))?;
        let core = name("core")?;
        assert_eq!(
            set.option(&core, &OptionName::new("archive_keep")?),
            Some(&OptionValue::Integer(50))
        );
        assert_eq!(
            set.option(&core, &OptionName::new("style_reinject")?),
            Some(&OptionValue::Boolean(true))
        );
        assert_eq!(set.option(&core, &OptionName::new("nope")?), None);
        assert_eq!(set.options(&core).map(BTreeMap::len), Some(2));
        let range = set.airsl().ok_or("core declares a range")?;
        assert_eq!(range.needs().len(), 1);
        assert!(set.pack(&core).is_some());
        assert!(set.extension(&ToolName::new("claude")?).is_some());
        assert_eq!(set.packs().len(), 1);
        assert_eq!(set.extensions().len(), 1);
        assert_eq!(set.project().packs().len(), 1);
        Ok(())
    }

    #[test]
    fn valid_set_has_no_airsl_range_when_no_pack_declares_one() -> TestResult {
        let set =
            run_check(&[(core(), "")], &["claude"])?.map_err(|refusals| refusals.join("; "))?;
        assert!(set.airsl().is_none());
        Ok(())
    }
}
