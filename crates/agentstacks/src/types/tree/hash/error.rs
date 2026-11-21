//! The refusal for a string that is not a content hash.
//!
//! It is a type of its own because it carries no location: the caller that parsed the text knows
//! which key held it and places the message there, which one shared error type could not do for
//! every caller.
//!
//! # Responsibilities
//!
//! - [`ContentHashError`] displays the bad-hash message without a location; the caller places it
//!   under the key that held the hash.
//!
//! # Non-responsibilities
//!
//! Deciding whether a string is a hash belongs to [`ContentHash`](crate::types::ContentHash).

/// A string that is not `sha256:` followed by 64 lowercase hex characters.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ContentHash;
///
/// let refusal = ContentHash::new("sha256:ABC").err();
/// assert_eq!(
///     refusal.map(|e| e.to_string()),
///     Some("bad hash (expected sha256:<64 lowercase hex>)".to_owned())
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("bad hash (expected sha256:<64 lowercase hex>)")]
pub struct ContentHashError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_error_displays_its_message() {
        assert_eq!(
            ContentHashError.to_string(),
            "bad hash (expected sha256:<64 lowercase hex>)"
        );
    }
}
