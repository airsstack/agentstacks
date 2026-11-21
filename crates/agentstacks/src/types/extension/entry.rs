//! The `entry` path of a tool extension.
//!
//! It is a type of its own, not a `String` in the manifest, so the rule that an entry stays inside
//! the extension is enforced once at construction and no later code can hold a path outside it.
//!
//! # Responsibilities
//!
//! - [`EntryPath`] is a relative path to a `.lua` file that stays inside the extension, so a held
//!   value never points outside the directory it was read from.
//! - An absolute path and a path with a `..` segment get the same refusal, checked before the
//!   file-name rule.
//! - [`EntryPath::normalized`] is the path without its leading `./` segments, the form that
//!   compares equal to a walked file's path.
//!
//! # Non-responsibilities
//!
//! It touches no file: whether the entry exists in the extension directory is decided by the
//! extension loader.

use std::fmt;

use crate::types::extension::error::ExtensionError;

/// A path inside an extension that ends in `.lua`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::EntryPath;
///
/// let entry = EntryPath::new("./adapter.lua")?;
/// assert_eq!(entry.as_str(), "./adapter.lua");
/// assert_eq!(entry.normalized(), "adapter.lua");
/// assert_eq!(entry.to_string(), "./adapter.lua");
/// # Ok::<(), agentstacks::types::ExtensionError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPath(String);

impl EntryPath {
    /// Parses `value` as an entry path.
    ///
    /// # Errors
    ///
    /// Returns [`ExtensionError::EntryOutside`] when `value` starts with `/` or has a `..`
    /// segment, and otherwise [`ExtensionError::EntryNotLua`] when it does not end in `.lua`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{EntryPath, ExtensionError};
    ///
    /// assert!(EntryPath::new("lib/adapter.lua").is_ok());
    ///
    /// let outside = EntryPath::new("../adapter.lua").err().ok_or("expected a refusal")?;
    /// assert!(matches!(outside, ExtensionError::EntryOutside { .. }));
    /// assert_eq!(outside.to_string(), "\"../adapter.lua\" must stay inside the extension");
    ///
    /// let not_lua = EntryPath::new("adapter.txt").err().ok_or("expected a refusal")?;
    /// assert_eq!(not_lua.to_string(), "\"adapter.txt\" is not a .lua file");
    /// # Ok::<(), &str>(())
    /// ```
    pub fn new(value: &str) -> Result<Self, ExtensionError> {
        if value.starts_with('/') || value.split('/').any(|segment| segment == "..") {
            return Err(ExtensionError::EntryOutside {
                value: value.to_owned(),
            });
        }
        if !value.as_bytes().ends_with(b".lua") {
            return Err(ExtensionError::EntryNotLua {
                value: value.to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }

    /// The path as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path with every leading `./` segment (and any slashes after it) removed, so
    /// `./lib/a.lua` reads `lib/a.lua`. Nothing else is rewritten: the removal is lexical and
    /// never looks at the disk.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::EntryPath;
    ///
    /// assert_eq!(EntryPath::new("./lib/a.lua")?.normalized(), "lib/a.lua");
    /// assert_eq!(EntryPath::new("././a.lua")?.normalized(), "a.lua");
    /// # Ok::<(), agentstacks::types::ExtensionError>(())
    /// ```
    #[must_use]
    pub fn normalized(&self) -> &str {
        let mut rest = self.0.as_str();
        while let Some(tail) = rest.strip_prefix("./") {
            rest = tail.trim_start_matches('/');
        }
        rest
    }
}

impl fmt::Display for EntryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn refusal(value: &str) -> Result<String, Box<dyn std::error::Error>> {
        match EntryPath::new(value) {
            Ok(path) => Err(format!("{path} was accepted").into()),
            Err(error) => Ok(error.to_string()),
        }
    }

    #[test]
    fn entry_path_accepts_relative_lua_paths() -> TestResult {
        for value in ["adapter.lua", "lib/adapter.lua"] {
            let path = EntryPath::new(value)?;
            assert_eq!(path.as_str(), value);
            assert_eq!(path.to_string(), value);
        }
        Ok(())
    }

    #[test]
    fn entry_path_refuses_paths_leaving_the_extension() -> TestResult {
        for value in ["../a.lua", "lib/../../a.lua", "/a.lua"] {
            assert_eq!(
                refusal(value)?,
                format!("\"{value}\" must stay inside the extension")
            );
        }
        Ok(())
    }

    #[test]
    fn entry_path_refuses_non_lua_files() -> TestResult {
        assert_eq!(refusal("adapter.js")?, "\"adapter.js\" is not a .lua file");
        Ok(())
    }

    #[test]
    fn entry_path_checks_containment_before_extension() -> TestResult {
        for value in ["../a.js", "/a.js"] {
            assert_eq!(
                refusal(value)?,
                format!("\"{value}\" must stay inside the extension")
            );
        }
        Ok(())
    }

    #[test]
    fn entry_path_normalized_drops_leading_current_directory_segments() -> TestResult {
        for (written, normalized) in [
            ("adapter.lua", "adapter.lua"),
            ("./adapter.lua", "adapter.lua"),
            ("././lib/adapter.lua", "lib/adapter.lua"),
            (".//adapter.lua", "adapter.lua"),
            ("lib/./adapter.lua", "lib/./adapter.lua"),
            (".hidden/adapter.lua", ".hidden/adapter.lua"),
        ] {
            let path = EntryPath::new(written)?;
            assert_eq!(path.normalized(), normalized, "{written}");
            assert_eq!(path.as_str(), written);
        }
        Ok(())
    }

    #[test]
    fn entry_path_allows_dots_inside_a_segment() -> TestResult {
        for value in ["a..b.lua", "lib/.x.lua"] {
            assert_eq!(EntryPath::new(value)?.as_str(), value);
        }
        Ok(())
    }
}
