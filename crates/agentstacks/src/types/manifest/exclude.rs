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

use globset::{GlobBuilder, GlobMatcher};

use crate::types::manifest::InvalidExcludeGlob;

/// A relative glob with its compiled matcher.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ExcludeGlob;
///
/// let tests = ExcludeGlob::new("**/*_test.lua")?;
/// assert!(tests.matches("hooks/enforce_test.lua"));
/// assert!(!tests.matches("hooks/enforce.lua"));
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
            return Err(InvalidExcludeGlob::MustBeRelative {
                path: text.to_owned(),
            });
        }
        let glob = GlobBuilder::new(text)
            .literal_separator(true)
            .build()
            .map_err(|_| InvalidExcludeGlob::NotAGlob {
                pattern: text.to_owned(),
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

    /// Whether the glob matches `path`, a pack-relative `/`-separated path.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        self.matcher.is_match(path)
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
    use crate::types::{Diagnostic, Location, ManifestError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn exclude_glob_compiles_a_typical_pattern() -> TestResult {
        let test_files = ExcludeGlob::new("**/*_test.lua")?;
        assert_eq!(test_files.pattern(), "**/*_test.lua");
        assert_eq!(test_files.to_string(), "**/*_test.lua");
        assert!(test_files.matches("hooks/enforce_test.lua"));
        let markdown = ExcludeGlob::new("**/*.md")?;
        assert!(markdown.matches("skills/discuss/SKILL.md"));
        assert!(!markdown.matches("hooks/enforce.lua"));
        Ok(())
    }

    #[test]
    fn exclude_glob_star_stays_in_one_segment_and_double_star_crosses() -> TestResult {
        let single = ExcludeGlob::new("*.md")?;
        assert!(single.matches("README.md"));
        assert!(!single.matches("skills/a/SKILL.md"));
        let double = ExcludeGlob::new("**/*.md")?;
        assert!(double.matches("skills/a/SKILL.md"));
        assert!(double.matches("README.md"));
        assert!(ExcludeGlob::new("hooks/**")?.matches("hooks/a/b.lua"));
        assert!(!ExcludeGlob::new("*_test.lua")?.matches("hooks/x_test.lua"));
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
}
