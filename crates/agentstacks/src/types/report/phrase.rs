//! The refusal phrases that more than one format words the same way.
//!
//! Each phrase is defined once here, as a small error type, and every module error enum that
//! raises it wraps that type in a variant of the same name. The wording therefore cannot drift
//! between formats, and a caller matches the same variant name whichever file refused the value.
//!
//! The phrases are types rather than one shared enum so that each module enum wraps only the
//! phrases it can raise, and its variants stay an exact list of what it refuses.
//!
//! # Responsibilities
//!
//! - [`ListedTwice`]: `<entry> listed twice`, generic over how the repeated entry is shown. A list
//!   entry shows as [`Quoted`] text; the lock shows a typed entry with its kind in front.
//! - [`MustBeRelative`]: `"<text>" must be relative`.
//! - [`OutsideProject`]: `"<text>" must stay inside the project`. A refusal that names no value
//!   uses the bare phrase, `OUTSIDE_PROJECT`.
//! - [`NotLuaFile`]: `"<text>" is not a .lua file`.
//! - [`DoesNotExist`]: `"<text>" does not exist`.
//! - [`UnsupportedApi`]: `<found> not supported (this agentstacks speaks <API>)`. A format whose
//!   message starts with `api` puts the word in front in its own variant.
//!
//! # Non-responsibilities
//!
//! A phrase only one format uses stays in that format's error enum, and the generic field shapes
//! (`required`, `empty`, ...) are [`FieldError`](crate::types::FieldError).

use std::fmt::{self, Display};

use crate::types::api::API;

/// The bare phrase of a path that climbs out of the project, for a refusal that names no value.
pub(in crate::types) const OUTSIDE_PROJECT: &str = "must stay inside the project";

/// Refused text, displayed between double quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quoted {
    text: String,
}

impl Quoted {
    /// Quotes `text`.
    pub(in crate::types) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// The text without the quotes.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Display for Quoted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.text)
    }
}

/// An entry that repeats an earlier one: `<entry> listed twice`.
///
/// `E` decides how the entry is shown: [`Quoted`] text for a list entry, a typed key for the lock.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{entry} listed twice")]
pub struct ListedTwice<E> {
    entry: E,
}

impl<E> ListedTwice<E> {
    /// The refusal of the repeated `entry`.
    pub(in crate::types) const fn new(entry: E) -> Self {
        Self { entry }
    }

    /// The repeated entry.
    #[must_use]
    pub const fn entry(&self) -> &E {
        &self.entry
    }
}

impl ListedTwice<Quoted> {
    /// The refusal of a repeated list entry written as `text`.
    pub(in crate::types) fn text(text: impl Into<String>) -> Self {
        Self::new(Quoted::new(text))
    }
}

/// A path that is absolute where a relative one is required.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{text}\" must be relative")]
pub struct MustBeRelative {
    text: String,
}

/// A path that climbs out of the project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{text}\" {}", OUTSIDE_PROJECT)]
pub struct OutsideProject {
    text: String,
}

/// A path that must name a `.lua` file and does not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{text}\" is not a .lua file")]
pub struct NotLuaFile {
    text: String,
}

/// A path that names no file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{text}\" does not exist")]
pub struct DoesNotExist {
    text: String,
}

/// Generates the constructor and the getter of a phrase that carries the refused text.
macro_rules! refused_text {
    ($($phrase:ident),+) => {$(
        impl $phrase {
            /// The refusal of `text`, the value as written.
            pub(in crate::types) fn new(text: impl Into<String>) -> Self {
                Self { text: text.into() }
            }

            /// The refused text, as written.
            #[must_use]
            pub fn text(&self) -> &str {
                &self.text
            }
        }
    )+};
}

refused_text!(MustBeRelative, OutsideProject, NotLuaFile, DoesNotExist);

/// An `api` this agentstacks does not speak: `<found> not supported (this agentstacks speaks
/// <API>)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{found} not supported (this agentstacks speaks {})", API)]
pub struct UnsupportedApi {
    found: i64,
}

impl UnsupportedApi {
    /// The refusal of the api `found`.
    pub(in crate::types) const fn new(found: i64) -> Self {
        Self { found }
    }

    /// The api the file asks for.
    #[must_use]
    pub const fn found(&self) -> i64 {
        self.found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_displays_text_between_double_quotes() {
        assert_eq!(Quoted::new("core").to_string(), "\"core\"");
        assert_eq!(Quoted::new("core").text(), "core");
    }

    #[test]
    fn listed_twice_shows_the_entry_then_the_phrase() {
        let refusal = ListedTwice::text("core");
        assert_eq!(refusal.to_string(), "\"core\" listed twice");
        assert_eq!(refusal.entry().text(), "core");
    }

    #[test]
    fn listed_twice_shows_any_displayable_entry() {
        assert_eq!(
            ListedTwice::new("tool \"claude\"").to_string(),
            "tool \"claude\" listed twice"
        );
    }

    #[test]
    fn text_phrases_quote_the_refused_text() {
        assert_eq!(
            MustBeRelative::new("/x").to_string(),
            "\"/x\" must be relative"
        );
        assert_eq!(
            OutsideProject::new("../x").to_string(),
            "\"../x\" must stay inside the project"
        );
        assert_eq!(
            NotLuaFile::new("x.sh").to_string(),
            "\"x.sh\" is not a .lua file"
        );
        assert_eq!(
            DoesNotExist::new("a.lua").to_string(),
            "\"a.lua\" does not exist"
        );
    }

    #[test]
    fn text_phrases_keep_the_text_as_written() {
        assert_eq!(MustBeRelative::new("/x").text(), "/x");
        assert_eq!(OutsideProject::new("../x").text(), "../x");
        assert_eq!(NotLuaFile::new("x.sh").text(), "x.sh");
        assert_eq!(DoesNotExist::new("a.lua").text(), "a.lua");
    }

    #[test]
    fn outside_project_bare_phrase_is_the_suffix_of_the_quoted_one() {
        assert_eq!(OUTSIDE_PROJECT, "must stay inside the project");
        assert!(
            OutsideProject::new("x")
                .to_string()
                .ends_with(OUTSIDE_PROJECT)
        );
    }

    #[test]
    fn unsupported_api_names_the_found_and_the_spoken_api() {
        let refusal = UnsupportedApi::new(1);
        assert_eq!(
            refusal.to_string(),
            "1 not supported (this agentstacks speaks 0)"
        );
        assert_eq!(refusal.found(), 1);
    }
}
