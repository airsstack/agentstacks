//! The one more `[tools.<name>]` table `ext add` writes into an existing `agentstacks.toml`.
//!
//! `ext add` installs a tool into a project whose author already wrote the file by hand, so it
//! has comments, alignment and a layout of its own. This rule adds the new table and leaves every
//! other byte as it was.
//!
//! # Responsibilities
//!
//! - `add_tool` writes `[tools.<name>]` with the keys of the source (`git`, then `tag` or `rev`,
//!   then `subdir`, or the single `path` as given), placed after the last `tools.*` table.
//! - When `tools` is an inline table the entry joins it, and when `tools` is written as dotted keys
//!   the entry becomes a `[tools.<name>]` table.
//! - It refuses a tool that is already listed, so a hand edit is never overwritten.
//!
//! # Non-responsibilities
//!
//! Reading and writing the file, and checking that the text is a valid project file, belong to the
//! flow that calls this.

use toml_edit::{DocumentMut, Item, Table, Value};

use crate::types::name::ToolName;
use crate::types::project::render::entry;
use crate::types::project::{ProjectError, Source};

/// `text` with the table `[tools.<name>]` added for `source`.
///
/// Comments, spacing and every other byte of `text` are kept.
///
/// # Errors
///
/// Returns [`ProjectError::ToolExists`] when `tools.<name>` is already there, and
/// [`ProjectError::ChangedDuringAdd`] when `text` is no longer TOML or its `tools` is neither a
/// table nor an inline table, which a file the caller has already checked cannot be. The flow that
/// calls this is the only caller, and for it that is the file having changed since it was read.
pub(crate) fn add_tool(
    text: &str,
    name: &ToolName,
    source: &Source,
) -> Result<String, ProjectError> {
    let mut document: DocumentMut = text.parse().map_err(|_| ProjectError::ChangedDuringAdd)?;
    let slot = document
        .entry("tools")
        .or_insert_with(|| Item::Table(implicit_table()));
    if slot
        .as_table_like()
        .is_some_and(|tools| tools.contains_key(name.as_str()))
    {
        return Err(ProjectError::ToolExists {
            name: name.as_str().to_owned(),
        });
    }
    let mut table = entry(source);
    match slot {
        Item::Table(tools) => {
            if let Some(last) = tools
                .iter()
                .filter_map(|(_, item)| item.as_table().and_then(Table::position))
                .max()
            {
                table.set_position(Some(last));
            }
            tools.insert(name.as_str(), Item::Table(table));
        }
        Item::Value(Value::InlineTable(tools)) => {
            tools.insert(name.as_str(), Value::InlineTable(table.into_inline_table()));
        }
        _ => return Err(ProjectError::ChangedDuringAdd),
    }
    Ok(document.to_string())
}

