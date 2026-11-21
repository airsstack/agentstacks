//! Turns a YAML parser refusal into a located diagnostic.
//!
//! Translating the parser's error is kept apart from running the parser so the line offset and
//! the quoting hint are stated once, in one place.
//!
//! # Responsibilities
//!
//! - [`syntax_diagnostic`] places the parser's message at the file line it concerns, or at the file
//!   when the parser gave no line. The parser counts lines from the YAML block and the opening
//!   `---` is file line 1, so the file line is the reported line plus 1. No column is reported.
//! - It adds the quoting hint when the reported line has a `{{` that follows `:`, `-`, `[` or `,`
//!   with only spaces between, which is how an unquoted value starting with a token reads.
//!
//! # Non-responsibilities
//!
//! Running the parser and reading its error belongs to [`Frontmatter`](crate::types::Frontmatter).

use std::num::NonZeroUsize;
use std::path::Path;

use crate::types::frontmatter::error::FrontmatterError;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::report::ParserMessage;

/// The characters after which an unquoted `{{` starts a YAML value.
const VALUE_STARTS: [u8; 4] = [b':', b'-', b'[', b','];

/// A syntax diagnostic for `file`.
///
/// `reported_line` is the 1-based line of `block` the parser named, if it named one, and `message`
/// is the parser's message, passed through unchanged.
pub(super) fn syntax_diagnostic(
    file: &Path,
    block: &str,
    reported_line: Option<u64>,
    message: &str,
) -> Diagnostic {
    let line = reported_line.and_then(|line| usize::try_from(line).ok());
    let file_line = line
        .and_then(|line| line.checked_add(1))
        .and_then(NonZeroUsize::new);
    let location = file_line.map_or_else(
        || Location::file(file),
        |file_line| Location::at_line(file, file_line),
    );
    let quoting_hint = file_line.is_some()
        && line
            .and_then(|line| line.checked_sub(1))
            .and_then(|index| block.split('\n').nth(index))
            .is_some_and(has_token_starting_a_value);
    Diagnostic::new(
        location,
        FrontmatterError::Syntax {
            message: ParserMessage::new(message),
            quoting_hint,
        },
    )
}

/// Whether some `{{` on `line` follows `:`, `-`, `[` or `,`, with only spaces between.
fn has_token_starting_a_value(line: &str) -> bool {
    let bytes = line.as_bytes();
    line.match_indices("{{").any(|(open, _)| {
        bytes[..open]
            .iter()
            .rev()
            .find(|byte| **byte != b' ')
            .is_some_and(|byte| VALUE_STARTS.contains(byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "SKILL.md";
    const HINT: &str = "(a value starting with {{ must be quoted)";

    fn diagnose(block: &str, line: Option<u64>, message: &str) -> String {
        syntax_diagnostic(Path::new(FILE), block, line, message).to_string()
    }

    #[test]
    fn line_is_offset_to_file_line() {
        assert_eq!(
            diagnose("a: [1, 2", Some(1), "unclosed bracket '['"),
            "SKILL.md:2: unclosed bracket '['"
        );
    }

    #[test]
    fn hint_after_colon_dash_bracket_or_comma() {
        let blocks = [
            "description: {{skill status}} runs it",
            "description:   {{skill status}}",
            "  - {{skill x}}",
            "tools: [{{skill x}}]",
            "tools: [a,  {{skill x}}]",
        ];
        let message = "expected string scalar";
        for block in blocks {
            let shown = diagnose(block, Some(1), message);
            assert_eq!(shown, format!("SKILL.md:2: {message} {HINT}"), "{block}");
        }
    }

    #[test]
    fn no_hint_for_token_mid_value() {
        let block = "name: x\ndescription: run {{skill status}} now";
        assert_eq!(
            diagnose(block, Some(2), "expected string scalar"),
            "SKILL.md:3: expected string scalar"
        );
        assert_eq!(
            diagnose(block, Some(1), "expected string scalar"),
            "SKILL.md:2: expected string scalar"
        );
    }

    #[test]
    fn no_hint_for_token_at_line_start() {
        assert_eq!(
            diagnose("{{skill x}}", Some(1), "expected string scalar"),
            "SKILL.md:2: expected string scalar"
        );
    }

    #[test]
    fn no_location_gives_file_only_diagnostic_without_hint() {
        assert_eq!(
            diagnose("description: {{skill status}}", None, "unexpected input"),
            "SKILL.md: unexpected input"
        );
    }
}
