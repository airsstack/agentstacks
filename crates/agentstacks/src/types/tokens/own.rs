//! Whether a pack named in a token or a hook `run` is the pack it sits in.
//!
//! Token checks and hook `run` checks both ask this question. It is one function so the two
//! cannot decide it differently.
//!
//! # Responsibilities
//!
//! - [`names_own_pack`] is the one rule for "this pack reference points back at its own pack": no
//!   pack named, or the pack's own name. The pack's own name is its `pack.toml` name; a pack set
//!   keys each pack by that same name, so a caller at either level passes the manifest name.
//!
//! # Non-responsibilities
//!
//! What follows from a reference to another pack (is it required, loaded, does it ship the item)
//! belongs to the checks over a whole set of packs.

use crate::types::name::PackName;

/// Whether `named`, the pack a token or hook `run` names, is the pack `own` it sits in: nothing
/// named, or `own` itself.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
/// use agentstacks::types::tokens::names_own_pack;
///
/// let core = PackName::new("core")?;
/// assert!(names_own_pack(None, &core));
/// assert!(names_own_pack(Some(&core), &core));
/// assert!(!names_own_pack(Some(&PackName::new("sdlc")?), &core));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn names_own_pack(named: Option<&PackName>, own: &PackName) -> bool {
    named.is_none_or(|pack| pack == own)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn no_pack_named_is_the_own_pack() -> TestResult {
        assert!(names_own_pack(None, &PackName::new("core")?));
        Ok(())
    }

    #[test]
    fn the_own_name_is_the_own_pack() -> TestResult {
        let core = PackName::new("core")?;
        assert!(names_own_pack(Some(&core), &core));
        Ok(())
    }

    #[test]
    fn another_name_is_another_pack() -> TestResult {
        let sdlc = PackName::new("sdlc")?;
        assert!(!names_own_pack(Some(&sdlc), &PackName::new("core")?));
        Ok(())
    }
}
