//! The refusals for a name that breaks its naming rule: a pack, tool or item name, an option
//! name, or an environment variable name.
//!
//! The three refusals share one shape (the refused value, quoted in a one-line message) but not
//! one wording, so each is its own type. They live in one file because they differ only in the
//! rule they name; the constructors that produce them live beside each name type.
//!
//! # Responsibilities
//!
//! - [`InvalidName`] holds the refused value and displays `"<value>" is not a valid name`.
//! - [`InvalidOptionName`] displays `"<value>" is not a valid option name`.
//! - [`InvalidEnvName`] displays `"<value>" is not an environment variable name`.
//!
//! # Non-responsibilities
//!
//! Callers whose message words a bad name differently map the error into their own message.
//!
//! Each type has unit tests for its message and accessor, and a doctest through the public
//! constructor that refuses.

/// A pack, tool or item name that is not valid.
///
/// # Examples
///
/// ```
/// use agentstacks::types::PackName;
///
/// let refused = PackName::new("Bad").err();
/// assert_eq!(refused.as_ref().map(ToString::to_string).as_deref(), Some("\"Bad\" is not a valid name"));
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("Bad"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" is not a valid name")]
pub struct InvalidName(String);

impl InvalidName {
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

/// An environment variable name that is not valid.
///
/// # Examples
///
/// ```
/// use agentstacks::types::EnvName;
///
/// let refused = EnvName::new("bad").err();
/// assert_eq!(refused.as_ref().map(ToString::to_string).as_deref(), Some("\"bad\" is not an environment variable name"));
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("bad"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" is not an environment variable name")]
pub struct InvalidEnvName(String);

impl InvalidEnvName {
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

/// An option name that is not valid.
///
/// # Examples
///
/// ```
/// use agentstacks::types::OptionName;
///
/// let refused = OptionName::new("Bad").err();
/// assert_eq!(refused.as_ref().map(ToString::to_string).as_deref(), Some("\"Bad\" is not a valid option name"));
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("Bad"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" is not a valid option name")]
pub struct InvalidOptionName(String);

impl InvalidOptionName {
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
    fn invalid_name_display_quotes_the_value() {
        assert_eq!(
            InvalidName::new("Core").to_string(),
            "\"Core\" is not a valid name"
        );
    }

    #[test]
    fn invalid_name_value_returns_the_refused_text() {
        assert_eq!(InvalidName::new("Core").value(), "Core");
    }

    #[test]
    fn invalid_env_name_display_quotes_the_value() {
        assert_eq!(
            InvalidEnvName::new("home").to_string(),
            "\"home\" is not an environment variable name"
        );
    }

    #[test]
    fn invalid_env_name_value_returns_the_refused_text() {
        assert_eq!(InvalidEnvName::new("home").value(), "home");
    }

    #[test]
    fn invalid_option_name_display_quotes_the_value() {
        assert_eq!(
            InvalidOptionName::new("a-b").to_string(),
            "\"a-b\" is not a valid option name"
        );
    }

    #[test]
    fn invalid_option_name_value_returns_the_refused_text() {
        assert_eq!(InvalidOptionName::new("a-b").value(), "a-b");
    }
}
