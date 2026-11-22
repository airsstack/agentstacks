//! A file's bytes as text, refused with the one wording every reader of a tree uses.
//!
//! # Responsibilities
//!
//! - `utf8_text`: a file's bytes as text, or an [`io::ErrorKind::InvalidData`] error displayed as
//!   `not valid UTF-8`, the same refusal `FileTree::read_text` gives.

use std::io;

/// The text of a file's `bytes`, refused the way `FileTree::read_text` refuses them.
///
/// # Errors
///
/// An [`io::ErrorKind::InvalidData`] error displayed as `not valid UTF-8` when `bytes` are not
/// UTF-8.
pub(crate) fn utf8_text(bytes: Vec<u8>) -> io::Result<String> {
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_text_words_bad_bytes_as_read_text_does() {
        assert_eq!(utf8_text(b"a\n".to_vec()).ok().as_deref(), Some("a\n"));
        let error = utf8_text(vec![0x66, 0xFF]).err();
        assert_eq!(
            error.as_ref().map(io::Error::kind),
            Some(io::ErrorKind::InvalidData)
        );
        assert_eq!(
            error.map(|e| e.to_string()),
            Some("not valid UTF-8".to_owned())
        );
    }
}
