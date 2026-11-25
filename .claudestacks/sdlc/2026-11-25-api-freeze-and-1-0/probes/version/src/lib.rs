//! Probe: the running version as a const `semver::Version`.
use semver::{Version, VersionReq};

const fn number(text: &str) -> u64 {
    match u64::from_str_radix(text, 10) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

/// This crate's version.
pub const VERSION: Version = Version::new(
    number(env!("CARGO_PKG_VERSION_MAJOR")),
    number(env!("CARGO_PKG_VERSION_MINOR")),
    number(env!("CARGO_PKG_VERSION_PATCH")),
);

/// Whether `range` includes this version.
#[must_use]
pub fn includes(range: &VersionReq) -> bool {
    range.matches(&VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_is_the_package_version() -> Result<(), semver::Error> {
        assert_eq!(VERSION.to_string(), env!("CARGO_PKG_VERSION"));
        assert!(includes(&VersionReq::parse(">=1.0")?));
        assert!(!includes(&VersionReq::parse(">=1.3")?));
        Ok(())
    }
}
