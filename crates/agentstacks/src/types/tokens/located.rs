//! A token together with where it was found.
//!
//! A [`Token`] holds only its byte span. The scanner knows the text and where it came from, so it
//! builds the [`Location`] and keeps it beside the token, and no later check has to recompute it.
//!
//! # Responsibilities
//!
//! - [`LocatedToken`] pairs a [`Token`] with the [`Location`] a refusal about it is reported at.
//!
//! # Non-responsibilities
//!
//! It carries no rule. The rules that read it (where a var may appear, whether a target exists,
//! whether a root token reaches `native/`) belong to the checks over a whole pack and a whole set
//! of packs.

use crate::types::report::Location;
use crate::types::tokens::parsed::Token;

/// A [`Token`] and its [`Location`].
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{TextOrigin, scan};
///
/// let origin = TextOrigin::Field { file: "pack.toml".into(), key_path: "hooks[0].text".to_owned() };
/// let (tokens, _) = scan("{{var home}}", &origin);
/// let located = tokens.first().ok_or("no token found")?;
/// assert_eq!(located.token().to_string(), "{{var home}}");
/// assert_eq!(located.location().to_string(), "pack.toml: hooks[0].text");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedToken {
    token: Token,
    location: Location,
}

impl LocatedToken {
    /// A located token.
    pub(crate) const fn new(token: Token, location: Location) -> Self {
        Self { token, location }
    }

    /// The token.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{skill review}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// assert_eq!(located.token().to_string(), "{{skill review}}");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn token(&self) -> &Token {
        &self.token
    }

    /// Where the token was found.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("one\n{{var home}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// assert_eq!(located.location().to_string(), "a.md:2");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn location(&self) -> &Location {
        &self.location
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use crate::types::{TextOrigin, TokenKind, scan};

    #[test]
    fn located_token_exposes_token_and_location() {
        let origin = TextOrigin::Lines {
            file: PathBuf::from("notes.md"),
            first_line: NonZeroUsize::new(3).unwrap_or(NonZeroUsize::MIN),
        };
        let (tokens, diagnostics) = scan("{{skill status}}", &origin);
        assert!(diagnostics.is_empty());
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens.first().map(|t| t.token().kind()),
            Some(TokenKind::Skill)
        );
        assert_eq!(
            tokens.first().map(|t| t.location().to_string()),
            Some("notes.md:3".to_owned())
        );
    }
}
