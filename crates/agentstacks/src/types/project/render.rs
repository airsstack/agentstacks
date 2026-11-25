//! The text of a new `agentstacks.toml`.
//!
//! `init` knows which tools and packs to list and where each comes from; this rule turns those
//! entries into the file, so the layout lives in one place.
//!
//! # Responsibilities
//!
//! - [`render_project`] writes `api = 1`, then the tool entries and the pack entries, each section
//!   in name order, with a blank line before every entry.
//! - A git source writes `git`, `tag` (or `rev`) and `subdir` in that order; a path source writes
//!   one `path` key.
//!
//! # Non-responsibilities
//!
//! Choosing the entries, and writing the text to disk, belong to the flow that calls this. Changing
//! the tags of an existing file belongs to [`retag_project`](crate::types::project::retag_project).

use toml_edit::{DocumentMut, Item, Table, value};

use crate::types::api::API;
use crate::types::name::{PackName, ToolName};
use crate::types::project::{Pin, Source};

/// The text of a new project file listing `tools` and `packs`.
///
/// Each section is written in name order whatever order the entries come in. `[tools]` and
/// `[packs]` are never written as headers of their own, only `[tools.<name>]` and
/// `[packs.<name>]`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{LocalPath, PackName, Source};
/// use agentstacks::types::project::render_project;
///
/// let team = (PackName::new("team")?, Source::Path(LocalPath::new("packs/team")?));
/// assert_eq!(
///     render_project(&[], &[team]),
///     "api = 1\n\n[packs.team]\npath = \"packs/team\"\n",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn render_project(tools: &[(ToolName, Source)], packs: &[(PackName, Source)]) -> String {
    let mut document = DocumentMut::new();
    document.insert("api", value(API));
    document.insert(
        "tools",
        section(tools.iter().map(|(name, source)| (name.as_str(), source))),
    );
    document.insert(
        "packs",
        section(packs.iter().map(|(name, source)| (name.as_str(), source))),
    );
    document.to_string()
}

/// An implicit table holding one sub-table per entry, in name order.
fn section<'a>(entries: impl Iterator<Item = (&'a str, &'a Source)>) -> Item {
    let mut sorted: Vec<_> = entries.collect();
    sorted.sort_by_key(|(name, _)| *name);
    let mut table = Table::new();
    table.set_implicit(true);
    for (name, source) in sorted {
        table.insert(name, Item::Table(entry(source)));
    }
    Item::Table(table)
}

/// The keys of one entry, in the order `git`, `tag` or `rev`, `subdir`, or the single `path`.
pub(super) fn entry(source: &Source) -> Table {
    let mut table = Table::new();
    match source {
        Source::Git { url, pin, subdir } => {
            table.insert("git", value(url.as_str()));
            match pin {
                Pin::Tag(tag) => table.insert("tag", value(tag.as_str())),
                Pin::Rev(rev) => table.insert("rev", value(rev.as_str())),
            };
            if let Some(subdir) = subdir {
                table.insert("subdir", value(subdir.as_str()));
            }
        }
        Source::Path(path) => {
            table.insert("path", value(path.as_str()));
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::project::{CommitId, GitTag, GitUrl, LocalPath, ProjectFile, SubPath};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn official(subdir: &str) -> Result<Source, Box<dyn std::error::Error>> {
        Ok(Source::Git {
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            pin: Pin::Tag(GitTag::new("packs-v0.1.0")?),
            subdir: Some(SubPath::new(subdir)?),
        })
    }

    fn official_set() -> Result<String, Box<dyn std::error::Error>> {
        Ok(render_project(
            &[(ToolName::new("claude")?, official("extensions/claude")?)],
            &[
                (PackName::new("core")?, official("packs/core")?),
                (PackName::new("sdlc")?, official("packs/sdlc")?),
            ],
        ))
    }

    #[test]
    fn official_set_renders_the_exact_bytes() -> TestResult {
        assert_eq!(
            official_set()?,
            "api = 1\n\n[tools.claude]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/core\"\n\n[packs.sdlc]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/sdlc\"\n"
        );
        Ok(())
    }

    #[test]
    fn rendered_text_parses_as_a_project_file() -> TestResult {
        let text = official_set()?;
        let project = ProjectFile::parse(Path::new("agentstacks.toml"), &text)
            .map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(project.tools().len(), 1);
        assert_eq!(project.packs().len(), 2);
        Ok(())
    }

    #[test]
    fn path_source_renders_one_path_key() -> TestResult {
        let text = render_project(
            &[],
            &[(
                PackName::new("team")?,
                Source::Path(LocalPath::new("packs/team")?),
            )],
        );
        assert!(text.contains("[packs.team]\npath = \"packs/team\"\n"));
        Ok(())
    }

    #[test]
    fn rev_source_renders_rev_instead_of_tag() -> TestResult {
        let source = Source::Git {
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            pin: Pin::Rev(CommitId::new("0123456789abcdef0123456789abcdef01234567")?),
            subdir: Some(SubPath::new("packs/core")?),
        };
        let text = render_project(&[], &[(PackName::new("core")?, source)]);
        assert_eq!(
            text,
            "api = 1\n\n[packs.core]\ngit = \"https://github.com/airsstack/agentstacks\"\nrev = \"0123456789abcdef0123456789abcdef01234567\"\nsubdir = \"packs/core\"\n"
        );
        Ok(())
    }

    #[test]
    fn no_empty_parent_table_is_written() -> TestResult {
        let text = official_set()?;
        assert!(!text.contains("[tools]\n"));
        assert!(!text.contains("[packs]\n"));
        Ok(())
    }

    #[test]
    fn entries_are_sorted_by_name_within_each_section() -> TestResult {
        let text = render_project(
            &[],
            &[
                (PackName::new("sdlc")?, official("packs/sdlc")?),
                (PackName::new("core")?, official("packs/core")?),
            ],
        );
        let core = text.find("[packs.core]").ok_or("core missing")?;
        let sdlc = text.find("[packs.sdlc]").ok_or("sdlc missing")?;
        assert!(core < sdlc);
        Ok(())
    }
}
