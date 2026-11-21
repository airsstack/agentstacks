//! The fixed variables a `var` token can name.
//!
//! The variable names are a closed set, so an unknown one is refused when the token is read and
//! the list of known names in the refusal comes from one place.
//!
//! # Responsibilities
//!
//! - [`Var`] is the closed set of var names.
//! - [`Var::parse`] turns a token target into a var, and refuses an unknown name with the list of
//!   known ones.
//!
//! # Non-responsibilities
//!
//! Where a var may appear (`arguments` only in `SKILL.md`) belongs to the checks over a whole pack.

use std::fmt;

use crate::types::tokens::error::TokenError;

/// A fixed variable.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Var;
///
/// assert_eq!(Var::parse("project_dir")?, Var::ProjectDir);
/// assert_eq!(Var::Home.to_string(), "home");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Var {
    /// `session_id`
    SessionId,
    /// `arguments`
    Arguments,
    /// `tmpdir`
    Tmpdir,
    /// `home`
    Home,
    /// `airsstack_home`
    AirsstackHome,
    /// `project_dir`
    ProjectDir,
}

impl Var {
    /// Every var, in the order the refusal lists them.
    const ALL: [Self; 6] = [
        Self::SessionId,
        Self::Arguments,
        Self::Tmpdir,
        Self::Home,
        Self::AirsstackHome,
        Self::ProjectDir,
    ];

    /// Parses a var name.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::UnknownVar`] when `word` is not one of the var names.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Var;
    ///
    /// assert_eq!(Var::parse("arguments")?, Var::Arguments);
    /// let refused = Var::parse("cwd").err().map(|error| error.to_string());
    /// assert_eq!(
    ///     refused.as_deref(),
    ///     Some("unknown var \"cwd\" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)"),
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(word: &str) -> Result<Self, TokenError> {
        Self::ALL
            .into_iter()
            .find(|var| var.as_str() == word)
            .ok_or_else(|| TokenError::UnknownVar {
                name: word.to_owned(),
            })
    }

    /// The var name as written in a token.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Var;
    ///
    /// assert_eq!(Var::AirsstackHome.as_str(), "airsstack_home");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionId => "session_id",
            Self::Arguments => "arguments",
            Self::Tmpdir => "tmpdir",
            Self::Home => "home",
            Self::AirsstackHome => "airsstack_home",
            Self::ProjectDir => "project_dir",
        }
    }

    /// Every name joined with `, `, as the unknown-var refusal lists them.
    pub(super) fn known_names() -> String {
        Self::ALL.map(Self::as_str).join(", ")
    }
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_parses_each_known_name() {
        for var in Var::ALL {
            let name = var.to_string();
            assert_eq!(Var::parse(&name), Ok(var));
        }
        assert_eq!(
            Var::ALL.map(|v| v.to_string()),
            [
                "session_id",
                "arguments",
                "tmpdir",
                "home",
                "airsstack_home",
                "project_dir"
            ]
        );
    }

    #[test]
    fn var_refuses_unknown_with_known_list() {
        for name in ["session", "core/tmpdir"] {
            assert_eq!(
                Var::parse(name).map_err(|e| e.to_string()),
                Err(format!(
                    "unknown var \"{name}\" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)"
                ))
            );
        }
    }
}
