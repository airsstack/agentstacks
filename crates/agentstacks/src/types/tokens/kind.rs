//! The kinds of token.
//!
//! The keywords are matched in one place so the scanner and the target grammar agree on which
//! kinds exist and which of them need a target.
//!
//! # Responsibilities
//!
//! - [`TokenKind`] is the closed set of token keywords: `skill`, `agent`, `skill_dir`, `root`,
//!   `var` and `option`.
//! - [`TokenKind::new`] turns the first word of a token into a kind.
//! - [`TokenKind::needs_target`] says whether a kind must be followed by a target.
//!
//! # Non-responsibilities
//!
//! What a target may be per kind belongs to [`TokenTarget`](crate::types::TokenTarget).

use std::fmt;
use std::str::FromStr;

use crate::types::tokens::error::TokenError;

/// The kind a token names in its first word.
///
/// # Examples
///
/// ```
/// use agentstacks::types::TokenKind;
///
/// assert_eq!(TokenKind::new("skill_dir")?, TokenKind::SkillDir);
/// assert_eq!(TokenKind::SkillDir.to_string(), "skill_dir");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A skill of this or another pack.
    Skill,
    /// An agent of this or another pack.
    Agent,
    /// The directory of a skill.
    SkillDir,
    /// The install root of a pack.
    Root,
    /// A fixed variable.
    Var,
    /// An option of this pack.
    Option,
}

impl TokenKind {
    /// Parses the keyword that opens a token.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::UnknownKind`] when `word` is not one of the token keywords.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::TokenKind;
    ///
    /// assert_eq!(TokenKind::new("agent")?, TokenKind::Agent);
    /// let refused = TokenKind::new("bogus").err().map(|error| error.to_string());
    /// assert_eq!(refused.as_deref(), Some("unknown token kind \"bogus\""));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(word: &str) -> Result<Self, TokenError> {
        match word {
            "skill" => Ok(Self::Skill),
            "agent" => Ok(Self::Agent),
            "skill_dir" => Ok(Self::SkillDir),
            "root" => Ok(Self::Root),
            "var" => Ok(Self::Var),
            "option" => Ok(Self::Option),
            _ => Err(TokenError::UnknownKind {
                text: word.to_owned(),
            }),
        }
    }

    /// Whether a token of this kind must name a target: true for every kind but `root`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::TokenKind;
    ///
    /// assert!(TokenKind::Skill.needs_target());
    /// assert!(!TokenKind::Root.needs_target());
    /// ```
    #[must_use]
    pub const fn needs_target(self) -> bool {
        !matches!(self, Self::Root)
    }

    /// The keyword as written in a token.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::TokenKind;
    ///
    /// assert_eq!(TokenKind::Option.as_str(), "option");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
            Self::SkillDir => "skill_dir",
            Self::Root => "root",
            Self::Var => "var",
            Self::Option => "option",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TokenKind {
    type Err = TokenError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for TokenKind {
    type Error = TokenError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [TokenKind; 6] = [
        TokenKind::Skill,
        TokenKind::Agent,
        TokenKind::SkillDir,
        TokenKind::Root,
        TokenKind::Var,
        TokenKind::Option,
    ];

    #[test]
    fn token_kind_parses_each_keyword() {
        for word in ["skill", "agent", "skill_dir", "root", "var", "option"] {
            let parsed = TokenKind::new(word);
            assert_eq!(
                parsed.as_ref().map(ToString::to_string),
                Ok(word.to_owned())
            );
        }
        let parsed: Vec<_> = ["skill", "agent", "skill_dir", "root", "var", "option"]
            .into_iter()
            .map(TokenKind::new)
            .collect();
        assert_eq!(parsed, ALL.map(Ok).to_vec());
    }

    #[test]
    fn token_kind_refuses_unknown_word() {
        assert_eq!(
            TokenKind::new("skil").map_err(|e| e.to_string()),
            Err("unknown token kind \"skil\"".to_owned())
        );
    }

    #[test]
    fn token_kind_says_which_kinds_need_a_target() {
        for kind in [
            TokenKind::Skill,
            TokenKind::Agent,
            TokenKind::SkillDir,
            TokenKind::Var,
            TokenKind::Option,
        ] {
            assert!(kind.needs_target(), "{kind}");
        }
        assert!(!TokenKind::Root.needs_target());
    }

    #[test]
    fn token_kind_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<TokenKind, TokenError>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["agent", "bogus"] {
            let expected = shown(TokenKind::new(text));
            assert_eq!(shown(text.parse::<TokenKind>()), expected);
            assert_eq!(shown(TokenKind::try_from(text)), expected);
        }
        assert!(TokenKind::new("agent").is_ok());
        assert!(TokenKind::new("bogus").is_err());
    }
}
