//! The confirmation prompt.
//!
//! # Responsibilities
//!
//! - [`Answer`] is what the author said: yes, no, or nothing because there is no terminal.
//! - [`ask`] writes the question and reads one line, with the terminal flag, the reader and the
//!   writer passed in.
//!
//! # Non-responsibilities
//!
//! Detecting the terminal, opening the streams and acting on the answer belong to the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::io::{BufRead, Write};

/// The question written to the error stream, with no trailing newline.
const QUESTION: &str = "Apply? [y/N] ";

/// What the author answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    /// A trimmed, case-insensitive `y` or `yes`.
    Yes,
    /// Anything else: another word, an empty line, end of input or a read error.
    No,
    /// There is no terminal to ask on, so nothing was asked.
    NoTerminal,
}

/// Asks `Apply? [y/N] ` on `err` and reads one line from `input`.
///
/// When `is_terminal` is false it returns [`Answer::NoTerminal`] without writing or reading. A
/// failure to write the question does not stop the read.
pub(crate) fn ask(is_terminal: bool, mut input: impl BufRead, mut err: impl Write) -> Answer {
    if !is_terminal {
        return Answer::NoTerminal;
    }
    let _ = write!(err, "{QUESTION}").and_then(|()| err.flush());
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(read) if read > 0 && is_yes(&line) => Answer::Yes,
        _ => Answer::No,
    }
}

/// Whether `line` is `y` or `yes`, ignoring case and surrounding whitespace.
fn is_yes(line: &str) -> bool {
    let word = line.trim();
    word.eq_ignore_ascii_case("y") || word.eq_ignore_ascii_case("yes")
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufRead, Read};

    use super::{Answer, ask};

    struct Failing;

    impl Read for Failing {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("broken"))
        }
    }

    impl BufRead for Failing {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::other("broken"))
        }

        fn consume(&mut self, _amount: usize) {}
    }

    fn answer(text: &str) -> Answer {
        ask(true, text.as_bytes(), Vec::new())
    }

    #[test]
    fn no_terminal_returns_without_prompting() {
        let mut err = Vec::new();
        let got = ask(false, &b"y\n"[..], &mut err);
        assert_eq!(got, Answer::NoTerminal);
        assert!(err.is_empty());
    }

    #[test]
    fn y_yes_and_padded_uppercase_are_yes() {
        for text in ["y\n", "Y\n", " yes \n", "YES\n", "y"] {
            assert_eq!(answer(text), Answer::Yes, "{text:?}");
        }
    }

    #[test]
    fn other_words_are_no() {
        for text in ["n\n", "yep\n", "yes please\n", "ye\n"] {
            assert_eq!(answer(text), Answer::No, "{text:?}");
        }
    }

    #[test]
    fn empty_line_and_eof_are_no() {
        assert_eq!(answer("\n"), Answer::No);
        assert_eq!(answer(""), Answer::No);
    }

    #[test]
    fn read_error_is_no() {
        assert_eq!(ask(true, Failing, Vec::new()), Answer::No);
    }

    #[test]
    fn prompt_text_goes_to_err() {
        let mut err = Vec::new();
        let _ = ask(true, &b"n\n"[..], &mut err);
        assert_eq!(err, b"Apply? [y/N] ");
    }
}
