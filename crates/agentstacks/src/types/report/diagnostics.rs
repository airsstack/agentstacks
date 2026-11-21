//! The collection of diagnostics a validation run returns.
//!
//! A run reports every refusal it finds, not just the first, and reports them in a stable order so
//! the same input always reads the same way. The collection is its own type so that order is
//! decided in one place.
//!
//! # Responsibilities
//!
//! - [`Diagnostics`] keeps every pushed [`Diagnostic`].
//! - On output it sorts by file path as a `/`-separated string compared in byte order. Within one
//!   file, diagnostics with a key path or no line come first in the order they were found, then
//!   diagnostics with a line by ascending line number. The sort is stable, so anything that rule
//!   does not order keeps its insertion order.
//!
//! # Non-responsibilities
//!
//! Producing the diagnostics belongs to the parsers and checks that push them.

use crate::types::report::diagnostic::Diagnostic;

/// Every diagnostic a run found, sorted on output.
///
/// # Examples
///
/// Diagnostics come back sorted by file, and within a file by line, whatever order they were found
/// in:
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use agentstacks::types::{Diagnostic, Diagnostics, FieldError, Location};
///
/// let line = |n| NonZeroUsize::new(n).ok_or("line numbers start at 1");
/// let mut found = Diagnostics::new();
/// found.push(Diagnostic::new(Location::at_line("b.md", line(2)?), FieldError::Empty));
/// found.push(Diagnostic::new(Location::at_line("a.md", line(9)?), FieldError::Required));
/// found.push(Diagnostic::new(Location::at_line("a.md", line(4)?), FieldError::UnknownKey));
///
/// let lines: Vec<String> = found.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(lines, ["a.md:4: unknown key", "a.md:9: required", "b.md:2: empty"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// An empty collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::Diagnostics;
    ///
    /// assert!(Diagnostics::new().is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds a diagnostic.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{Diagnostic, Diagnostics, FieldError, Location};
    ///
    /// let mut found = Diagnostics::new();
    /// found.push(Diagnostic::new(Location::file("pack.toml"), FieldError::Empty));
    /// assert_eq!(found.len(), 1);
    /// ```
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    /// How many diagnostics the collection holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the collection holds no diagnostic.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Every diagnostic, in output order.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{Diagnostic, Diagnostics, FieldError, Location};
    ///
    /// let mut found = Diagnostics::new();
    /// found.push(Diagnostic::new(Location::file("b.toml"), FieldError::Empty));
    /// found.push(Diagnostic::new(Location::file("a.toml"), FieldError::Required));
    /// let sorted = found.into_sorted();
    /// assert_eq!(sorted[0].to_string(), "a.toml: required");
    /// ```
    #[must_use]
    pub fn into_sorted(mut self) -> Vec<Diagnostic> {
        // `None` orders before `Some`, so unlined diagnostics precede lined ones; the sort is
        // stable, so ties keep their insertion order.
        self.items
            .sort_by_cached_key(|d| (d.location().file_text().into_bytes(), d.location().line()));
        self.items
    }

    /// `Ok(value)` when the collection is empty, otherwise `Err` holding the collection itself,
    /// which hands its diagnostics back in output order.
    ///
    /// # Errors
    ///
    /// Returns the collection when it is not empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{Diagnostic, Diagnostics, FieldError, Location};
    ///
    /// assert_eq!(Diagnostics::new().into_result(7).ok(), Some(7));
    ///
    /// let mut found = Diagnostics::new();
    /// found.push(Diagnostic::new(Location::file("pack.toml"), FieldError::Required));
    /// assert_eq!(found.into_result(7).err().map(|errors| errors.len()), Some(1));
    /// ```
    pub fn into_result<T>(self, value: T) -> Result<T, Self> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(self)
        }
    }
}

/// A single refusal as a collection of one.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Diagnostic, Diagnostics, FieldError, Location};
///
/// let refusal = Diagnostic::new(Location::file("SKILL.md"), FieldError::Required);
/// let found = Diagnostics::from(refusal);
/// assert_eq!(found.len(), 1);
/// ```
impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            items: vec![diagnostic],
        }
    }
}

impl Extend<Diagnostic> for Diagnostics {
    fn extend<I: IntoIterator<Item = Diagnostic>>(&mut self, iter: I) {
        self.items.extend(iter);
    }
}

