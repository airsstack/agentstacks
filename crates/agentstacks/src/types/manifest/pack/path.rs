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

use crate::types::manifest::InvalidPackPath;

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
        let path = || text.to_owned();
        if text.is_empty() {
            return Err(InvalidPackPath::Empty);
        }
        if text == "." {
            return Err(InvalidPackPath::IsDot);
        }
        if text.starts_with('/') {
            return Err(InvalidPackPath::MustBeRelative { path: path() });
        }
        if text.contains('\\') {
            return Err(InvalidPackPath::Backslash { path: path() });
        }
        if text.split('/').any(|segment| segment == "..") {
            return Err(InvalidPackPath::MustStayInsidePack { path: path() });
        }
        if text.split('/').any(|segment| segment == ".") {
            return Err(InvalidPackPath::DotSegment { path: path() });
        }
        if text.ends_with('/') {
            return Err(InvalidPackPath::TrailingSlash { path: path() });
        }
        if text.split('/').any(str::is_empty) {
            return Err(InvalidPackPath::EmptySegment { path: path() });
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
}
