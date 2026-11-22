//! The indent a JSON text is laid out with.
//!
//! # Responsibilities
//!
//! - [`Indent`]: the indent of one nesting level, read from the first indented line of a text.
//!
//! # Non-responsibilities
//!
//! Printing with the indent belongs to the printer, and deciding when to read it belongs to the
//! code that edits the document.

/// The indent of one nesting level: spaces, or one tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::types::json) struct Indent(String);

impl Indent {
    /// Reads the indent of `text` from its first indented line after the first line.
    ///
    /// The leading run of that line is the indent when it is all spaces or exactly one tab. A
    /// mixed run, a run of several tabs, or a text with no indented line gives two spaces. A
    /// trailing `\r` is never part of the run, so CRLF text reads like LF text.
    pub(in crate::types::json) fn detect(text: &str) -> Self {
        let Some(line) = text
            .split('\n')
            .skip(1)
            .find(|line| line.starts_with([' ', '\t']))
        else {
            return Self::two_spaces();
        };
        let run = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        if run == "\t" || run.bytes().all(|byte| byte == b' ') {
            Self(run.to_owned())
        } else {
            Self::two_spaces()
        }
    }

    /// Two spaces.
    fn two_spaces() -> Self {
        Self("  ".to_owned())
    }

    /// The indent as bytes, for the pretty-printer.
    pub(in crate::types::json) const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A four-space layout whose number spellings a re-print changes.
    const SOURCE: &str = "{\n    \"permissions\": {\n        \"allow\": [\n            \"Bash(x:*)\"\n        ]\n    },\n    \"zeta\": 1,\n    \"alpha\": 12345678901234567890123,\n    \"d\": 0.10,\n    \"hooks\": {}\n}";

    fn detected(text: &str) -> Vec<u8> {
        Indent::detect(text).as_bytes().to_vec()
    }

    #[test]
    fn indent_takes_two_spaces_from_the_first_indented_line() {
        assert_eq!(detected("{\n  \"a\": 1\n}"), b"  ");
    }

    #[test]
    fn indent_takes_four_spaces_from_the_first_indented_line() {
        assert_eq!(detected(SOURCE), b"    ");
    }

    #[test]
    fn indent_takes_one_tab_from_the_first_indented_line() {
        assert_eq!(detected("{\n\t\"a\": 1\n}"), b"\t");
    }

    #[test]
    fn indent_falls_back_to_two_spaces_for_mixed_whitespace() {
        assert_eq!(detected("{\n\t  \"a\": 1\n}"), b"  ");
        assert_eq!(detected("{\n \t\"a\": 1\n}"), b"  ");
    }

    #[test]
    fn indent_falls_back_to_two_spaces_for_two_tabs() {
        assert_eq!(detected("{\n\t\t\"a\": 1\n}"), b"  ");
    }

    #[test]
    fn indent_falls_back_to_two_spaces_without_an_indented_line() {
        assert_eq!(detected("{\"a\":1}"), b"  ");
        assert_eq!(detected(""), b"  ");
        assert_eq!(detected("{\n\"a\": 1\n}"), b"  ");
    }

    #[test]
    fn indent_ignores_the_first_line() {
        assert_eq!(detected("    {\n  \"a\": 1\n}"), b"  ");
    }

    #[test]
    fn indent_reads_crlf_text_like_lf_text() {
        assert_eq!(detected(&SOURCE.replace('\n', "\r\n")), b"    ");
    }
}