/// Iterating consumes the collection and yields the diagnostics in sorted output order, the same
/// order [`Diagnostics::into_sorted`] returns.
impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_sorted().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::Path;

    use super::*;
    use crate::types::{FieldError, Location};

    type TestResult = Result<(), Box<dyn Error>>;

    fn lined(
        file: impl Into<std::path::PathBuf>,
        line: usize,
    ) -> Result<Diagnostic, Box<dyn Error>> {
        let line = NonZeroUsize::new(line).ok_or("test line must be non-zero")?;
        Ok(Diagnostic::new(
            Location::at_line(file, line),
            FieldError::Empty,
        ))
    }

    fn keyed(file: &str, key: &str) -> Diagnostic {
        Diagnostic::new(Location::at_key(file, key), FieldError::Required)
    }

    fn plain(file: &str) -> Diagnostic {
        Diagnostic::new(Location::file(file), FieldError::UnknownKey)
    }

    fn shown(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(ToString::to_string).collect()
    }

    fn collect(items: Vec<Diagnostic>) -> Diagnostics {
        let mut diagnostics = Diagnostics::new();
        diagnostics.extend(items);
        diagnostics
    }

    #[test]
    fn new_collection_is_empty() {
        let diagnostics = Diagnostics::new();
        assert_eq!(diagnostics.len(), 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn push_and_extend_keep_every_diagnostic() {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(plain("a.md"));
        diagnostics.extend([plain("b.md"), plain("c.md")]);
        assert_eq!(diagnostics.len(), 3);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn into_sorted_orders_by_file() {
        let sorted = collect(vec![plain("b.md"), plain("a.md")]).into_sorted();
        assert_eq!(shown(&sorted), ["a.md: unknown key", "b.md: unknown key"]);
    }

    #[test]
    fn into_sorted_compares_files_as_slash_separated_bytes() -> TestResult {
        let nested = Path::new("a").join("b.md");
        let sorted = collect(vec![lined(nested, 1)?, lined("a-b.md", 1)?]).into_sorted();
        assert_eq!(shown(&sorted), ["a-b.md:1: empty", "a/b.md:1: empty"]);
        Ok(())
    }

    #[test]
    fn into_sorted_puts_unlined_before_lined_in_one_file() -> TestResult {
        let sorted =
            collect(vec![lined("a.md", 2)?, keyed("a.md", "x"), plain("a.md")]).into_sorted();
        assert_eq!(
            shown(&sorted),
            ["a.md: x: required", "a.md: unknown key", "a.md:2: empty"]
        );
        Ok(())
    }

    #[test]
    fn into_sorted_orders_lines_numerically() -> TestResult {
        let sorted = collect(vec![
            lined("a.md", 10)?,
            lined("a.md", 9)?,
            lined("a.md", 2)?,
        ])
        .into_sorted();
        assert_eq!(
            shown(&sorted),
            ["a.md:2: empty", "a.md:9: empty", "a.md:10: empty"]
        );
        Ok(())
    }

    #[test]
    fn into_sorted_keeps_push_order_for_many_equal_keys() {
        // More than 20 elements, so a sort that is not stable would have room to reorder ties.
        // Pushes alternate between two files; within each file every sort key is equal.
        let mut expected_a = Vec::new();
        let mut expected_b = Vec::new();
        let mut items = Vec::new();
        for i in 0..32 {
            let key = format!("k{i}");
            if i % 2 == 0 {
                expected_a.push(format!("a.md: {key}: required"));
                items.push(keyed("a.md", &key));
            } else {
                expected_b.push(format!("b.md: {key}: required"));
                items.push(keyed("b.md", &key));
            }
        }
        let expected: Vec<String> = expected_a.into_iter().chain(expected_b).collect();

        let sorted = collect(items).into_sorted();
        assert_eq!(shown(&sorted), expected);
    }

    #[test]
    fn into_sorted_keeps_found_order_for_ties() {
        let same_line = Diagnostic::new(
            Location::at_line("a.md", NonZeroUsize::MIN),
            FieldError::Required,
        );
        let sorted = collect(vec![
            Diagnostic::new(
                Location::at_line("a.md", NonZeroUsize::MIN),
                FieldError::Empty,
            ),
            same_line,
        ])
        .into_sorted();
        assert_eq!(shown(&sorted), ["a.md:1: empty", "a.md:1: required"]);

        let sorted = collect(vec![
            keyed("a.md", "pack.version"),
            keyed("a.md", "pack.api"),
        ])
        .into_sorted();
        assert_eq!(
            shown(&sorted),
            ["a.md: pack.version: required", "a.md: pack.api: required"]
        );

        let sorted = collect(vec![plain("a.md"), keyed("a.md", "x")]).into_sorted();
        assert_eq!(shown(&sorted), ["a.md: unknown key", "a.md: x: required"]);
    }

    #[test]
    fn into_sorted_mixed_files_and_lines() -> TestResult {
        let sorted = collect(vec![
            lined("b.md", 3)?,
            lined("a.md", 9)?,
            keyed("a.md", "x"),
            lined("a.md", 2)?,
        ])
        .into_sorted();
        assert_eq!(
            shown(&sorted),
            [
                "a.md: x: required",
                "a.md:2: empty",
                "a.md:9: empty",
                "b.md:3: empty"
            ]
        );
        Ok(())
    }

    #[test]
    fn a_single_diagnostic_converts_to_a_collection_of_one() {
        let diagnostics = Diagnostics::from(plain("a.md"));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(shown(&diagnostics.into_sorted()), ["a.md: unknown key"]);
    }

    #[test]
    fn extend_with_another_collection_keeps_all_and_sorts() -> TestResult {
        let mut first = collect(vec![lined("b.md", 1)?]);
        let second = collect(vec![lined("a.md", 3)?, keyed("a.md", "x")]);
        first.extend(second);
        assert_eq!(first.len(), 3);
        assert_eq!(
            shown(&first.into_iter().collect::<Vec<_>>()),
            ["a.md: x: required", "a.md:3: empty", "b.md:1: empty"]
        );
        Ok(())
    }

    #[test]
    fn into_result_is_ok_only_when_empty() -> TestResult {
        assert!(matches!(Diagnostics::new().into_result(7), Ok(7)));

        let items = collect(vec![lined("b.md", 1)?, lined("a.md", 1)?])
            .into_result(7)
            .err();
        assert_eq!(
            items.map(|refusals| shown(&refusals.into_sorted())),
            Some(vec!["a.md:1: empty".to_owned(), "b.md:1: empty".to_owned()])
        );
        Ok(())
    }
}
