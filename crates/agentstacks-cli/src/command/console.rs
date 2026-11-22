//! The terminal a command talks to.
//!
//! # Responsibilities
//!
//! - [`Console`] holds whether stdin is a terminal, the line reader and the two output streams,
//!   so a command takes them as one value and a test passes buffers.
//! - It prints a line to either stream and prints a refusal's diagnostics, one per line, sorted.
//!
//! # Non-responsibilities
//!
//! Opening the real streams and detecting the terminal belong to `main`. A write that fails (a
//! closed pipe, say) is dropped: there is no better place left to report it.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::fmt::Display;
use std::io::{BufRead, Write};

use agentstacks::types::Diagnostics;

/// Stdin as a line reader with its terminal flag, stdout and stderr.
#[derive(Debug)]
pub(crate) struct Console<I, O, E> {
    /// Whether stdin is a terminal, so a question can be asked.
    pub(crate) is_terminal: bool,
    /// Where the answer to a question is read from.
    pub(crate) input: I,
    /// Plans, reports and results.
    pub(crate) out: O,
    /// Refusals, the question and the follow-up lines.
    pub(crate) err: E,
}

impl<I: BufRead, O: Write, E: Write> Console<I, O, E> {
    /// Writes `text` and a newline to stdout.
    pub(crate) fn out_line(&mut self, text: impl Display) {
        let _ = writeln!(self.out, "{text}");
    }

    /// Writes `text` and a newline to stderr.
    pub(crate) fn err_line(&mut self, text: impl Display) {
        let _ = writeln!(self.err, "{text}");
    }

    /// Writes each diagnostic to stderr, one per line, in `into_sorted` order.
    pub(crate) fn refuse(&mut self, diagnostics: Diagnostics) {
        for diagnostic in diagnostics.into_sorted() {
            self.err_line(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use agentstacks::types::{Diagnostic, FieldError, Location};

    use super::*;

    fn console() -> Console<&'static [u8], Vec<u8>, Vec<u8>> {
        Console {
            is_terminal: false,
            input: &b""[..],
            out: Vec::new(),
            err: Vec::new(),
        }
    }

    #[test]
    fn lines_go_to_their_own_stream() {
        let mut console = console();
        console.out_line("to out");
        console.err_line("to err");
        assert_eq!(console.out, b"to out\n");
        assert_eq!(console.err, b"to err\n");
    }

    #[test]
    fn refusals_print_sorted_one_per_line() {
        let mut found = Diagnostics::new();
        found.push(Diagnostic::new(Location::file("b.toml"), FieldError::Empty));
        found.push(Diagnostic::new(
            Location::file("a.toml"),
            FieldError::Required,
        ));
        let mut console = console();
        console.refuse(found);
        assert_eq!(console.err, b"a.toml: required\nb.toml: empty\n");
        assert!(console.out.is_empty());
    }
}
