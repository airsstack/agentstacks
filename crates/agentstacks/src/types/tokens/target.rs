//! What a token points at, one variant per kind.
//!
//! Each kind of token takes a different target. One enum with a variant per kind keeps a kind
//! and its target together, so no caller can build a pair that disagrees.
//!
//! # Responsibilities
//!
//! - [`TokenTarget`] has a variant per [`TokenKind`], so a kind and its target cannot disagree.
//! - [`TokenTarget::parse`] applies the grammar rules for a kind and its target words.
//!
//! # Non-responsibilities
//!
//! Whether the target exists, and where a target may appear, belong to the checks over a whole pack
//! and a whole set of packs.

use std::fmt;

use crate::types::name::OptionName;
use crate::types::name::PackName;
use crate::types::tokens::error::TokenError;
use crate::types::tokens::kind::TokenKind;
use crate::types::tokens::reference::ItemRef;
use crate::types::tokens::var::Var;

/// The target of a token.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{TokenKind, TokenTarget};
///
/// let target = TokenTarget::parse(TokenKind::Skill, &["core/review"])?;
/// assert_eq!(target.to_string(), "core/review");
/// assert_eq!(TokenTarget::parse(TokenKind::Root, &[])?.to_string(), "");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenTarget {
    /// A skill, `[pack/]item`.
    Skill(ItemRef),
    /// An agent, `[pack/]item`.
    Agent(ItemRef),
    /// The directory of a skill, `[pack/]item`.
    SkillDir(ItemRef),
    /// The install root of a pack; `None` is the pack the token sits in.
    Root(Option<PackName>),
    /// A fixed variable.
    Var(Var),
    /// An option of the pack the token sits in.
    Option(OptionName),
}

impl TokenTarget {
    /// Parses the words after the kind of a token.
    ///
    /// # Errors
    ///
    /// Returns the [`TokenError`] for the first rule `words` breaks: no word for a kind that needs
    /// a target, two or more words for any kind, or a word that is not a valid target for the
    /// kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{TokenKind, TokenTarget};
    ///
    /// assert!(TokenTarget::parse(TokenKind::Option, &["verbose"]).is_ok());
    /// let refused = TokenTarget::parse(TokenKind::Skill, &[]).err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("token \"skill\" needs a target"));
    /// ```
    pub fn parse(kind: TokenKind, words: &[&str]) -> Result<Self, TokenError> {
        let word = match words {
            [] => {
                return match kind {
                    TokenKind::Root => Ok(Self::Root(None)),
                    _ => Err(TokenError::NeedsTarget { kind }),
                };
            }
            [word] => *word,
            _ => return Err(TokenError::TakesOneTarget { kind }),
        };
        match kind {
            TokenKind::Skill => Ok(Self::Skill(ItemRef::parse(word)?)),
            TokenKind::Agent => Ok(Self::Agent(ItemRef::parse(word)?)),
            TokenKind::SkillDir => Ok(Self::SkillDir(ItemRef::parse(word)?)),
            TokenKind::Root if word.contains('/') => Err(TokenError::RootTakesPackName {
                target: word.to_owned(),
            }),
            TokenKind::Root => Ok(Self::Root(Some(PackName::new(word)?))),
            TokenKind::Var => Ok(Self::Var(Var::parse(word)?)),
            TokenKind::Option if word.contains('/') => Err(TokenError::OptionNotThisPack),
            TokenKind::Option => Ok(Self::Option(OptionName::new(word)?)),
        }
    }

    /// The kind this target belongs to.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{TokenKind, TokenTarget};
    ///
    /// let target = TokenTarget::parse(TokenKind::Var, &["tmpdir"])?;
    /// assert_eq!(target.kind(), TokenKind::Var);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn kind(&self) -> TokenKind {
        match self {
            Self::Skill(_) => TokenKind::Skill,
            Self::Agent(_) => TokenKind::Agent,
            Self::SkillDir(_) => TokenKind::SkillDir,
            Self::Root(_) => TokenKind::Root,
            Self::Var(_) => TokenKind::Var,
            Self::Option(_) => TokenKind::Option,
        }
    }
}

