//! The airsl version range a pack requires.
//!
//! It is a type of its own, not a string in `Requires`, because the operator allow-list and the as-
//! written text are rules of the range itself: every holder of a range gets the same operator check
//! instead of re-checking text.
//!
//! # Responsibilities
//!
//! - [`AirslReq`] holds a `semver::VersionReq` that parsed, together with the text as written.
//! - It accepts only the eight operators the range intersection understands: exact, greater,
//!   greater-or-equal, less, less-or-equal, tilde, caret and wildcard. `*` has no comparators and
//!   is accepted as unbounded.
//! - `Display` gives the range as written, because messages about a range quote what the author
//!   wrote and the semver `Display` normalises it (`>= 0.1.4` prints `>=0.1.4`).
//!
//! # Non-responsibilities
//!
//! Intersecting ranges and comparing them with the installed airsl belongs to the checks over a
//! whole set of packs; this type only guarantees what a held range may contain.

use std::fmt;

use semver::{Op, VersionReq};

use crate::types::manifest::InvalidAirslReq;

/// A `requires.airsl` range whose operators are all supported.
///
/// # Examples
///
/// ```
/// use agentstacks::types::AirslReq;
///
/// let req = AirslReq::new(">= 0.1.4")?;
/// assert_eq!(req.written(), ">= 0.1.4");
///
/// let refused = AirslReq::new("nope").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"nope\" is not a version range");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirslReq {
    written: String,
    req: VersionReq,
}

impl AirslReq {
    /// Parses `text` as a version range.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidAirslReq::NotVersionRange`] when `text` is not a range, and
    /// [`InvalidAirslReq::UnsupportedOperator`] when a comparator uses an operator outside the
    /// supported eight.
    pub fn new(text: &str) -> Result<Self, InvalidAirslReq> {
        let req = VersionReq::parse(text).map_err(|_| InvalidAirslReq::NotVersionRange {
            text: text.to_owned(),
        })?;
        if !req.comparators.iter().all(|c| is_supported(c.op)) {
            return Err(InvalidAirslReq::UnsupportedOperator {
                text: text.to_owned(),
            });
        }
        Ok(Self {
            written: text.to_owned(),
            req,
        })
    }

    /// The range as the semver crate holds it.
    ///
    /// Crate-private so the semver type stays out of the public API; the range intersection over a
    /// set of packs is its caller.
    #[must_use]
    pub(crate) const fn as_version_req(&self) -> &VersionReq {
        &self.req
    }

    /// The range as written.
    #[must_use]
    pub fn written(&self) -> &str {
        &self.written
    }
}

impl fmt::Display for AirslReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written)
    }
}

/// Whether `op` is one of the eight operators the range intersection understands.
///
/// `Op` is `#[non_exhaustive]`, so an operator a later semver release adds lands in the fallback
/// arm and is refused instead of being accepted silently.
const fn is_supported(op: Op) -> bool {
    matches!(
        op,
        Op::Exact
            | Op::Greater
            | Op::GreaterEq
            | Op::Less
            | Op::LessEq
            | Op::Tilde
            | Op::Caret
            | Op::Wildcard
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn refused(text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let error = AirslReq::new(text).err().ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "requires.airsl");
        Ok(Diagnostic::new(location, ManifestError::from(error)).to_string())
    }

    #[test]
    fn airsl_req_accepts_cited_ranges() -> TestResult {
        for text in [">=0.1.4", ">= 0.1.4", "0.1", "*"] {
            let req = AirslReq::new(text)?;
            assert_eq!(req.to_string(), text);
            assert_eq!(req.written(), text);
        }
        assert!(AirslReq::new("*")?.as_version_req().comparators.is_empty());
        // The semver `Display` normalises the spelling; `Display` of the type keeps what was written.
        assert_eq!(
            AirslReq::new(">= 0.1.4")?.as_version_req().to_string(),
            ">=0.1.4"
        );
        Ok(())
    }

    #[test]
    fn airsl_req_refuses_a_non_range() -> TestResult {
        assert_eq!(
            refused("nope")?,
            "packs/core/pack.toml: requires.airsl: \"nope\" is not a version range"
        );
        assert_eq!(
            refused("")?,
            "packs/core/pack.toml: requires.airsl: \"\" is not a version range"
        );
        Ok(())
    }

    #[test]
    fn operator_check_accepts_the_eight_operators() {
        for op in [
            Op::Exact,
            Op::Greater,
            Op::GreaterEq,
            Op::Less,
            Op::LessEq,
            Op::Tilde,
            Op::Caret,
            Op::Wildcard,
        ] {
            assert!(is_supported(op), "{op:?}");
        }
    }

    #[test]
    fn unsupported_operator_message_names_the_range() {
        let error = ManifestError::from(InvalidAirslReq::UnsupportedOperator {
            text: ">=0.1".to_owned(),
        });
        let location = Location::at_key("packs/core/pack.toml", "requires.airsl");
        assert_eq!(
            Diagnostic::new(location, error).to_string(),
            "packs/core/pack.toml: requires.airsl: \">=0.1\" uses an operator this agentstacks does not support"
        );
    }
}
