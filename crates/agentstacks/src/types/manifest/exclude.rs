//! A glob a pack uses to leave files out of what it installs.
//!
//! It is a type of its own because the relative-path rule and the compiled matcher belong to the
//! glob, so `PackManifest` holds a checked glob instead of a string it must remember to check.
//!
//! # Responsibilities
//!
//! - [`ExcludeGlob`] keeps the pattern as written, for messages, and the compiled matcher, for
//!   matching.
//! - `*` and `?` stay inside one path segment and never match `/`; only `**` crosses segments.
//! - It refuses a pattern that is not relative (a leading `/`) and one the glob parser refuses.
//!   The relative check is the crate's own, because the glob parser accepts `/tmp/**`.
//!
//! # Non-responsibilities
//!
//! Which files a glob may exclude, and what it excludes, belongs to the checks over a loaded
//! pack.

use std::fmt;
use std::str::FromStr;

use globset::{GlobBuilder, GlobMatcher};

use crate::types::manifest::InvalidExcludeGlob;
use crate::types::report::MustBeRelative;
use crate::types::tree::TreePath;

/// A relative glob with its compiled matcher.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ExcludeGlob;
///
/// let tests = ExcludeGlob::new("**/*_test.lua")?;
/// assert_eq!(tests.pattern(), "**/*_test.lua");
///
/// let refused = ExcludeGlob::new("/tmp/**").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"/tmp/**\" must be relative");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct ExcludeGlob {
    pattern: String,
    matcher: GlobMatcher,
}

impl ExcludeGlob {
    /// Compiles `text` as an exclude glob in which `*` does not cross `/`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidExcludeGlob::MustBeRelative`] for a leading `/`, and
    /// [`InvalidExcludeGlob::NotAGlob`] when the pattern does not compile.
    pub fn new(text: &str) -> Result<Self, InvalidExcludeGlob> {
        if text.starts_with('/') {
            return Err(MustBeRelative::new(text).into());
        }
        let glob = GlobBuilder::new(text)
            .literal_separator(true)
            .build()
            .map_err(|_| InvalidExcludeGlob::NotAGlob {
                text: text.to_owned(),
            })?;
        Ok(Self {
            pattern: text.to_owned(),
            matcher: glob.compile_matcher(),
        })
    }

    /// The pattern as written.
    #[must_use]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Whether the glob matches the walked file `path`, relative to the pack root.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::fs::StdFileTree;
    /// use agentstacks::types::{ExcludeGlob, walk};
    ///
    /// let pack = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/packs/valid-render-exclude");
    /// let (walked, _) = walk(&StdFileTree::new(pack));
    /// let tests = ExcludeGlob::new("**/*_test.lua")?;
    /// let excluded: Vec<&str> = walked
    ///     .files()
    ///     .iter()
    ///     .filter(|path| tests.matches(path))
    ///     .map(|path| path.as_str())
    ///     .collect();
    /// assert_eq!(excluded, ["scripts/a_test.lua"]);
    /// # Ok::<(), agentstacks::types::InvalidExcludeGlob>(())
    /// ```
    #[must_use]
    pub fn matches(&self, path: &TreePath) -> bool {
        self.matcher.is_match(path.as_str())
    }
}

impl FromStr for ExcludeGlob {
    type Err = InvalidExcludeGlob;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for ExcludeGlob {
    type Error = InvalidExcludeGlob;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

impl PartialEq for ExcludeGlob {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

impl Eq for ExcludeGlob {}

impl fmt::Display for ExcludeGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manifest::ManifestError;
    use crate::types::report::{Diagnostic, Location};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Whether `glob` matches the pack-relative file written `path`.
    fn matches(glob: &ExcludeGlob, path: &str) -> bool {
        glob.matches(&TreePath::from_parts(&[path]))
    }

    #[test]
    fn exclude_glob_compiles_a_typical_pattern() -> TestResult {
        let test_files = ExcludeGlob::new("**/*_test.lua")?;
        assert_eq!(test_files.pattern(), "**/*_test.lua");
        assert_eq!(test_files.to_string(), "**/*_test.lua");
        assert!(matches(&test_files, "hooks/enforce_test.lua"));
        let markdown = ExcludeGlob::new("**/*.md")?;
        assert!(matches(&markdown, "skills/discuss/SKILL.md"));
        assert!(!matches(&markdown, "hooks/enforce.lua"));
        Ok(())
    }

    #[test]
    fn exclude_glob_star_stays_in_one_segment_and_double_star_crosses() -> TestResult {
        let single = ExcludeGlob::new("*.md")?;
        assert!(matches(&single, "README.md"));
        assert!(!matches(&single, "skills/a/SKILL.md"));
        let double = ExcludeGlob::new("**/*.md")?;
        assert!(matches(&double, "skills/a/SKILL.md"));
        assert!(matches(&double, "README.md"));
        assert!(matches(&ExcludeGlob::new("hooks/**")?, "hooks/a/b.lua"));
        assert!(!matches(
            &ExcludeGlob::new("*_test.lua")?,
            "hooks/x_test.lua"
        ));
        Ok(())
    }

    #[test]
    fn exclude_glob_refuses_invalid_and_absolute_globs() -> TestResult {
        let cases = [
            ("[", "\"[\" is not a valid glob"),
            ("/tmp/**", "\"/tmp/**\" must be relative"),
        ];
        for (text, message) in cases {
            let error = ExcludeGlob::new(text).err().ok_or("expected a refusal")?;
            let location = Location::at_key("packs/core/pack.toml", "exclude[0]");
            assert_eq!(
                Diagnostic::new(location, ManifestError::from(error)).to_string(),
                format!("packs/core/pack.toml: exclude[0]: {message}")
            );
        }
        Ok(())
    }

    #[test]
    fn exclude_glob_from_str_and_try_from_match_new() -> TestResult {
        assert_eq!(
            "**/*.md".parse::<ExcludeGlob>()?,
            ExcludeGlob::new("**/*.md")?
        );
        assert_eq!(
            ExcludeGlob::try_from("hooks/**")?,
            ExcludeGlob::new("hooks/**")?
        );
        assert_eq!(
            "/tmp/**".parse::<ExcludeGlob>().err(),
            ExcludeGlob::new("/tmp/**").err()
        );
        assert_eq!(
            ExcludeGlob::try_from("[").err(),
            ExcludeGlob::new("[").err()
        );
        Ok(())
    }
}
