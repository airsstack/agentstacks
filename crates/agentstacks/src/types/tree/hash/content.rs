//! A SHA-256 digest in the form the lock pins.
//!
//! It is its own type so a digest is parsed once and every later use can trust its form, instead of
//! passing hash text around and re-checking it.
//!
//! # Responsibilities
//!
//! - [`ContentHash`] is `sha256:` followed by 64 lowercase hex characters. Parsing refuses
//!   anything else, so a value in hand is always well formed.
//! - [`ContentHash::of_bytes`] hashes raw bytes with no normalisation.
//!
//! # Non-responsibilities
//!
//! Which bytes get hashed belongs to the caller; the tree hash lives in
//! [`tree_hash()`](super::tree_hash()).

use std::fmt;
use std::str::FromStr;

use sha2::{Digest, Sha256};

use crate::types::tree::hash::error::ContentHashError;

/// The prefix every content hash starts with.
const PREFIX: &str = "sha256:";

/// The number of hex characters in a SHA-256 digest.
const HEX_LEN: usize = 64;

/// The lowercase hex digit for each nibble value.
const NIBBLES: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// A `sha256:<64 lowercase hex>` digest.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ContentHash;
///
/// // The digest of no bytes at all.
/// let empty = ContentHash::of_bytes(b"");
/// assert_eq!(
///     empty.to_string(),
///     "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
/// );
///
/// // Text parses back to the same value; anything else is refused.
/// assert_eq!(ContentHash::new(&empty.to_string())?, empty);
/// assert!(ContentHash::new("sha256:ABC").is_err());
/// # Ok::<(), agentstacks::types::ContentHashError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash {
    hex: String,
}

impl ContentHash {
    /// Parses `sha256:` followed by exactly 64 characters from `0-9a-f`.
    ///
    /// # Errors
    ///
    /// [`ContentHashError`] for any other text.
    pub fn new(text: &str) -> Result<Self, ContentHashError> {
        let hex = text.strip_prefix(PREFIX).ok_or(ContentHashError)?;
        let well_formed = hex.len() == HEX_LEN
            && hex
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        if well_formed {
            Ok(Self {
                hex: hex.to_owned(),
            })
        } else {
            Err(ContentHashError)
        }
    }

    /// The SHA-256 digest of `bytes`.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hex = String::with_capacity(HEX_LEN);
        for byte in Sha256::digest(bytes) {
            hex.push(NIBBLES[usize::from(byte >> 4)]);
            hex.push(NIBBLES[usize::from(byte & 0x0F)]);
        }
        Self { hex }
    }

    /// The 64 hex characters, without the `sha256:` prefix.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.hex
    }
}

impl FromStr for ContentHash {
    type Err = ContentHashError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl TryFrom<&str> for ContentHash {
    type Error = ContentHashError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}", self.hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_HEX: &str = "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7";
    const X_HEX: &str = "73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac";

    #[test]
    fn content_hash_of_bytes_matches_known_sha256_digests() {
        assert_eq!(
            ContentHash::of_bytes(b"a\n").to_string(),
            format!("sha256:{A_HEX}")
        );
        assert_eq!(
            ContentHash::of_bytes(b"x\n").to_string(),
            format!("sha256:{X_HEX}")
        );
    }

    #[test]
    fn content_hash_hex_omits_the_prefix() {
        assert_eq!(ContentHash::of_bytes(b"a\n").hex(), A_HEX);
    }

    #[test]
    fn content_hash_new_round_trips_display() {
        let hash = ContentHash::of_bytes(b"a\n");
        let text = hash.to_string();
        assert_eq!(ContentHash::new(&text), Ok(hash.clone()));
        assert_eq!(text.parse::<ContentHash>(), Ok(hash.clone()));
        assert_eq!(ContentHash::try_from(text.as_str()), Ok(hash));
    }

    #[test]
    fn content_hash_refuses_bad_forms() {
        let upper = format!("sha256:{}", A_HEX.to_uppercase());
        let short = format!("sha256:{}", &A_HEX[..63]);
        let long = format!("sha256:{A_HEX}0");
        let wrong_prefix = format!("md5:{A_HEX}");
        let with_g = format!("sha256:g{}", &A_HEX[1..]);
        for bad in [
            upper.as_str(),
            short.as_str(),
            long.as_str(),
            wrong_prefix.as_str(),
            A_HEX,
            "",
            with_g.as_str(),
        ] {
            let message = ContentHash::new(bad).err().map(|e| e.to_string());
            assert_eq!(
                message.as_deref(),
                Some("bad hash (expected sha256:<64 lowercase hex>)"),
                "input {bad:?}"
            );
        }
    }
}