/// A table with no header of its own, which only its sub-tables show.
fn implicit_table() -> Table {
    let mut table = Table::new();
    table.set_implicit(true);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::project::{CommitId, GitTag, GitUrl, LocalPath, Pin, ProjectFile, SubPath};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn mytool() -> Result<(ToolName, Source), Box<dyn std::error::Error>> {
        Ok((
            ToolName::new("mytool")?,
            Source::Git {
                url: GitUrl::new("https://example.com/x.git")?,
                pin: Pin::Tag(GitTag::new("v1.2.0")?),
                subdir: Some(SubPath::new("ext/mytool")?),
            },
        ))
    }

    fn add(text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let (name, source) = mytool()?;
        Ok(add_tool(text, &name, &source)?)
    }

    fn tool_names(text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let project = ProjectFile::parse(std::path::Path::new("agentstacks.toml"), text)
            .map_err(|refusals| format!("{refusals:?}"))?;
        let mut names: Vec<String> = project
            .tools()
            .keys()
            .map(|name| name.as_str().to_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    #[test]
    fn inserts_after_last_tool_table() -> TestResult {
        let text = "# project\napi = 1\n\n# the tool\n[tools.claude]\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core] # core pack\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"packs/core\"\n";
        let out = add(text)?;
        assert_eq!(
            out,
            "# project\napi = 1\n\n# the tool\n[tools.claude]\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"extensions/claude\"\n\n[tools.mytool]\ngit = \"https://example.com/x.git\"\ntag = \"v1.2.0\"\nsubdir = \"ext/mytool\"\n\n[packs.core] # core pack\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"packs/core\"\n"
        );
        assert_eq!(tool_names(&out)?, ["claude", "mytool"]);
        Ok(())
    }

    #[test]
    fn inline_tools() -> TestResult {
        let text = "api = 1\ntools = { claude = { path = \"ext/claude\" } }\n\n[packs.core]\npath = \"packs/core\"\n";
        let out = add(text)?;
        assert_eq!(
            out,
            "api = 1\ntools = { claude = { path = \"ext/claude\" } , mytool = { git = \"https://example.com/x.git\", tag = \"v1.2.0\", subdir = \"ext/mytool\" } }\n\n[packs.core]\npath = \"packs/core\"\n"
        );
        assert_eq!(tool_names(&out)?, ["claude", "mytool"]);
        Ok(())
    }

    #[test]
    fn dotted_tools() -> TestResult {
        let text =
            "api = 1\ntools.claude.path = \"ext/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
        let out = add(text)?;
        assert_eq!(
            out,
            "api = 1\ntools.claude.path = \"ext/claude\"\n\n[tools.mytool]\ngit = \"https://example.com/x.git\"\ntag = \"v1.2.0\"\nsubdir = \"ext/mytool\"\n\n[packs.core]\npath = \"packs/core\"\n"
        );
        assert_eq!(tool_names(&out)?, ["claude", "mytool"]);
        Ok(())
    }

    #[test]
    fn keys_in_order_git_tag_subdir() -> TestResult {
        let name = ToolName::new("mytool")?;
        let source = Source::Git {
            url: GitUrl::new("https://example.com/x.git")?,
            pin: Pin::Rev(CommitId::new("0123456789abcdef0123456789abcdef01234567")?),
            subdir: Some(SubPath::new("ext/mytool")?),
        };
        let out = add_tool(
            "api = 1\n\n[tools.claude]\npath = \"ext/claude\"\n",
            &name,
            &source,
        )?;
        assert_eq!(
            out,
            "api = 1\n\n[tools.claude]\npath = \"ext/claude\"\n\n[tools.mytool]\ngit = \"https://example.com/x.git\"\nrev = \"0123456789abcdef0123456789abcdef01234567\"\nsubdir = \"ext/mytool\"\n"
        );
        Ok(())
    }

    #[test]
    fn path_source_as_written() -> TestResult {
        let name = ToolName::new("x")?;
        let source = Source::Path(LocalPath::new("./ext/x")?);
        let out = add_tool(
            "api = 1\n\n[tools.claude]\npath = \"ext/claude\"\n",
            &name,
            &source,
        )?;
        assert!(out.ends_with("[tools.x]\npath = \"./ext/x\"\n"), "{out}");
        Ok(())
    }

    #[test]
    fn existing_key_refused() -> TestResult {
        let name = ToolName::new("claude")?;
        let source = Source::Path(LocalPath::new("ext/other")?);
        for text in [
            "api = 1\n\n[tools.claude]\npath = \"ext/claude\"\n",
            "api = 1\ntools = { claude = { path = \"ext/claude\" } }\n",
            "api = 1\ntools.claude.path = \"ext/claude\"\n",
        ] {
            let refusal = add_tool(text, &name, &source)
                .err()
                .ok_or("an existing tool was overwritten")?;
            assert_eq!(
                refusal.to_string(),
                "tools.claude already exists (edit it there)"
            );
        }
        Ok(())
    }

    #[test]
    fn missing_tools_section_is_created() -> TestResult {
        let out = add("api = 1\n\n[packs.core]\npath = \"packs/core\"\n")?;
        assert_eq!(tool_names(&out)?, ["mytool"]);
        assert!(out.contains("[tools.mytool]\n"), "{out}");
        Ok(())
    }

    #[test]
    fn text_that_is_not_toml_is_a_change() -> TestResult {
        let refusal = add("api = \n").err().ok_or("accepted")?;
        assert_eq!(refusal.to_string(), "changed during ext add (run it again)");
        Ok(())
    }
}