impl fmt::Display for TokenTarget {
    /// Writes the target as it appears in a token; `root` with no pack writes nothing.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Skill(item) | Self::Agent(item) | Self::SkillDir(item) => item.fmt(f),
            Self::Root(Some(pack)) => pack.fmt(f),
            Self::Root(None) => Ok(()),
            Self::Var(var) => var.fmt(f),
            Self::Option(option) => option.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_shown(kind: TokenKind, words: &[&str]) -> Result<String, String> {
        TokenTarget::parse(kind, words)
            .map(|t| t.to_string())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn token_target_parses_per_kind() {
        let cases: [(TokenKind, &[&str], &str); 7] = [
            (TokenKind::Skill, &["sdlc/status"], "sdlc/status"),
            (TokenKind::Agent, &["coder"], "coder"),
            (TokenKind::SkillDir, &["discuss"], "discuss"),
            (TokenKind::Root, &[], ""),
            (TokenKind::Root, &["core"], "core"),
            (TokenKind::Var, &["tmpdir"], "tmpdir"),
            (TokenKind::Option, &["archive_keep"], "archive_keep"),
        ];
        for (kind, words, shown) in cases {
            assert_eq!(parse_shown(kind, words), Ok(shown.to_owned()), "{kind}");
            assert_eq!(
                TokenTarget::parse(kind, words).map(|t| t.kind()),
                Ok(kind),
                "{kind}"
            );
        }
        assert!(matches!(
            TokenTarget::parse(TokenKind::Root, &[]),
            Ok(TokenTarget::Root(None))
        ));
        assert!(matches!(
            TokenTarget::parse(TokenKind::Root, &["core"]),
            Ok(TokenTarget::Root(Some(_)))
        ));
    }

    #[test]
    fn token_target_refuses_missing_or_extra_targets() {
        for kind in [
            TokenKind::Skill,
            TokenKind::Agent,
            TokenKind::SkillDir,
            TokenKind::Var,
            TokenKind::Option,
        ] {
            assert_eq!(
                parse_shown(kind, &[]),
                Err(format!("token \"{kind}\" needs a target"))
            );
        }
        assert_eq!(
            parse_shown(TokenKind::Root, &["core", "extra"]),
            Err("token \"root\" takes one target".to_owned())
        );
        let extra: [(TokenKind, &[&str]); 5] = [
            (TokenKind::Skill, &["a", "b"]),
            (TokenKind::Agent, &["a", "b"]),
            (TokenKind::SkillDir, &["a", "b"]),
            (TokenKind::Var, &["tmpdir", "home"]),
            (TokenKind::Option, &["a", "b"]),
        ];
        for (kind, words) in extra {
            assert_eq!(
                parse_shown(kind, words),
                Err(format!("token \"{kind}\" takes one target"))
            );
        }
    }

    #[test]
    fn token_target_refuses_root_var_and_option_shapes() {
        let known = "session_id, arguments, tmpdir, home, airsstack_home, project_dir";
        let cases: [(TokenKind, &str, String); 5] = [
            (
                TokenKind::Root,
                "core/x",
                "token \"root\" takes a pack name, not \"core/x\"".to_owned(),
            ),
            (
                TokenKind::Root,
                "Core",
                "\"Core\" is not a valid name".to_owned(),
            ),
            (
                TokenKind::Var,
                "core/tmpdir",
                format!("unknown var \"core/tmpdir\" (known: {known})"),
            ),
            (
                TokenKind::Option,
                "a-b",
                "\"a-b\" is not a valid option name".to_owned(),
            ),
            (
                TokenKind::Option,
                "core/archive_keep",
                "option tokens name this pack's options only".to_owned(),
            ),
        ];
        for (kind, word, message) in cases {
            assert_eq!(parse_shown(kind, &[word]), Err(message), "{kind} {word}");
        }
    }
}
