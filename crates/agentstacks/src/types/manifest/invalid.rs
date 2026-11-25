//! The refusals the manifest value types return when their constructors reject a value.
//!
//! These errors sit together because each is only the refusal wording of one value type in this
//! group, and a reader checking what an author sees wants every message in one place. The value
//! types themselves stay in their own files with their parsing rules.
//!
//! # Responsibilities
//!
//! - [`InvalidRange`]: a `requires` text that is no range or uses an unsupported operator.
//! - [`InvalidExcludeGlob`]: an `exclude` pattern that is absolute or not a glob.
//! - [`InvalidOptionType`]: an option `type` outside `boolean`, `integer` and `string`.
//! - [`InvalidPackPath`]: a pack-relative path with a variant per path rule it breaks.
//! - [`InvalidVersion`]: text that is not a semantic version.
//!
//! # Non-responsibilities
//!
//! The parsing rules that raise these errors belong to the value types; a reader whose message
//! words a refusal differently maps the error into its own message. The tests assert the exact
//! `Display` text of every variant.

use crate::types::report::MustBeRelative;

/// A text that cannot be a [`VersionRange`](crate::types::VersionRange).
///
/// # Examples
///
/// ```
/// use agentstacks::types::VersionRange;
///
/// let refused = VersionRange::new("nope").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"nope\" is not a version range");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidRange {
    /// A value that is not a version range.
    #[error("\"{text}\" is not a version range")]
    NotVersionRange {
        /// The refused text.
        text: String,
    },
    /// A version range whose operator this version of agentstacks does not support.
    #[error("\"{text}\" uses an operator this agentstacks does not support")]
    UnsupportedOperator {
        /// The range as written.
        text: String,
    },
}

/// A pattern that cannot be an [`ExcludeGlob`](crate::types::ExcludeGlob).
///
/// # Examples
///
/// ```
/// use agentstacks::types::ExcludeGlob;
///
/// let refused = ExcludeGlob::new("[").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"[\" is not a valid glob");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidExcludeGlob {
    /// A pattern that starts with `/`.
    #[error(transparent)]
    MustBeRelative(#[from] MustBeRelative),
    /// A pattern the glob parser refuses.
    #[error("\"{text}\" is not a valid glob")]
    NotAGlob {
        /// The refused pattern, as written.
        text: String,
    },
}

/// An option `type` that is none of `boolean`, `integer` and `string`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::OptionType;
///
/// let refused = OptionType::new("number").err();
/// assert_eq!(
///     refused.as_ref().map(ToString::to_string).as_deref(),
///     Some("\"number\" (expected boolean, integer, string)")
/// );
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("number"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" (expected boolean, integer, string)")]
pub struct InvalidOptionType(String);

impl InvalidOptionType {
    /// An error for the refused `value`.
    pub(crate) fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    /// The refused value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

/// A path that cannot be a [`PackPath`](crate::types::PackPath).
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackPath;
///
/// let refused = PackPath::new("a//b").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"a//b\" must not have an empty segment");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPackPath {
    /// An empty path.
    #[error("\"\" must not be empty")]
    Empty,
    /// The path `.`, which names the pack's root rather than a file in it.
    #[error("\".\" must name a file inside the pack")]
    IsDot,
    /// A path that starts with `/`.
    #[error(transparent)]
    MustBeRelative(#[from] MustBeRelative),
    /// A path written with `\` instead of `/`.
    #[error("\"{text}\" must use / as the separator")]
    Backslash {
        /// The refused path, as written.
        text: String,
    },
    /// A path with a `..` segment.
    #[error("\"{text}\" must stay inside the pack")]
    MustStayInsidePack {
        /// The refused path, as written.
        text: String,
    },
    /// A path with a `.` segment, such as `./x` or `a/./b`.
    #[error("\"{text}\" must not have a . segment")]
    DotSegment {
        /// The refused path, as written.
        text: String,
    },
    /// A path that ends with `/`.
    #[error("\"{text}\" must not end with /")]
    TrailingSlash {
        /// The refused path, as written.
        text: String,
    },
    /// A path with an empty segment, such as `a//b`.
    #[error("\"{text}\" must not have an empty segment")]
    EmptySegment {
        /// The refused path, as written.
        text: String,
    },
}

/// A version that is not a semantic version.
///
/// A pack's `pack.version`, an extension's `version` and a locked entry's version are refused with
/// this error, through [`ManifestError::Version`](crate::types::ManifestError::Version),
/// [`ExtensionError::Version`](crate::types::ExtensionError::Version) and
/// [`LockError::Version`](crate::types::LockError::Version). The lock's `[airsl] version` words its
/// refusal itself, as [`LockError::NotAVersion`](crate::types::LockError::NotAVersion).
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackVersion;
///
/// let refused = PackVersion::new("1.2").err();
/// assert_eq!(
///     refused.as_ref().map(ToString::to_string).as_deref(),
///     Some("\"1.2\" is not semver")
/// );
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("1.2"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" is not semver")]
pub struct InvalidVersion(String);

impl InvalidVersion {
    /// An error for the refused `value`.
    pub(crate) fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    /// The refused value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_range_displays_each_message() {
        let cases = [
            (
                InvalidRange::NotVersionRange {
                    text: "nope".to_owned(),
                },
                "\"nope\" is not a version range",
            ),
            (
                InvalidRange::UnsupportedOperator {
                    text: ">=0.1".to_owned(),
                },
                "\">=0.1\" uses an operator this agentstacks does not support",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn invalid_exclude_glob_displays_each_message() {
        let cases = [
            (
                InvalidExcludeGlob::from(MustBeRelative::new("/x")),
                "\"/x\" must be relative",
            ),
            (
                InvalidExcludeGlob::NotAGlob {
                    text: "[".to_owned(),
                },
                "\"[\" is not a valid glob",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn invalid_option_type_names_the_allowed_set() {
        let error = InvalidOptionType::new("number");
        assert_eq!(
            error.to_string(),
            "\"number\" (expected boolean, integer, string)"
        );
        assert_eq!(error.value(), "number");
    }

    #[test]
    fn invalid_pack_path_displays_each_message() {
        let text = || "p".to_owned();
        let cases = [
            (InvalidPackPath::Empty, "\"\" must not be empty"),
            (
                InvalidPackPath::IsDot,
                "\".\" must name a file inside the pack",
            ),
            (
                InvalidPackPath::from(MustBeRelative::new(text())),
                "\"p\" must be relative",
            ),
            (
                InvalidPackPath::Backslash { text: text() },
                "\"p\" must use / as the separator",
            ),
            (
                InvalidPackPath::MustStayInsidePack { text: text() },
                "\"p\" must stay inside the pack",
            ),
            (
                InvalidPackPath::DotSegment { text: text() },
                "\"p\" must not have a . segment",
            ),
            (
                InvalidPackPath::TrailingSlash { text: text() },
                "\"p\" must not end with /",
            ),
            (
                InvalidPackPath::EmptySegment { text: text() },
                "\"p\" must not have an empty segment",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn invalid_version_display_quotes_the_value() {
        assert_eq!(
            InvalidVersion::new("1.2").to_string(),
            "\"1.2\" is not semver"
        );
    }

    #[test]
    fn value_returns_the_refused_text() {
        assert_eq!(InvalidVersion::new("1.2").value(), "1.2");
    }
}
