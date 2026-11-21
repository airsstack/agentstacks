//! The frontmatter refusals that are not one of the generic field shapes.
//!
//! These variants are specific to frontmatter, so they are kept apart from the generic
//! [`FieldError`](crate::types::FieldError) shapes that every format shares.
//!
//! # Responsibilities
//!
//! - [`FrontmatterError`] has a variant per message that skill and agent frontmatter words
//!   itself; each displays the exact message, to which the [`Location`](crate::types::Location) in front
//!   adds the file or key path.
//!
//! # Non-responsibilities
//!
//! The generic shapes (required, wrong type, empty) belong to [`FieldError`](crate::types::FieldError).
//! Deciding when a variant applies belongs to the checks over the frontmatter.

use crate::types::report::TypeWord;

/// A frontmatter refusal with its own wording.
///
/// # Examples
///
/// ```
/// use agentstacks::types::FrontmatterError;
///
/// assert_eq!(FrontmatterError::Missing.to_string(), "missing frontmatter");
/// assert_eq!(
///     FrontmatterError::UnknownKey { key: "colour".to_owned() }.to_string(),
///     "unknown key \"colour\"",
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrontmatterError {
    /// The file has no frontmatter block: it is missing, not first, or not closed.
    #[error("missing frontmatter")]
    Missing,
    /// The YAML does not parse.
    #[error("{message}{}", if *quoting_hint { " (a value starting with {{ must be quoted)" } else { "" })]
    Syntax {
        /// The parser's own message.
        message: String,
        /// Whether to add the hint that a value starting with `{{` must be quoted.
        quoting_hint: bool,
    },
    /// The block parsed to something other than a mapping.
    #[error("expected a mapping, got {got}")]
    NotAMapping {
        /// The type the block has.
        got: TypeWord,
    },
    /// `argument_hint` holds something other than a string.
    #[error("expected a string, got {got}")]
    NotAString {
        /// The type the value has.
        got: TypeWord,
    },
    /// `name` holds a token, so it is not a literal.
    #[error("name must be a literal")]
    NameNotLiteral,
    /// `name` differs from the directory or file stem the item lives at.
    #[error("name \"{found}\" does not match \"{expected}\"")]
    NameMismatch {
        /// The `name` the frontmatter holds.
        found: String,
        /// The name the directory or file stem gives.
        expected: String,
    },
    /// A key the frontmatter does not define.
    #[error("unknown key \"{key}\"")]
    UnknownKey {
        /// The refused key.
        key: String,
    },
    /// A value outside a fixed vocabulary.
    #[error("\"{value}\" (expected {allowed})")]
    NotOneOf {
        /// The refused value.
        value: String,
        /// The allowed values, joined with `", "`.
        allowed: String,
    },
    /// An entry of `tools` that is not an agent tool.
    #[error("unknown \"{value}\"")]
    UnknownTool {
        /// The refused tool.
        value: String,
    },
    /// An entry of `tools` that is listed more than once.
    #[error("\"{value}\" listed twice")]
    DuplicateTool {
        /// The repeated tool.
        value: String,
    },
    /// A key of `native` that is not a valid tool name.
    #[error("\"{value}\" is not a valid tool name")]
    InvalidToolName {
        /// The refused key.
        value: String,
    },
}

impl FrontmatterError {
    /// A [`FrontmatterError::NotOneOf`] for `value`, listing `allowed` joined with `", "`.
    pub(crate) fn not_one_of<'a>(value: &str, allowed: impl IntoIterator<Item = &'a str>) -> Self {
        Self::NotOneOf {
            value: value.to_owned(),
            allowed: allowed.into_iter().collect::<Vec<_>>().join(", "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_error_displays_each_message() {
        let cases = [
            (FrontmatterError::Missing, "missing frontmatter"),
            (FrontmatterError::NameNotLiteral, "name must be a literal"),
            (
                FrontmatterError::NotAMapping {
                    got: TypeWord::Null,
                },
                "expected a mapping, got null",
            ),
            (
                FrontmatterError::NotAString {
                    got: TypeWord::Boolean,
                },
                "expected a string, got boolean",
            ),
            (
                FrontmatterError::NameMismatch {
                    found: "x".to_owned(),
                    expected: "y".to_owned(),
                },
                "name \"x\" does not match \"y\"",
            ),
            (
                FrontmatterError::UnknownKey {
                    key: "disable-model-invocation".to_owned(),
                },
                "unknown key \"disable-model-invocation\"",
            ),
            (
                FrontmatterError::NotOneOf {
                    value: "manual".to_owned(),
                    allowed: "user, any".to_owned(),
                },
                "\"manual\" (expected user, any)",
            ),
            (
                FrontmatterError::UnknownTool {
                    value: "bash".to_owned(),
                },
                "unknown \"bash\"",
            ),
            (
                FrontmatterError::DuplicateTool {
                    value: "read".to_owned(),
                },
                "\"read\" listed twice",
            ),
            (
                FrontmatterError::InvalidToolName {
                    value: "Claude".to_owned(),
                },
                "\"Claude\" is not a valid tool name",
            ),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
        }
    }

    #[test]
    fn not_one_of_joins_the_allowed_values() {
        let error = FrontmatterError::not_one_of("manual", ["user", "any"]);
        assert_eq!(error.to_string(), "\"manual\" (expected user, any)");
    }

    #[test]
    fn syntax_error_appends_quoting_hint_only_when_set() {
        let message = "expected string scalar".to_owned();
        let hinted = FrontmatterError::Syntax {
            message: message.clone(),
            quoting_hint: true,
        };
        let plain = FrontmatterError::Syntax {
            message,
            quoting_hint: false,
        };
        assert_eq!(
            hinted.to_string(),
            "expected string scalar (a value starting with {{ must be quoted)"
        );
        assert_eq!(plain.to_string(), "expected string scalar");
    }
}
