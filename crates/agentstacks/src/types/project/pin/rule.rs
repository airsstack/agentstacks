//! The pin rule a file applies to its git sources.
//!
//! The project file and the lock read their sources with one reader but pin them differently; the
//! rule is a value the caller passes, so the reader needs no knowledge of which file it serves.
//!
//! # Responsibilities
//!
//! - [`PinRule`] names the two rules and holds data only: `agentstacks.toml` pins by exactly one of
//!   `tag` and `rev`, while `agentstacks.lock` requires `rev` and keeps `tag` optional.
//!
//! # Non-responsibilities
//!
//! Applying the rule belongs to the reader of a source entry, [`SourceFields`](crate::types::SourceFields).

/// Which pin keys a git source must carry.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PinRule;
///
/// // The project file pins by exactly one key; the lock always records the commit.
/// let rule = PinRule::RevRequired;
/// assert_ne!(rule, PinRule::ExactlyOneOfTagRev);
/// let copy = rule;
/// assert_eq!(rule, copy);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinRule {
    /// Exactly one of `tag` and `rev`.
    ExactlyOneOfTagRev,
    /// `rev` is required and `tag` is optional.
    RevRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_rule_names_two_distinct_rules() {
        assert_ne!(PinRule::ExactlyOneOfTagRev, PinRule::RevRequired);
        let rule = PinRule::RevRequired;
        let copy = rule;
        assert_eq!(rule, copy);
    }
}
