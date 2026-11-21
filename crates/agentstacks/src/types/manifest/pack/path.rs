//! A path inside a pack, written relative to the pack's root.
//!
//! It is a type of its own because `render` and the hook `run` both hold pack-relative paths, and
//! one checked type keeps the path rules in one place.
//!
//! # Responsibilities
//!
//! - [`PackPath`] is a `/`-separated path that names a file inside the pack: it is relative, has
//!   no `.`, `..` or empty segment, does not end with `/`, is not `.` and uses no `\`. The text is
//!   kept as written.
//!
//! # Non-responsibilities
//!
//! Whether the file exists, and whether it is excluded, belongs to the checks over a loaded pack.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::types::manifest::InvalidPackPath;
use crate::types::report::MustBeRelative;

/// A relative path that stays inside its pack.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackPath;
///
/// let path = PackPath::new("hooks/enforce.lua")?;
/// assert_eq!(path.as_str(), "hooks/enforce.lua");
/// assert!(path.has_extension("lua"));
///
/// let refused = PackPath::new("a/./b").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"a/./b\" must not have a . segment");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackPath(String);

impl PackPath {
    /// Parses `text` as a pack-relative path.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPackPath::Empty`] for `""`, [`InvalidPackPath::IsDot`] for `.`,
    /// [`InvalidPackPath::MustBeRelative`] for a leading `/`, [`InvalidPackPath::Backslash`] for a
    /// `\`, [`InvalidPackPath::MustStayInsidePack`] for any `..` segment,
    /// [`InvalidPackPath::DotSegment`] for a `.` segment such as `./x` or `a/./b`,
    /// [`InvalidPackPath::TrailingSlash`] for a trailing `/`, and
    /// [`InvalidPackPath::EmptySegment`] for an empty segment such as `a//b`.
    pub fn new(text: &str) -> Result<Self, InvalidPackPath> {
        if text.is_empty() {
            return Err(InvalidPackPath::Empty);
        }
        if text == "." {
            return Err(InvalidPackPath::IsDot);
        }
        if text.starts_with('/') {
            return Err(MustBeRelative::new(text).into());
        }
        if text.contains('\\') {
            return Err(InvalidPackPath::Backslash {
                text: text.to_owned(),
            });
        }
        if text.split('/').any(|segment| segment == "..") {
            return Err(InvalidPackPath::MustStayInsidePack {
                text: text.to_owned(),
            });
        }
        if text.split('/').any(|segment| segment == ".") {
            return Err(InvalidPackPath::DotSegment {
                text: text.to_owned(),
            });
        }
        if text.ends_with('/') {
            return Err(InvalidPackPath::TrailingSlash {
                text: text.to_owned(),
            });
        }
        if text.split('/').any(str::is_empty) {
            return Err(InvalidPackPath::EmptySegment {
                text: text.to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// Whether the last segment ends in `.<extension>`, compared exactly.
    #[must_use]
    pub fn has_extension(&self, extension: &str) -> bool {
        Path::new(&self.0)
            .extension()
            .is_some_and(|found| found == extension)
    }

    /// The path as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PackPath {
    type Err = InvalidPackPath;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for PackPath {
    type Error = InvalidPackPath;

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
    fn pack_path_accepts_relative_paths() -> TestResult {
        for text in ["enforcement.json", "hooks/x.lua", "hooks/.keep"] {
            assert_eq!(PackPath::new(text)?.as_str(), text);
            assert_eq!(PackPath::new(text)?.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn pack_path_compares_the_extension_exactly() -> TestResult {
        let path = PackPath::new("hooks/x.lua")?;
        assert!(path.has_extension("lua"));
        assert!(!path.has_extension("sh"));
        assert!(!PackPath::new("hooks/xlua")?.has_extension("lua"));
        Ok(())
    }

    #[test]
    fn pack_path_refuses_absolute_and_escaping_paths() -> TestResult {
        let cases = [
            ("/x", "\"/x\" must be relative"),
            ("../x", "\"../x\" must stay inside the pack"),
            ("a/../../x", "\"a/../../x\" must stay inside the pack"),
            ("", "\"\" must not be empty"),
            (".", "\".\" must name a file inside the pack"),
            ("a//b", "\"a//b\" must not have an empty segment"),
            ("./x", "\"./x\" must not have a . segment"),
            ("a/./b", "\"a/./b\" must not have a . segment"),
            ("a/.", "\"a/.\" must not have a . segment"),
            ("dir/", "\"dir/\" must not end with /"),
            ("a\\b", "\"a\\b\" must use / as the separator"),
        ];
        for (text, message) in cases {
            let error = PackPath::new(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "render[0]");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!("packs/core/pack.toml: render[0]: {message}")
            );
        }
        Ok(())
    }

    #[test]
    fn pack_path_builds_alike_through_new_from_str_and_try_from() {
        fn shown(built: Result<PackPath, InvalidPackPath>) -> Result<String, String> {
            built
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
        for text in ["hooks/x.lua", "a/./b"] {
            let expected = shown(PackPath::new(text));
            assert_eq!(shown(text.parse::<PackPath>()), expected);
            assert_eq!(shown(PackPath::try_from(text)), expected);
        }
        assert!(PackPath::new("hooks/x.lua").is_ok());
        assert!(PackPath::new("a/./b").is_err());
    }
}
