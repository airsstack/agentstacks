//! The message a TOML or YAML parser gave when it refused a file.
//!
//! A parser's message is text this crate does not word, so it gets one type of its own. Every
//! refusal that carries a parser's words holds this type rather than a bare `String`, whichever
//! parser produced them.
//!
//! # Responsibilities
//!
//! - [`ParserMessage`] holds the parser's message unchanged and displays it byte for byte.
//!
//! # Non-responsibilities
//!
//! Locating the message (the file and line) belongs to the reader that ran the parser, and any
//! hint added after the message belongs to the error that carries it.

use std::fmt;

/// A parser's own message, kept and displayed unchanged.
///
/// # Examples
///
/// A TOML syntax refusal carries the parser's message:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{FormatError, ProjectFile};
///
/// let refused = ProjectFile::parse(Path::new("agentstacks.toml"), "api = ")
///     .err()
///     .ok_or("expected a refusal")?;
/// let diagnostic = refused.into_sorted().into_iter().next().ok_or("expected a diagnostic")?;
/// let FormatError::TomlSyntax(message) = diagnostic.error() else {
///     return Err("expected a syntax refusal".into());
/// };
/// assert!(!message.as_str().is_empty());
/// assert_eq!(diagnostic.error().to_string(), message.as_str());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserMessage(String);

impl ParserMessage {
    /// Keeps `message`, the parser's words, unchanged.
    pub(crate) fn new(message: &str) -> Self {
        Self(message.to_owned())
    }

    /// The message as the parser wrote it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ParserMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_message_displays_the_text_unchanged() {
        for text in [
            "expected string scalar",
            "unclosed bracket '['",
            "",
            "  a\tb  ",
        ] {
            let message = ParserMessage::new(text);
            assert_eq!(message.as_str(), text);
            assert_eq!(message.to_string(), text);
        }
    }
}
