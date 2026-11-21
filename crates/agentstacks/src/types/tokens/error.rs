//! The refusals the token scanner owns.
//!
//! A token that cannot be read is refused with one of these messages, located by the diagnostic
//! that carries it.
//!
//! The scanner's refusals are one enum of their own, so every message of the token grammar is
//! listed in one place and pinned by its tests.
//!
//! # Responsibilities
//!
//! - [`TokenError`] has a variant per grammar refusal, each displaying its message without a
//!   location.
//! - Name refusals ([`InvalidName`], [`InvalidOptionName`]) pass through unchanged.
//!
//! # Non-responsibilities
//!
//! Refusals that need the pack or the file a token sits in (where a var may appear, whether a
//! target exists) belong to the checks over a whole pack and a whole set of packs.

use crate::types::name::InvalidName;
use crate::types::name::InvalidOptionName;
use crate::types::tokens::kind::TokenKind;
use crate::types::tokens::var::Var;

/// A token refused while it was read.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{TokenError, TokenKind};
///
/// assert_eq!(TokenError::Unterminated.to_string(), "unterminated token");
/// assert_eq!(TokenError::Empty.to_string(), "empty token");
/// assert_eq!(
///     TokenError::NeedsTarget { kind: TokenKind::Agent }.to_string(),
///     "token \"agent\" needs a target",
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    /// The first word is not a known token kind.
    #[error("unknown token kind \"{text}\"")]
    UnknownKind {
        /// The word that is not a kind.
        text: String,
    },
    /// An opening `{{` has no closing `}}` on its line.
    #[error("unterminated token")]
    Unterminated,
    /// Nothing but whitespace sits between the braces.
    #[error("empty token")]
    Empty,
    /// A kind that needs a target was given none.
    #[error("token \"{kind}\" needs a target")]
    NeedsTarget {
        /// The kind that needs a target.
        kind: TokenKind,
    },
    /// A kind was given two or more target words.
    #[error("token \"{kind}\" takes one target")]
    TakesOneTarget {
        /// The kind that takes one target.
        kind: TokenKind,
    },
    /// A target is not `[pack/]name`.
    #[error("\"{text}\" is not a valid target (expected [pack/]name)")]
    InvalidTarget {
        /// The target as written.
        text: String,
    },
    /// A pack or item name is not valid.
    #[error(transparent)]
    Name(#[from] InvalidName),
    /// A `root` token names something other than a pack.
    #[error("token \"root\" takes a pack name, not \"{text}\"")]
    RootTakesPackName {
        /// The target as written.
        text: String,
    },
    /// A `var` token names a var that does not exist.
    #[error("unknown var \"{text}\" (known: {})", Var::known_names())]
    UnknownVar {
        /// The name as written.
        text: String,
    },
    /// An option name is not valid.
    #[error(transparent)]
    OptionName(#[from] InvalidOptionName),
    /// An `option` token names an option of another pack.
    #[error("option tokens name this pack's options only")]
    OptionNotThisPack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_error_displays_each_refusal_message() {
        let cases: [(TokenError, &str); 11] = [
            (
                TokenError::UnknownKind {
                    text: "skil".to_owned(),
                },
                "unknown token kind \"skil\"",
            ),
            (TokenError::Unterminated, "unterminated token"),
            (TokenError::Empty, "empty token"),
            (
                TokenError::NeedsTarget {
                    kind: TokenKind::Skill,
                },
                "token \"skill\" needs a target",
            ),
            (
                TokenError::TakesOneTarget {
                    kind: TokenKind::Root,
                },
                "token \"root\" takes one target",
            ),
            (
                TokenError::InvalidTarget {
                    text: "a/b/c".to_owned(),
                },
                "\"a/b/c\" is not a valid target (expected [pack/]name)",
            ),
            (
                TokenError::Name(InvalidName::new("Core")),
                "\"Core\" is not a valid name",
            ),
            (
                TokenError::RootTakesPackName {
                    text: "core/x".to_owned(),
                },
                "token \"root\" takes a pack name, not \"core/x\"",
            ),
            (
                TokenError::UnknownVar {
                    text: "session".to_owned(),
                },
                "unknown var \"session\" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)",
            ),
            (
                TokenError::OptionName(InvalidOptionName::new("a-b")),
                "\"a-b\" is not a valid option name",
            ),
            (
                TokenError::OptionNotThisPack,
                "option tokens name this pack's options only",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
