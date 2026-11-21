//! Splices token values into scanned text.
//!
//! Splicing is kept apart from scanning because choosing a value is the caller's business: the
//! scanner finds each token once, and this function applies whatever values the caller chose over
//! the same spans.
//!
//! # Responsibilities
//!
//! - [`render`] copies the text, replaces each valued token span with its value verbatim, and
//!   turns every `\{{` outside those spans into `{{`.
//! - Values are never scanned again, so a value may hold `{{` freely.
//!
//! A `\` directly before `{{` is always the escape, even in text where a backslash means
//! something else, such as a Windows path.
//!
//! # Non-responsibilities
//!
//! Finding tokens belongs to [`scan`](crate::types::tokens::scan::scan), and choosing values belongs to the
//! caller.

use crate::types::tokens::values::TokenValues;

/// Renders `text` with `values` spliced over the token spans.
///
/// `text` must be the exact text that was scanned, because the spans index it, and `values` must
/// hold one value per token. A span with no value is copied as written.
///
/// Each span must lie within `text`, on `char` boundaries, and after the span before it. A span
/// that breaks this is skipped in a release build, so its value is silently dropped, and panics in
/// a debug build.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{TextOrigin, TokenValues, render, scan};
///
/// let text = r"Run {{var home}}/bin, not \{{var home}}.";
/// let origin = TextOrigin::Lines { file: "SKILL.md".into(), first_line: NonZeroUsize::MIN };
/// let (tokens, refusals) = scan(text, &origin);
/// assert!(refusals.is_empty());
///
/// let token = tokens.first().ok_or("no token found")?;
/// let mut values = TokenValues::new();
/// values.insert(token.token(), "/home/me".to_owned());
/// assert_eq!(render(text, &values), "Run /home/me/bin, not {{var home}}.");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn render(text: &str, values: &TokenValues) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied_to = 0;
    for (span, value) in values.iter() {
        debug_assert!(
            span.start >= copied_to
                && span.start <= span.end
                && span.end <= text.len()
                && text.is_char_boundary(span.start)
                && text.is_char_boundary(span.end),
            "token span {span:?} does not fit the text it was scanned from"
        );
        let Some(gap) = text.get(copied_to..span.start) else {
            continue;
        };
        out.push_str(&unescape(gap));
        out.push_str(value);
        copied_to = span.end;
    }
    out.push_str(&unescape(text.get(copied_to..).unwrap_or_default()));
    out
}

/// Turns every `\{{` in `text` into `{{`.
fn unescape(text: &str) -> String {
    text.replace("\\{{", "{{")
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use super::*;
    use crate::types::{TextOrigin, scan};

    fn origin() -> TextOrigin {
        TextOrigin::Lines {
            file: PathBuf::from("notes.md"),
            first_line: NonZeroUsize::MIN,
        }
    }

    fn render_with(text: &str, replacements: &[&str]) -> String {
        let (tokens, _) = scan(text, &origin());
        assert_eq!(tokens.len(), replacements.len(), "{text}");
        let mut values = TokenValues::new();
        for (token, value) in tokens.iter().zip(replacements) {
            values.insert(token.token(), (*value).to_owned());
        }
        render(text, &values)
    }

    #[test]
    fn render_replaces_tokens_and_keeps_surrounding_text() {
        assert_eq!(
            render_with("run {{skill sdlc/status}} now", &["/status"]),
            "run /status now"
        );
    }

    #[test]
    fn render_copies_a_token_with_no_value_as_written() {
        let text = "a {{root}} b {{skill x}}";
        let (tokens, _) = scan(text, &origin());
        let mut values = TokenValues::new();
        if let [_, second] = tokens.as_slice() {
            values.insert(second.token(), "/x".to_owned());
        }
        assert_eq!(values.iter().count(), 1);
        assert_eq!(render(text, &values), "a {{root}} b /x");
    }

    #[test]
    fn render_unescapes_literal_braces() {
        assert_eq!(render_with("$\\{{ secrets.X }}", &[]), "${{ secrets.X }}");
    }

    #[test]
    fn render_does_not_rescan_values() {
        assert_eq!(
            render_with("a {{root}} b", &["{{skill x}}"]),
            "a {{skill x}} b"
        );
    }

    #[test]
    fn render_backslash_before_token_is_the_escape() {
        assert_eq!(render_with("C:\\{{root}}", &[]), "C:{{root}}");
    }

    #[test]
    fn render_handles_several_tokens_and_an_escape_on_one_line() {
        assert_eq!(
            render_with(
                "see {{skill status}}, \\{{literal}} and {{root}}/x",
                &["/status", ".agentstacks/core"]
            ),
            "see /status, {{literal}} and .agentstacks/core/x"
        );
    }
}
