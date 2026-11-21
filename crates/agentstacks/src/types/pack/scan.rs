//! Turning the bytes of a rendered file into located tokens.
//!
//! A Markdown body and a `render` file are scanned the same way: the bytes must be UTF-8, and the
//! text is scanned once with its lines counted from the top of the file.
//!
//! It is separate from the loader so a body and a `render` file take one path to tokens, and so the
//! line arithmetic is tested on its own.
//!
//! # Responsibilities
//!
//! - [`rendered_text`] returns the text of a file's bytes, or the refusal that the file is not
//!   valid UTF-8 and cannot be rendered.
//! - [`scan_body`] scans text that begins on a given line of a file and returns the tokens found
//!   and the token refusals, each located at `<file>:<line>` with the line counted from the top of
//!   the file.
//!
//! # Non-responsibilities
//!
//! Finding where a body begins belongs to the frontmatter, and the rules over the tokens found
//! belong to the pack-local token rules.

use std::num::NonZeroUsize;
use std::path::Path;

use crate::types::pack::error::PackError;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::report::Location;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TextOrigin;
use crate::types::tokens::scan;

/// The text of `bytes`, the content of the file `file`.
///
/// # Errors
///
/// The refusal located at `file` when `bytes` are not valid UTF-8.
pub(super) fn rendered_text(file: &Path, bytes: Vec<u8>) -> Result<String, Diagnostic> {
    String::from_utf8(bytes).map_err(|_| Diagnostic::new(Location::file(file), PackError::NotUtf8))
}

/// Scans `text`, whose first line is line `start_line` of `file`.
#[must_use]
pub(super) fn scan_body(
    file: &Path,
    text: &str,
    start_line: NonZeroUsize,
) -> (Vec<LocatedToken>, Diagnostics) {
    scan(
        text,
        &TextOrigin::Lines {
            file: file.to_path_buf(),
            first_line: start_line,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn rendered_text_refuses_invalid_utf8() {
        let file = Path::new("packs/core/data.json");
        assert_eq!(
            rendered_text(file, vec![0x66, 0xFF])
                .err()
                .map(|d| d.to_string()),
            Some("packs/core/data.json: not valid UTF-8, cannot render".to_owned())
        );
        assert_eq!(
            rendered_text(file, b"a\n".to_vec()).ok().as_deref(),
            Some("a\n")
        );
    }

    #[test]
    fn scan_body_counts_lines_from_the_file_top() -> TestResult {
        let start = NonZeroUsize::new(5).ok_or("line 5")?;
        let (tokens, refused) = scan_body(Path::new("a.md"), "text\n{{skill status}}\n", start);
        assert!(refused.is_empty());
        let shown: Vec<String> = tokens.iter().map(|t| t.location().to_string()).collect();
        assert_eq!(shown, ["a.md:6"]);
        Ok(())
    }

    #[test]
    fn scan_body_locates_token_errors() -> TestResult {
        let start = NonZeroUsize::new(6).ok_or("line 6")?;
        let (tokens, refused) = scan_body(Path::new("a.md"), "{{skil x}}\n", start);
        assert!(tokens.is_empty());
        let shown: Vec<String> = refused
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(shown, ["a.md:6: unknown token kind \"skil\""]);
        Ok(())
    }
}
