//! The replacement text for each token of a scanned text.
//!
//! It is a type of its own so the key (the token's span) and the text-order iteration are fixed
//! in one place, and no caller sorts or re-keys a map before rendering.
//!
//! # Responsibilities
//!
//! - [`TokenValues`] holds one replacement string per token, keyed by the token's span, so two
//!   tokens with the same kind and target can still take different values.
//! - Iteration is in text order, by span start.
//!
//! # Non-responsibilities
//!
//! Choosing a value belongs to the caller, and splicing values into text belongs to
//! [`render`](crate::types::tokens::render::render).

use std::collections::BTreeMap;
use std::ops::Range;

use crate::types::tokens::parsed::Token;

/// A replacement string for each of a set of tokens.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{TextOrigin, TokenValues, scan};
///
/// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
/// let (tokens, _) = scan("{{var home}} {{var home}}", &origin);
/// let mut values = TokenValues::new();
/// for (located, value) in tokens.iter().zip(["/one", "/two"]) {
///     values.insert(located.token(), value.to_owned());
/// }
/// let in_order: Vec<&str> = values.iter().map(|(_, value)| value).collect();
/// assert_eq!(in_order, ["/one", "/two"]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenValues {
    by_span: BTreeMap<(usize, usize), String>,
}

impl TokenValues {
    /// An empty set of values.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::TokenValues;
    ///
    /// assert_eq!(TokenValues::new().iter().count(), 0);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            by_span: BTreeMap::new(),
        }
    }

    /// Sets the value for `token`, replacing an earlier value for the same token.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, TokenValues, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{var home}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// let mut values = TokenValues::new();
    /// values.insert(located.token(), "first".to_owned());
    /// values.insert(located.token(), "second".to_owned());
    /// assert_eq!(values.iter().count(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn insert(&mut self, token: &Token, value: String) {
        let span = token.span();
        self.by_span.insert((span.start, span.end), value);
    }

    /// The values in text order, each with the span of the token it replaces.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    ///
    /// use agentstacks::types::{TextOrigin, TokenValues, scan};
    ///
    /// let origin = TextOrigin::Lines { file: "a.md".into(), first_line: NonZeroUsize::MIN };
    /// let (tokens, _) = scan("{{var home}}", &origin);
    /// let located = tokens.first().ok_or("no token found")?;
    /// let mut values = TokenValues::new();
    /// values.insert(located.token(), "/home/me".to_owned());
    /// assert_eq!(values.iter().next(), Some((0..located.token().span().end, "/home/me")));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (Range<usize>, &str)> {
        self.by_span
            .iter()
            .map(|(&(start, end), value)| (start..end, value.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use super::*;
    use crate::types::{TextOrigin, scan};

    fn origin() -> TextOrigin {
        TextOrigin::Lines {
            file: PathBuf::from("notes.md"),
            first_line: NonZeroUsize::MIN,
        }
    }

    #[test]
    fn token_values_iterate_in_text_order() {
        let (tokens, _) = scan("{{root}} and {{skill a}}", &origin());
        let [first, second] = tokens.as_slice() else {
            assert_eq!(tokens.len(), 2);
            return;
        };

        let mut values = TokenValues::new();
        values.insert(second.token(), "second".to_owned());
        values.insert(first.token(), "first".to_owned());
        let collected: Vec<_> = values.iter().collect();
        assert_eq!(collected, [(0..8, "first"), (13..24, "second")]);

        values.insert(first.token(), "replaced".to_owned());
        let collected: Vec<_> = values.iter().collect();
        assert_eq!(collected, [(0..8, "replaced"), (13..24, "second")]);
    }
}
