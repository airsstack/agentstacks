//! The kind of item a pack ships that content can point at.
//!
//! It is its own file because the token rules, the hook rules and the refusal all name the kind, and
//! none of them may own a type the others depend on.
//!
//! # Responsibilities
//!
//! - [`ItemKind`] names the two kinds of item a token or a hook can point at: a skill and an
//!   agent, and displays them as the words a refusal uses.
//!
//! # Non-responsibilities
//!
//! Finding the items of a pack belongs to the layout, and the refusal that names one belongs to
//! [`PackError`](crate::types::PackError).

use std::fmt;

/// The kind of item a pack ships that content can point at.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ItemKind;
///
/// assert_eq!(ItemKind::Skill.to_string(), "skill");
/// assert_eq!(ItemKind::Agent.to_string(), "agent");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A skill directory.
    Skill,
    /// An agent file.
    Agent,
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_kind_displays_the_word_a_refusal_uses() {
        assert_eq!(ItemKind::Skill.to_string(), "skill");
        assert_eq!(ItemKind::Agent.to_string(), "agent");
    }
}
