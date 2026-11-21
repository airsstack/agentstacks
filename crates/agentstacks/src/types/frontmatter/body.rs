//! Where a Markdown file's body begins.
//!
//! The body scan starts right after the closing `---` of the frontmatter, and a token's line in the
//! body is its line in the body text plus the body's first line in the file. This type carries both
//! numbers so the body is scanned once and its tokens are still located by file line.
//!
//! # Responsibilities
//!
//! - [`BodyStart`] holds the byte offset into the file text where the body begins and the 1-based
//!   file line of its first line.
//! - [`BodyStart::body`] slices the body out of the file text.
//!
//! # Non-responsibilities
//!
//! Finding the closing `---` belongs to [`Frontmatter`](crate::types::Frontmatter).

use std::num::NonZeroUsize;

/// The offset and file line where a Markdown body begins.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::Frontmatter;
///
/// let text = "---\nk: v\n---\nfirst\nsecond\n";
/// let start = Frontmatter::split(Path::new("a.md"), text)
///     .map_err(|refusal| refusal.to_string())?
///     .body_start();
/// assert_eq!(start.line().get(), 4);
/// assert_eq!(start.body(text), "first\nsecond\n");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyStart {
    offset: usize,
    line: NonZeroUsize,
}

impl BodyStart {
    /// A body that begins at byte `offset` of the file text, on 1-based file line `line`.
    pub(crate) const fn new(offset: usize, line: NonZeroUsize) -> Self {
        Self { offset, line }
    }

    /// The 1-based file line the body's first line is on.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::Frontmatter;
    ///
    /// let start = Frontmatter::split(Path::new("a.md"), "---\n---\nbody")
    ///     .map_err(|refusal| refusal.to_string())?
    ///     .body_start();
    /// assert_eq!(start.line().get(), 3);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn line(&self) -> NonZeroUsize {
        self.line
    }

    /// The body: the part of `text` from the offset on. Returns an empty string when `text` is
    /// shorter than the offset or the offset is not on a character boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::Frontmatter;
    ///
    /// let text = "---\nk: v\n---\nbody\n";
    /// let start = Frontmatter::split(Path::new("a.md"), text)
    ///     .map_err(|refusal| refusal.to_string())?
    ///     .body_start();
    /// assert_eq!(start.body(text), "body\n");
    /// assert_eq!(start.body(""), "");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn body<'a>(&self, text: &'a str) -> &'a str {
        text.get(self.offset..).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn body_start_slices_body_and_reports_line() -> TestResult {
        let text = "---\nname: x\n---\n# Title\n";
        let start = BodyStart::new(16, NonZeroUsize::new(4).ok_or("line is non-zero")?);
        assert_eq!(start.body(text), "# Title\n");
        assert_eq!(start.line().get(), 4);
        Ok(())
    }
}
