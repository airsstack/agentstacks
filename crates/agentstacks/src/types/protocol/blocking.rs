//! Whether a hook may block.
//!
//! # Responsibilities
//!
//! - [`Blocking`] names the two answers and the argument the runtime reads to tell them apart.

/// Whether a hook was declared `may_block`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::protocol::Blocking;
///
/// assert_eq!(Blocking::from(true).as_arg(), "1");
/// assert_eq!(Blocking::from(false).as_arg(), "0");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocking {
    /// The hook is declared `may_block`; the runtime passes its block on.
    May,
    /// The hook is not declared `may_block`; the runtime drops its block.
    MayNot,
}

impl Blocking {
    /// The command-line argument that carries this answer.
    #[must_use]
    pub const fn as_arg(self) -> &'static str {
        match self {
            Self::May => "1",
            Self::MayNot => "0",
        }
    }
}

impl From<bool> for Blocking {
    fn from(may_block: bool) -> Self {
        if may_block { Self::May } else { Self::MayNot }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_args() {
        assert_eq!(Blocking::May.as_arg(), "1");
        assert_eq!(Blocking::MayNot.as_arg(), "0");
        assert_eq!(Blocking::from(true), Blocking::May);
        assert_eq!(Blocking::from(false), Blocking::MayNot);
    }
}
