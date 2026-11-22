//! New tags written into an existing `agentstacks.toml` with its comments kept.
//!
//! `update` finds newer tags for entries the author already wrote by hand, so the file has
//! comments, alignment and a layout of its own. This rule changes only the tag text and leaves every
//! other byte as it was.
//!
//! # Responsibilities
//!
//! - [`retag_project`] sets `<packs|tools>.<name>.tag` for each change, in a standard table, an
//!   inline table or dotted keys alike, keeping the comment and spacing around the old value.
//! - It refuses a missing tag, and a tag that is no longer the one the change expects (the file
//!   was edited after `update` listed it), so a hand edit is never overwritten.
//! - One refusal stops the whole call; no partial text is returned.
//!
//! # Non-responsibilities
//!
//! Reading and writing the file, and checking that the text is a valid project file, belong to the
//! flow that calls this. Finding newer tags belongs to the update rules.

use toml_edit::{DocumentMut, Item, Value};

use crate::types::project::{GitTag, ProjectError};
use crate::types::report::{Diagnostic, FormatError, Location, ParserMessage};
use crate::types::resolve::SourceKey;

/// The file the refusals are located in.
const PROJECT_FILE: &str = "agentstacks.toml";

/// `text` with each change applied: the tag of entry `key` goes from `current` to `new`.
///
/// Comments, spacing and every other byte of `text` are kept.
///
/// # Errors
///
/// Returns a [`Diagnostic`] located at the entry's key path when the entry has no `tag`
/// (`no tag to update (run update again)`) or its tag is not `current`
/// (`tag changed during update (run update again)`). Returns one located at the file when `text`
/// is not TOML, in the parser's own words. Nothing is returned on a refusal, however many changes
/// came before it.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, PackName};
/// use agentstacks::types::project::retag_project;
/// use agentstacks::types::resolve::SourceKey;
///
/// let text = "[packs.core]\ngit = \"https://x\"\ntag = \"packs-v0.1.0\" # pinned\n";
/// let change = (
///     SourceKey::Pack(PackName::new("core")?),
///     GitTag::new("packs-v0.1.0")?,
///     GitTag::new("packs-v0.2.0")?,
/// );
/// let out = retag_project(text, &[change]).map_err(|refusal| refusal.to_string())?;
/// assert_eq!(out, "[packs.core]\ngit = \"https://x\"\ntag = \"packs-v0.2.0\" # pinned\n");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn retag_project(
    text: &str,
    changes: &[(SourceKey, GitTag, GitTag)],
) -> Result<String, Diagnostic> {
    let mut document: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
        Diagnostic::new(
            Location::file(PROJECT_FILE),
            FormatError::TomlSyntax(ParserMessage::new(&error.to_string())),
        )
    })?;
    for (key, current, new) in changes {
        retag_entry(&mut document, key, current, new)?;
    }
    Ok(document.to_string())
}

