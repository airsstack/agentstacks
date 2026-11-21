//! The messages of `agentstacks.toml` with a wording of their own.
//!
//! The generic shapes belong to [`FieldError`](crate::types::FieldError) and the source messages to
//! [`SourceError`](crate::types::SourceError); every other refusal the project file can meet is one
//! variant here.
//!
//! It is a file of its own because these refusals belong to the project file alone, while the
//! source messages are shared with the lock; keeping them apart keeps each wording in one place.
//!
//! # Responsibilities
//!
//! - [`ProjectError`] has a variant per project-file refusal, each displaying exactly the message
//!   the author reads after the location.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries
//! it.

use crate::types::report::UnsupportedApi;

/// A refusal of `agentstacks.toml` as a whole or of an entry's name.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ProjectError;
///
/// assert_eq!(ProjectError::NothingToInstall.to_string(), "nothing to install");
/// assert_eq!(ProjectError::InvalidName.to_string(), "not a valid name");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectError {
    /// An `api` this agentstacks does not speak.
    #[error("api {0}")]
    UnsupportedApi(#[from] UnsupportedApi),
    /// A file that lists no tool or no pack.
    #[error("nothing to install")]
    NothingToInstall,
    /// A `[tools.*]` or `[packs.*]` key that is not a valid name.
    #[error("not a valid name")]
    InvalidName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_error_messages_are_the_refusal_texts() {
        assert_eq!(
            ProjectError::from(UnsupportedApi::new(1)).to_string(),
            "api 1 not supported (this agentstacks speaks 0)"
        );
        assert_eq!(
            ProjectError::NothingToInstall.to_string(),
            "nothing to install"
        );
        assert_eq!(ProjectError::InvalidName.to_string(), "not a valid name");
    }
}
