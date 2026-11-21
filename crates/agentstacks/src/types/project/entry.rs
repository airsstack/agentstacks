//! One pack the project installs.
//!
//! A pack entry is not a bare [`Source`]: unlike a tool, a pack carries an `options` table, so the
//! entry pairs the source with it instead of widening `Source` for both.
//!
//! # Responsibilities
//!
//! - [`PackEntry`] holds the pack's [`Source`] and the `options` table as the author wrote it.
//!
//! # Non-responsibilities
//!
//! Typing the options against the pack's declarations needs the pack's `pack.toml`, so it belongs
//! to the code that has loaded the pack.

use toml::Table;

use crate::types::project::Source;

/// A `[packs.<name>]` entry.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{ProjectFile, Source};
///
/// let text = "api = 0\n\
///     [tools.claude]\n\
///     path = \"extensions/claude\"\n\
///     [packs.team-lint]\n\
///     path = \"packs/team-lint\"\n";
/// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
/// let entry = project.packs().values().next().ok_or("no pack entry")?;
/// assert!(matches!(entry.source(), Source::Path(path) if path.as_str() == "packs/team-lint"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PackEntry {
    source: Source,
    options: Table,
}

impl PackEntry {
    /// An entry for a pack from `source` with the raw `options`.
    #[must_use]
    pub(crate) const fn new(source: Source, options: Table) -> Self {
        Self { source, options }
    }

    /// Where the pack comes from.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    /// The options as written, untyped; empty when the entry has none.
    #[must_use]
    pub(crate) const fn options(&self) -> &Table {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::project::LocalPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn pack_entry_keeps_its_source_and_raw_options() -> TestResult {
        let source = Source::Path(LocalPath::new("packs/team-lint")?);
        let options: Table = "archive_keep = 50\nname = \"x\"\n".parse()?;
        let entry = PackEntry::new(source.clone(), options.clone());
        assert_eq!(entry.source(), &source);
        assert_eq!(entry.options(), &options);
        assert_eq!(
            entry
                .options()
                .get("archive_keep")
                .and_then(toml::Value::as_integer),
            Some(50)
        );
        Ok(())
    }

    #[test]
    fn pack_entry_options_may_be_empty() -> TestResult {
        let entry = PackEntry::new(Source::Path(LocalPath::new("p")?), Table::new());
        assert!(entry.options().is_empty());
        Ok(())
    }
}
