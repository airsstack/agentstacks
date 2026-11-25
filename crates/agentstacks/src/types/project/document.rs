//! The editable parse of an existing `agentstacks.toml`, shared by the rules that rewrite one.
//!
//! # Responsibilities
//!
//! - `parse_document` parses text into a `toml_edit` document, which keeps comments and spacing,
//!   and refuses text that is not TOML at the project file in the parser's own words.
//!
//! # Non-responsibilities
//!
//! Checking that the document is a valid project file belongs to
//! [`ProjectFile`](crate::types::project::ProjectFile), and the edits themselves to the rules that
//! call this.

use toml_edit::DocumentMut;

use crate::types::project::PROJECT_FILE;
use crate::types::report::{Diagnostic, FormatError, Location, ParserMessage};

/// `text` as an editable document.
///
/// # Errors
///
/// Returns a [`Diagnostic`] located at the project file when `text` is not TOML, in the parser's
/// own words.
pub(super) fn parse_document(text: &str) -> Result<DocumentMut, Diagnostic> {
    text.parse().map_err(|error: toml_edit::TomlError| {
        Diagnostic::new(
            Location::file(PROJECT_FILE),
            FormatError::TomlSyntax(ParserMessage::new(&error.to_string())),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_that_is_toml_keeps_its_comments() -> Result<(), Diagnostic> {
        let text = "api = 1 # pinned\n";
        assert_eq!(parse_document(text)?.to_string(), text);
        Ok(())
    }

    #[test]
    fn text_that_is_not_toml_is_refused_at_the_project_file() {
        let refusal = parse_document("api = \n")
            .err()
            .map(|found| found.to_string());
        assert!(
            matches!(&refusal, Some(shown) if shown.starts_with("agentstacks.toml:")),
            "{refusal:?}"
        );
    }
}
