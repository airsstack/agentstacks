//! The refusals the directory walker raises.
//!
//! A refusal is a message without its location: the diagnostic that carries it names the path.
//!
//! It is separate from the walker so a loader can match on the refusal and reword it, which it
//! could not do with a message already baked into a diagnostic.
//!
//! # Responsibilities
//!
//! - [`WalkError`] has one variant per refusal: a symlink, a file that is neither regular nor a
//!   directory, and a path that is not valid UTF-8.
//!
//! # Non-responsibilities
//!
//! An I/O failure is never a `WalkError`; it is
//! [`FormatError::Io`](crate::types::FormatError::Io). The extension wording of the symlink refusal
//! belongs to the extension loader, which rewords [`WalkError::Symlink`].

/// A directory entry the walker refuses.
///
/// # Examples
///
/// ```
/// use agentstacks::types::WalkError;
///
/// assert_eq!(
///     WalkError::Symlink.to_string(),
///     "symlinks are not allowed in a pack"
/// );
/// assert_eq!(
///     WalkError::SpecialFile.to_string(),
///     "only regular files and directories are allowed"
/// );
/// assert_eq!(WalkError::NotUtf8.to_string(), "path is not valid UTF-8");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WalkError {
    /// The entry is a symlink, to a file or to a directory.
    #[error("symlinks are not allowed in a pack")]
    Symlink,
    /// The entry is a FIFO, socket, device or other file that is neither regular nor a directory.
    #[error("only regular files and directories are allowed")]
    SpecialFile,
    /// A component of the entry's path is not valid UTF-8.
    #[error("path is not valid UTF-8")]
    NotUtf8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_error_symlink_displays_the_pack_wording() {
        assert_eq!(
            WalkError::Symlink.to_string(),
            "symlinks are not allowed in a pack"
        );
    }

    #[test]
    fn walk_error_special_file_displays_its_message() {
        assert_eq!(
            WalkError::SpecialFile.to_string(),
            "only regular files and directories are allowed"
        );
    }

    #[test]
    fn walk_error_not_utf8_displays_its_message() {
        assert_eq!(WalkError::NotUtf8.to_string(), "path is not valid UTF-8");
    }
}
