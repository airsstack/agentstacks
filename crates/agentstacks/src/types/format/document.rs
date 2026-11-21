//! Reads TOML text into a raw table.
//!
//! Every TOML file of the crate goes through `parse_toml_document`, and each field is then
//! checked by hand through `TomlFields`. A typed struct parse is never used because it stops at
//! the first error, and a run reports every error of a file. Both are crate-internal, because the
//! table they hand out is a `toml` type.
//!
//! # Responsibilities
//!
//! - `parse_toml_document` returns the text's [`toml::Table`], or one syntax [`Diagnostic`]
//!   located at `<file>:<line>` carrying the parser's message.
//! - Duplicate keys, duplicate tables and integers beyond `i64` are syntax errors of the parser,
//!   so they arrive here.
//! - Empty, whitespace-only and comment-only text is an empty table, so the fields it lacks are
//!   refused one by one by the caller.
//!
//! # Non-responsibilities
//!
//! Reading and checking the table's fields belongs to `TomlFields`.

use std::num::NonZeroUsize;
use std::path::Path;

use toml::Table;

use crate::types::report::Diagnostic;
use crate::types::report::FormatError;
use crate::types::report::Location;
use crate::types::report::ParserMessage;

/// Parses `text`, the content of `file`, into a raw table.
///
/// # Errors
///
/// Returns one [`Diagnostic`] located at the line the parser's error span starts on, whose message
/// is the parser's own.
pub(in crate::types) fn parse_toml_document(file: &Path, text: &str) -> Result<Table, Diagnostic> {
    text.parse::<Table>().map_err(|error| {
        let offset = error.span().map_or(0, |span| span.start);
        Diagnostic::new(
            Location::at_line(file, line_of(text, offset)),
            FormatError::TomlSyntax(ParserMessage::new(error.message())),
        )
    })
}

/// The 1-based line of byte `offset` in `text`: the count of `\n` before it, plus one.
fn line_of(text: &str, offset: usize) -> NonZeroUsize {
    let newlines = text
        .bytes()
        .take(offset)
        .filter(|byte| *byte == b'\n')
        .count();
    NonZeroUsize::MIN.saturating_add(newlines)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn file() -> &'static Path {
        Path::new("packs/core/pack.toml")
    }

    fn syntax_error(text: &str) -> Result<String, Box<dyn std::error::Error>> {
        match parse_toml_document(file(), text) {
            Ok(_) => Err("expected a syntax error".into()),
            Err(diagnostic) => Ok(diagnostic.to_string()),
        }
    }

    #[test]
    fn parse_document_reads_a_table() -> TestResult {
        let table =
            parse_toml_document(file(), "[pack]\nname = \"core\"\n").map_err(|d| d.to_string())?;
        let name = table
            .get("pack")
            .and_then(toml::Value::as_table)
            .and_then(|pack| pack.get("name"))
            .and_then(toml::Value::as_str);
        assert_eq!(name, Some("core"));
        Ok(())
    }

    #[test]
    fn parse_document_reads_empty_text_as_an_empty_table() -> TestResult {
        for text in ["", "  \n\t", "# only a comment\n"] {
            let table = parse_toml_document(file(), text).map_err(|d| d.to_string())?;
            assert!(table.is_empty(), "{text:?}");
        }
        Ok(())
    }

    #[test]
    fn parse_document_locates_a_syntax_error_by_line() -> TestResult {
        assert_eq!(
            syntax_error("a = [1,")?,
            "packs/core/pack.toml:1: unclosed array, expected `]`"
        );
        assert_eq!(
            syntax_error("\n\na = [1,")?,
            "packs/core/pack.toml:3: unclosed array, expected `]`"
        );
        Ok(())
    }

    #[test]
    fn parse_document_treats_duplicates_and_overflow_as_syntax_errors() -> TestResult {
        for text in ["a = 1\na = 2", "[x]\n[x]"] {
            let shown = syntax_error(text)?;
            assert!(shown.starts_with("packs/core/pack.toml:"), "{shown}");
            assert!(shown.contains("duplicate key"), "{shown}");
        }
        let shown = syntax_error("a = 9223372036854775808")?;
        assert!(shown.starts_with("packs/core/pack.toml:1: "), "{shown}");
        assert!(shown.contains("u64 value was too large"), "{shown}");
        Ok(())
    }

    #[test]
    fn line_of_counts_newlines_before_the_offset() {
        assert_eq!(line_of("a\nb\nc", 0).get(), 1);
        assert_eq!(line_of("a\nb\nc", 2).get(), 2);
        assert_eq!(line_of("a\nb\nc", 5).get(), 3);
    }
}
