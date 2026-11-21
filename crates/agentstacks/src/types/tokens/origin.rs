//! How the position of a scanned text is reported.
//!
//! The same scanner reads Markdown bodies, `render` files, frontmatter values and hook fields. Each
//! caller says which kind of text it hands over, so a token or a refusal is located the way the
//! author would look for it.
//!
//! # Responsibilities
//!
//! - [`TextOrigin::Lines`] locates by file and line, for text that is a run of lines in a file.
//! - [`TextOrigin::Field`] locates by file and key path, for text that is one value of a field.
//! - [`TextOrigin::locate`] builds the [`Location`] for a 0-based line of the scanned text.
//!
//! # Non-responsibilities
//!
//! Finding tokens belongs to [`scan`](crate::types::tokens::scan::scan).

use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::types::report::Location;

/// Where a scanned text came from.
///
/// # Examples
///
/// ```
/// use agentstacks::types::TextOrigin;
///
/// let field = TextOrigin::Field { file: "pack.toml".into(), key_path: "hooks[0].text".to_owned() };
/// assert_eq!(field.locate(5).to_string(), "pack.toml: hooks[0].text");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOrigin {
    /// Text that is a run of lines of a file.
    Lines {
        /// The file the text is in.
        file: PathBuf,
        /// The 1-based line of the file that the first line of the text is on.
        first_line: NonZeroUsize,
    },
    /// Text that is the value of one field.
    Field {
        /// The file the field is in.
        file: PathBuf,
        /// The dotted key path of the field, such as `hooks[0].text`.
        key_path: String,
    },
}

impl TextOrigin {
    /// The location of the 0-based `line_in_text` of the scanned text.
    ///
    /// A `Lines` origin gives the file line `first_line + line_in_text`. A `Field` origin gives
    /// the key path whatever the line.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::TextOrigin;
    ///
    /// let first_line = NonZeroUsize::new(10).ok_or("line numbers start at 1")?;
    /// let lines = TextOrigin::Lines { file: "SKILL.md".into(), first_line };
    /// assert_eq!(lines.locate(2).to_string(), "SKILL.md:12");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn locate(&self, line_in_text: usize) -> Location {
        match self {
            Self::Lines { file, first_line } => {
                Location::at_line(file, first_line.saturating_add(line_in_text))
            }
            Self::Field { file, key_path } => Location::at_key(file, key_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap_or(NonZeroUsize::MIN)
    }

    #[test]
    fn lines_origin_offsets_the_line() {
        let origin = TextOrigin::Lines {
            file: PathBuf::from("skills/x/SKILL.md"),
            first_line: line(6),
        };
        assert_eq!(origin.locate(2).to_string(), "skills/x/SKILL.md:8");
    }

    #[test]
    fn field_origin_locates_by_key_path() {
        let origin = TextOrigin::Field {
            file: PathBuf::from("pack.toml"),
            key_path: "hooks[0].text".to_owned(),
        };
        assert_eq!(origin.locate(0).to_string(), "pack.toml: hooks[0].text");
        assert_eq!(origin.locate(3).to_string(), "pack.toml: hooks[0].text");
        assert_eq!(origin.locate(3).line(), None);
    }
}