/// Replaces the `tag` of one entry, copying the old value's decor onto the new one.
///
/// The lookup goes through `Table::get_mut` and `TableLike::get_mut`, which only look. `Item::get_mut`
/// and indexing a missing entry go through `IndexMut`, which writes an empty table for the entry
/// into the document.
fn retag_entry(
    document: &mut DocumentMut,
    key: &SourceKey,
    current: &GitTag,
    new: &GitTag,
) -> Result<(), Diagnostic> {
    let (section, name) = match key {
        SourceKey::Pack(name) => ("packs", name.as_str()),
        SourceKey::Tool(name) => ("tools", name.as_str()),
    };
    let refuse = |refusal: ProjectError| Diagnostic::new(key.location(), refusal);
    let slot = document
        .get_mut(section)
        .and_then(Item::as_table_like_mut)
        .and_then(|table| table.get_mut(name))
        .and_then(Item::as_table_like_mut)
        .and_then(|entry| entry.get_mut("tag"))
        .and_then(Item::as_value_mut)
        .ok_or_else(|| refuse(ProjectError::NoTagToUpdate))?;
    if slot.as_str() != Some(current.as_str()) {
        return Err(refuse(ProjectError::TagChanged));
    }
    let decor = slot.decor().clone();
    *slot = Value::from(new.as_str());
    *slot.decor_mut() = decor;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::{PackName, ToolName};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OLD: &str = "packs-v0.1.0";
    const NEW: &str = "packs-v0.2.0";

    fn pack(name: &str, current: &str, new: &str) -> Result<(SourceKey, GitTag, GitTag), String> {
        let key = SourceKey::Pack(PackName::new(name).map_err(|e| e.to_string())?);
        Ok((
            key,
            GitTag::new(current).map_err(|e| e.to_string())?,
            GitTag::new(new).map_err(|e| e.to_string())?,
        ))
    }

    fn retag(text: &str, changes: &[(SourceKey, GitTag, GitTag)]) -> Result<String, String> {
        retag_project(text, changes).map_err(|refusal| refusal.to_string())
    }

    #[test]
    fn trailing_comment_survives_a_retag() -> TestResult {
        let text = "# my project\napi = 0\n\n[packs.core]\ngit = \"https://x\"\ntag = \"packs-v0.1.0\"   # pinned\nsubdir = \"packs/core\"\n";
        let out = retag(text, &[pack("core", OLD, NEW)?])?;
        assert_eq!(out, text.replace(OLD, NEW));
        Ok(())
    }

    #[test]
    fn inline_table_tag_is_retagged() -> TestResult {
        let text =
            "api = 0\n\n[packs]\ncore = { git = \"https://x\", tag = \"packs-v0.1.0\" } # c\n";
        let out = retag(text, &[pack("core", OLD, NEW)?])?;
        assert_eq!(out, text.replace(OLD, NEW));
        Ok(())
    }

    #[test]
    fn dotted_key_tag_is_retagged() -> TestResult {
        let text =
            "api = 0\npacks.core.git = \"https://x\"\npacks.core.tag = \"packs-v0.1.0\" # c\n";
        let out = retag(text, &[pack("core", OLD, NEW)?])?;
        assert_eq!(out, text.replace(OLD, NEW));
        Ok(())
    }

    #[test]
    fn tool_tag_is_retagged() -> TestResult {
        let text = "[tools.claude]\ntag = \"packs-v0.1.0\" # t\n\n[packs.core]\ntag = \"packs-v0.1.0\" # p\n";
        let key = SourceKey::Tool(ToolName::new("claude")?);
        let out = retag(text, &[(key, GitTag::new(OLD)?, GitTag::new(NEW)?)])?;
        assert_eq!(
            out,
            "[tools.claude]\ntag = \"packs-v0.2.0\" # t\n\n[packs.core]\ntag = \"packs-v0.1.0\" # p\n"
        );
        Ok(())
    }

    #[test]
    fn missing_pack_is_refused_and_writes_nothing_into_the_document() -> TestResult {
        let text = "[packs.core]\ntag = \"packs-v0.1.0\"\n";
        let refusal = retag(text, &[pack("nope", OLD, NEW)?]);
        assert_eq!(
            refusal,
            Err("agentstacks.toml: packs.nope: no tag to update (run update again)".to_owned())
        );

        let mut document: DocumentMut = text.parse()?;
        let (key, current, new) = pack("nope", OLD, NEW)?;
        assert!(retag_entry(&mut document, &key, &current, &new).is_err());
        assert_eq!(document.to_string(), text);
        Ok(())
    }

    #[test]
    fn entry_without_a_tag_is_refused() -> TestResult {
        let text = "[packs.core]\npath = \"packs/core\"\n";
        assert_eq!(
            retag(text, &[pack("core", OLD, NEW)?]),
            Err("agentstacks.toml: packs.core: no tag to update (run update again)".to_owned())
        );
        Ok(())
    }

    #[test]
    fn several_changes_apply_in_one_call() -> TestResult {
        let text = "[packs.core]\ntag = \"packs-v0.1.0\"\n\n[packs.sdlc]\ntag = \"packs-v0.1.0\"\n";
        let out = retag(text, &[pack("core", OLD, NEW)?, pack("sdlc", OLD, NEW)?])?;
        assert_eq!(out, text.replace(OLD, NEW));
        Ok(())
    }

    #[test]
    fn changed_tag_is_refused() -> TestResult {
        let text = "[packs.core]\ntag = \"packs-v0.1.5\"\n";
        assert_eq!(
            retag(text, &[pack("core", OLD, NEW)?]),
            Err(
                "agentstacks.toml: packs.core: tag changed during update (run update again)"
                    .to_owned()
            )
        );
        Ok(())
    }

    #[test]
    fn one_refusal_returns_no_text_for_the_changes_before_it() -> TestResult {
        let text = "[packs.core]\ntag = \"packs-v0.1.0\"\n";
        let refused = retag(text, &[pack("core", OLD, NEW)?, pack("nope", OLD, NEW)?]);
        assert_eq!(
            refused,
            Err("agentstacks.toml: packs.nope: no tag to update (run update again)".to_owned())
        );
        Ok(())
    }

    #[test]
    fn text_that_is_not_toml_is_refused_at_the_file() {
        let refused = retag("[packs.core\n", &[]);
        assert!(
            matches!(&refused, Err(message) if message.starts_with("agentstacks.toml: ")),
            "{refused:?}"
        );
    }
}
