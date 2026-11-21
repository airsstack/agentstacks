//! One token found in scanned text.
//!
//! A token is its own type, apart from the scanner, because the scanner only produces tokens;
//! rendering, values and the checks over a pack read them without scanning anything.
//!
//! # Responsibilities
//!
//! - [`Token`] holds a target, the byte span of the whole `{{...}}` in the scanned text, and whether
//!   a `root` token reaches `native/` (see [`scan`](super::scan::scan)).
//! - Its `Display` is the canonical form `{{<kind> <target>}}`, or `{{root}}` for `root` with no
//!   pack.
//!
//! # Non-responsibilities
//!
//! Finding tokens belongs to [`scan`](crate::types::tokens::scan::scan), and the rules over stored tokens belong
//! to the checks over a whole pack and a whole set of packs.

use std::fmt;
use std::ops::Range;

use crate::types::tokens::kind::TokenKind;
use crate::types::tokens::target::TokenTarget;

/// A token read from scanned text.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{TextOrigin, TokenKind, scan};
///
/// let origin = TextOrigin::Lines { file: "SKILL.md".into(), first_line: NonZeroUsize::MIN };
/// let (tokens, _) = scan("{{agent core/reviewer}}", &origin);
/// let located = tokens.first().ok_or("no token found")?;
/// assert_eq!(located.token().kind(), TokenKind::Agent);
/// assert_eq!(located.token().to_string(), "{{agent core/reviewer}}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    target: TokenTarget,
    span: Range<usize>,
    reaches_native: bool,
}

impl Token {
    /// A token with `target` over `span` of the scanned text.
    pub(crate) const fn new(target: TokenTarget, span: Range<usize>, reaches_native: bool) -> Self {
        Self {
            target,
            span,
            reaches_native,
        }
    }

    /// What the token points at.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, TokenTarget, Var, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{var home}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// assert_eq!(located.token().target(), &TokenTarget::Var(Var::Home));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn target(&self) -> &TokenTarget {
        &self.target
    }

    /// The token's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, TokenKind, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{option verbose}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// assert_eq!(located.token().kind(), TokenKind::Option);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn kind(&self) -> TokenKind {
        self.target.kind()
    }

    /// The byte range of the whole `{{...}}` in the scanned text.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("at {{var home}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// assert_eq!(located.token().span(), 3..15);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Whether this is a `root` token whose scanned text continues with `/native` as a whole path
    /// segment, however many `/` precede it.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{root}}/native/hooks.toml {{root}}/skills", &origin);
    /// let reaches: Vec<bool> = tokens.iter().map(|located| located.token().reaches_native()).collect();
    /// assert_eq!(reaches, [true, false]);
    /// ```
    #[must_use]
    pub const fn reaches_native(&self) -> bool {
        self.reaches_native
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{{")?;
        write!(f, "{}", self.kind())?;
        let target = self.target.to_string();
        if !target.is_empty() {
            write!(f, " {target}")?;
        }
        f.write_str("}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(kind: TokenKind, words: &[&str]) -> Option<Token> {
        TokenTarget::new(kind, words)
            .ok()
            .map(|target| Token::new(target, 0..1, false))
    }

    #[test]
    fn token_displays_canonical_form() {
        let cases: [(TokenKind, &[&str], &str); 4] = [
            (TokenKind::Skill, &["sdlc/status"], "{{skill sdlc/status}}"),
            (TokenKind::Root, &[], "{{root}}"),
            (TokenKind::Root, &["core"], "{{root core}}"),
            (TokenKind::Var, &["tmpdir"], "{{var tmpdir}}"),
        ];
        for (kind, words, shown) in cases {
            assert_eq!(
                token(kind, words).map(|t| t.to_string()),
                Some(shown.to_owned())
            );
        }
    }

    #[test]
    fn accessors_return_constructed_parts() {
        let built = TokenTarget::new(TokenKind::Root, &[])
            .ok()
            .map(|target| Token::new(target, 4..12, true));
        assert_eq!(built.as_ref().map(Token::kind), Some(TokenKind::Root));
        assert_eq!(built.as_ref().map(Token::span), Some(4..12));
        assert_eq!(built.as_ref().map(Token::reaches_native), Some(true));
        assert_eq!(
            built.as_ref().map(|t| t.target().clone()),
            Some(TokenTarget::Root(None))
        );
    }
}
