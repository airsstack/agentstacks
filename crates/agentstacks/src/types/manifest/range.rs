//! The version range a `requires` table holds, for `requires.airsl` and `requires.agentstacks`.
//!
//! It is a type of its own, not a string in `Requires`, because the operator allow-list and the as-
//! written text are rules of the range itself: every holder of a range gets the same operator check
//! instead of re-checking text.
//!
//! # Responsibilities
//!
//! - [`VersionRange`] holds a `semver::VersionReq` that parsed, together with the text as written.
//! - It accepts only the eight operators the range intersection understands: exact, greater,
//!   greater-or-equal, less, less-or-equal, tilde, caret and wildcard. `*` has no comparators and
//!   is accepted as unbounded.
//! - `VersionRange::read` reads one optional range key of a manifest table, for the pack and the
//!   extension manifest alike, refusing it in the error type of the file being read.
//! - `Display` gives the range as written, because messages about a range quote what the author
//!   wrote and the semver `Display` normalises it (`>= 0.1.4` prints `>=0.1.4`).
//!
//! # Non-responsibilities
//!
//! Intersecting ranges and comparing them with the installed airsl or agentstacks belongs to the
//! checks over a whole set of packs and to the loaders; this type only guarantees what a held range
//! may contain.

use std::fmt;
use std::str::FromStr;

use semver::{Op, VersionReq};

use crate::types::format::TomlFields;
use crate::types::manifest::InvalidRange;
use crate::types::report::FormatError;

/// A `requires` version range whose operators are all supported.
///
/// # Examples
///
/// ```
/// use agentstacks::types::VersionRange;
///
/// let req = VersionRange::new(">= 0.1.4")?;
/// assert_eq!(req.written(), ">= 0.1.4");
///
/// let refused = VersionRange::new("nope").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"nope\" is not a version range");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    written: String,
    req: VersionReq,
}

impl VersionRange {
    /// Parses `text` as a version range.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRange::NotVersionRange`] when `text` is not a range, and
    /// [`InvalidRange::UnsupportedOperator`] when a comparator uses an operator outside the
    /// supported eight.
    pub fn new(text: &str) -> Result<Self, InvalidRange> {
        let req = VersionReq::parse(text).map_err(|_| InvalidRange::NotVersionRange {
            text: text.to_owned(),
        })?;
        if !req.comparators.iter().all(|c| is_supported(c.op)) {
            return Err(InvalidRange::UnsupportedOperator {
                text: text.to_owned(),
            });
        }
        Ok(Self {
            written: text.to_owned(),
            req,
        })
    }

    /// Reads the optional range at `key` of `fields`, refusing a value that is not a string or not
    /// a range.
    ///
    /// A value that is not a range is refused at `key` as `E`, the error of the file being read, so
    /// the pack and the extension manifest word a bad range the same way.
    pub(in crate::types) fn read<E>(fields: &mut TomlFields, key: &str) -> Option<Self>
    where
        E: From<InvalidRange> + Into<FormatError>,
    {
        fields
            .optional_string(key)
            .and_then(|text| match Self::new(&text) {
                Ok(range) => Some(range),
                Err(error) => {
                    fields.refuse(key, E::from(error));
                    None
                }
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

impl fmt::Display for VersionRange {
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

impl FromStr for VersionRange {
    type Err = InvalidRange;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for VersionRange {
    type Error = InvalidRange;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn refused(text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let error = VersionRange::new(text).err().ok_or("expected a refusal")?;
        let location = Location::at_key("packs/core/pack.toml", "requires.airsl");
        Ok(Diagnostic::new(location, ManifestError::from(error)).to_string())
    }

    #[test]
    fn version_range_accepts_cited_ranges() -> TestResult {
        for text in [">=0.1.4", ">= 0.1.4", "0.1", "*"] {
            let req = VersionRange::new(text)?;
            assert_eq!(req.to_string(), text);
            assert_eq!(req.written(), text);
        }
        assert!(
            VersionRange::new("*")?
                .as_version_req()
                .comparators
                .is_empty()
        );
        // The semver `Display` normalises the spelling; `Display` of the type keeps what was written.
        assert_eq!(
            VersionRange::new(">= 0.1.4")?.as_version_req().to_string(),
            ">=0.1.4"
        );
        Ok(())
    }

    #[test]
    fn version_range_refuses_a_non_range() -> TestResult {
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
        let error = ManifestError::from(InvalidRange::UnsupportedOperator {
            text: ">=0.1".to_owned(),
        });
        let location = Location::at_key("packs/core/pack.toml", "requires.airsl");
        assert_eq!(
            Diagnostic::new(location, error).to_string(),
            "packs/core/pack.toml: requires.airsl: \">=0.1\" uses an operator this agentstacks does not support"
        );
    }

    #[test]
    fn version_range_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<VersionRange, InvalidRange>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in [">= 0.1.4", "nope"] {
            let expected = shown(VersionRange::new(text));
            assert_eq!(shown(text.parse::<VersionRange>()), expected);
            assert_eq!(shown(VersionRange::try_from(text)), expected);
        }
        assert!(VersionRange::new(">= 0.1.4").is_ok());
        assert!(VersionRange::new("nope").is_err());
    }
}
