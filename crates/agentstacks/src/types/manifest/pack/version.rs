//! A semantic version, held as one that parsed.
//!
//! It is separate because a pack, an extension and the locked airsl all hold a version, and all must
//! order and display it the same way.
//!
//! # Responsibilities
//!
//! - [`PackVersion`] is a semantic version (`major.minor.patch`, with optional pre-release and
//!   build parts) that parsed, so a held version is always valid. It orders by semver precedence
//!   and displays as written. A pack's `pack.version`, an extension's version, a locked entry's
//!   version and the locked airsl version all use it.
//! - Its refusal is [`InvalidVersion`]. The lock's `[airsl] version` replaces it with its own
//!   message, [`LockError::NotAVersion`](crate::types::LockError::NotAVersion).
//!
//! # Non-responsibilities
//!
//! Choosing between versions, and pinning one in the lock, belong to the code that holds the
//! versions.

use std::fmt;
use std::str::FromStr;

use semver::Version;

use crate::types::manifest::InvalidVersion;

/// A valid semantic version, such as a pack's, an extension's or an airsl's.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackVersion;
///
/// let version = PackVersion::new("0.4.2")?;
/// assert_eq!((version.major(), version.minor(), version.patch()), (0, 4, 2));
/// assert!(version < PackVersion::new("0.10.0")?);
///
/// let refused = PackVersion::new("1.2").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"1.2\" is not semver");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackVersion(Version);

impl PackVersion {
    /// Parses `text` as a semantic version.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidVersion`] when `text` is not a semantic version.
    pub fn new(text: &str) -> Result<Self, InvalidVersion> {
        Version::parse(text)
            .map(Self)
            .map_err(|_| InvalidVersion::new(text))
    }

    /// The version as the semver crate holds it.
    ///
    /// Crate-private so the semver type stays out of the public API; matching a version against a
    /// range is its caller.
    pub(crate) const fn as_version(&self) -> &Version {
        &self.0
    }

    /// The major number.
    #[must_use]
    pub const fn major(&self) -> u64 {
        self.0.major
    }

    /// The minor number.
    #[must_use]
    pub const fn minor(&self) -> u64 {
        self.0.minor
    }

    /// The patch number.
    #[must_use]
    pub const fn patch(&self) -> u64 {
        self.0.patch
    }
}

impl fmt::Display for PackVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for PackVersion {
    type Err = InvalidVersion;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for PackVersion {
    type Error = InvalidVersion;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn pack_version_parses_and_displays() -> TestResult {
        for text in ["0.4.2", "1.0.0-rc.1", "2.1.0+build.5"] {
            assert_eq!(PackVersion::new(text)?.to_string(), text);
        }
        let version = PackVersion::new("0.4.2")?;
        assert_eq!(
            (version.major(), version.minor(), version.patch()),
            (0, 4, 2)
        );
        Ok(())
    }

    #[test]
    fn pack_version_orders_by_precedence() -> TestResult {
        assert!(PackVersion::new("1.2.0")? < PackVersion::new("1.10.0")?);
        assert!(PackVersion::new("1.0.0-rc.1")? < PackVersion::new("1.0.0")?);
        Ok(())
    }

    #[test]
    fn pack_version_refuses_a_non_semver() -> TestResult {
        for text in ["1.2", "v1.0.0", ""] {
            let error = PackVersion::new(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "pack.version");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!("packs/core/pack.toml: pack.version: \"{text}\" is not semver")
            );
        }
        Ok(())
    }

    #[test]
    fn pack_version_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<PackVersion, InvalidVersion>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["0.4.2", "1.2"] {
            let expected = shown(PackVersion::new(text));
            assert_eq!(shown(text.parse::<PackVersion>()), expected);
            assert_eq!(shown(PackVersion::try_from(text)), expected);
        }
        assert!(PackVersion::new("0.4.2").is_ok());
        assert!(PackVersion::new("1.2").is_err());
    }
}
