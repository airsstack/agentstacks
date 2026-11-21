//! The YAML frontmatter block at the top of a skill or agent file.
//!
//! Skills and agents share the same block shape, so the block is found and parsed once here,
//! before any metadata type checks its keys.
//!
//! # Responsibilities
//!
//! - [`Frontmatter::split`] finds the block between the opening `---` on line 1 and the first later
//!   `---` line, and the [`BodyStart`] right after it.
//! - A delimiter is a line that, without its line ending, is exactly `---`. Later `---` lines belong
//!   to the body.
//! - [`Frontmatter::mapping`] parses the block into its entries, or refuses it as one syntax
//!   diagnostic or as not being a mapping.
//!
//! # Non-responsibilities
//!
//! Checking the keys and values of the block belongs to the skill and agent metadata types.

use std::num::NonZeroUsize;
use std::path::Path;

use serde_saphyr::{DefaultMessageFormatter, MessageFormatter};

use super::syntax::syntax_diagnostic;
use crate::types::format::YamlValue;
use crate::types::frontmatter::body::BodyStart;
use crate::types::frontmatter::error::FrontmatterError;
use crate::types::report::Diagnostic;
use crate::types::report::Location;

/// The delimiter line of a frontmatter block.
const DELIMITER: &str = "---";

/// Whether `line`, with its line ending removed, is exactly the delimiter.
fn is_delimiter(line: &str) -> bool {
    let without_newline = line.strip_suffix('\n').unwrap_or(line);
    without_newline
        .strip_suffix('\r')
        .unwrap_or(without_newline)
        == DELIMITER
}

/// The YAML block of one Markdown file, and where its body begins.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::Frontmatter;
///
/// let text = "---\nname: review\n---\nBody text\n";
/// let block = Frontmatter::split(Path::new("SKILL.md"), text).map_err(|refusal| refusal.to_string())?;
/// assert_eq!(block.yaml(), "name: review\n");
/// assert_eq!(block.body_start().body(text), "Body text\n");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Frontmatter<'a> {
    file: &'a Path,
    yaml: &'a str,
    body_start: BodyStart,
}

impl<'a> Frontmatter<'a> {
    /// Splits `text`, the content of `file`, into its YAML block and body start.
    ///
    /// # Errors
    ///
    /// Returns a `missing frontmatter` [`Diagnostic`] at `file` when line 1 is not a `---` line or
    /// no later `---` line closes the block.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::Frontmatter;
    ///
    /// let file = Path::new("SKILL.md");
    /// assert!(Frontmatter::split(file, "---\na: 1\n---\n").is_ok());
    /// let refused = Frontmatter::split(file, "no block here\n").err().map(|refusal| refusal.to_string());
    /// assert_eq!(refused.as_deref(), Some("SKILL.md: missing frontmatter"));
    /// ```
    pub fn split(file: &'a Path, text: &'a str) -> Result<Self, Diagnostic> {
        let missing = || Diagnostic::new(Location::file(file), FrontmatterError::Missing);
        let mut lines = text.split_inclusive('\n');
        let opening = lines.next().ok_or_else(missing)?;
        if !is_delimiter(opening) {
            return Err(missing());
        }
        let yaml_start = opening.len();
        let mut line_start = yaml_start;
        for (index, line) in lines.enumerate() {
            let line_end = line_start + line.len();
            if is_delimiter(line) {
                // The closing delimiter is file line `index + 2`; the body starts on the next,
                // line `index + 3`, which is never zero.
                let body_line = NonZeroUsize::MIN.saturating_add(index + 2);
                return Ok(Self {
                    file,
                    yaml: &text[yaml_start..line_start],
                    body_start: BodyStart::new(line_end, body_line),
                });
            }
            line_start = line_end;
        }
        Err(missing())
    }

    /// Parses the block into its mapping entries, in source order.
    ///
    /// # Errors
    ///
    /// Returns one syntax [`Diagnostic`] when the YAML does not parse, located at the line the
    /// parser reports. Returns `frontmatter: expected a mapping, got <type>` when the block parses
    /// to anything but a mapping; an empty block parses to `null`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{Frontmatter, YamlValue};
    ///
    /// let block = Frontmatter::split(Path::new("SKILL.md"), "---\nname: review\n---\n")
    ///     .map_err(|refusal| refusal.to_string())?;
    /// let entries = block.mapping().map_err(|refusal| refusal.to_string())?;
    /// assert_eq!(entries, [("name".to_owned(), YamlValue::Str("review".to_owned()))]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn mapping(&self) -> Result<Vec<(String, YamlValue)>, Diagnostic> {
        match serde_saphyr::from_str::<YamlValue>(self.yaml) {
            Ok(YamlValue::Map(entries)) => Ok(entries),
            Ok(other) => Err(Diagnostic::new(
                Location::at_key(self.file, "frontmatter"),
                FrontmatterError::NotAMapping {
                    got: other.kind_word(),
                },
            )),
            Err(error) => Err(syntax_diagnostic(
                self.file,
                self.yaml,
                error.location().map(|location| location.line()),
                &DefaultMessageFormatter.format_message(&error),
            )),
        }
    }

    /// The YAML block between the delimiters, ending with its last line ending.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::Frontmatter;
    ///
    /// let block = Frontmatter::split(Path::new("a.md"), "---\nk: v\n---\n")
    ///     .map_err(|refusal| refusal.to_string())?;
    /// assert_eq!(block.yaml(), "k: v\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn yaml(&self) -> &'a str {
        self.yaml
    }

    /// Where the body begins.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::Frontmatter;
    ///
    /// let block = Frontmatter::split(Path::new("a.md"), "---\nk: v\n---\nbody\n")
    ///     .map_err(|refusal| refusal.to_string())?;
    /// assert_eq!(block.body_start().line().get(), 4);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn body_start(&self) -> BodyStart {
        self.body_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "SKILL.md";

    fn split(text: &str) -> Result<Frontmatter<'_>, String> {
        Frontmatter::split(Path::new(FILE), text).map_err(|d| d.to_string())
    }

    fn mapped(block: &str) -> Result<Vec<(String, YamlValue)>, String> {
        let text = format!("---\n{block}---\n");
        split(&text)?.mapping().map_err(|d| d.to_string())
    }

    fn mapped_error(block: &str) -> Option<String> {
        mapped(block).err()
    }

    const HINT: &str = "(a value starting with {{ must be quoted)";

    #[test]
    fn quoting_hint_on_value_starting_with_token() {
        assert_eq!(
            mapped_error("description: {{skill status}} runs it\n").as_deref(),
            Some(format!("SKILL.md:2: expected string scalar {HINT}").as_str())
        );
    }

    #[test]
    fn quoting_hint_on_list_item_starting_with_token() {
        assert_eq!(
            mapped_error("tools:\n  - {{skill x}}\n").as_deref(),
            Some(format!("SKILL.md:3: expected string scalar {HINT}").as_str())
        );
    }

    #[test]
    fn quoting_hint_on_flow_sequence_item() {
        assert_eq!(
            mapped_error("tools: [a, {{skill x}}]\n").as_deref(),
            Some(format!("SKILL.md:2: expected string scalar {HINT}").as_str())
        );
    }

    #[test]
    fn no_hint_when_reported_line_has_no_token() {
        assert_eq!(
            mapped_error("a: [1, 2\n").as_deref(),
            Some("SKILL.md:2: unclosed bracket '['")
        );
    }

    #[test]
    fn duplicate_key_is_a_syntax_error() {
        assert_eq!(
            mapped_error("a: 1\na: 2\n").as_deref(),
            Some(
                "SKILL.md:3: duplicate mapping key: a, set DuplicateKeyPolicy in Options if acceptable"
            )
        );
    }

    #[test]
    fn duplicate_key_on_a_token_line_gets_no_hint() {
        assert_eq!(
            mapped_error("a: run {{skill x}}\na: run {{skill y}}\n").as_deref(),
            Some(
                "SKILL.md:3: duplicate mapping key: a, set DuplicateKeyPolicy in Options if acceptable"
            )
        );
    }

    #[test]
    fn integer_beyond_i64_gets_its_own_message() {
        assert_eq!(
            mapped_error("a: 1\nn: 9223372036854775808\n").as_deref(),
            Some("SKILL.md:3: integer beyond i64")
        );
        assert_eq!(
            mapped_error("n: [18446744073709551615]\n").as_deref(),
            Some("SKILL.md:2: integer beyond i64")
        );
    }

    #[test]
    fn integer_at_i64_max_parses() -> TestResult {
        let entries = mapped("n: 9223372036854775807\n")?;
        assert_eq!(entries, vec![("n".to_owned(), YamlValue::Int(i64::MAX))]);
        Ok(())
    }

    #[test]
    fn quoted_token_value_parses() -> TestResult {
        let entries = mapped("description: \"{{skill status}} runs it\"\n")?;
        assert_eq!(
            entries,
            vec![(
                "description".to_owned(),
                YamlValue::Str("{{skill status}} runs it".to_owned())
            )]
        );
        Ok(())
    }

    #[test]
    fn empty_frontmatter_is_refused_as_null() {
        let refused = "SKILL.md: frontmatter: expected a mapping, got null";
        assert_eq!(mapped_error("").as_deref(), Some(refused));
        assert_eq!(mapped_error("# a comment\n").as_deref(), Some(refused));
    }

    #[test]
    fn non_mapping_frontmatter_is_refused() {
        assert_eq!(
            mapped_error("- a\n").as_deref(),
            Some("SKILL.md: frontmatter: expected a mapping, got array")
        );
        assert_eq!(
            mapped_error("just text\n").as_deref(),
            Some("SKILL.md: frontmatter: expected a mapping, got string")
        );
    }

    #[test]
    fn split_returns_yaml_and_body_start() -> TestResult {
        let text = "---\nname: x\n---\n# Title\n";
        let split = split(text)?;
        assert_eq!(split.yaml(), "name: x\n");
        assert_eq!(split.body_start().body(text), "# Title\n");
        assert_eq!(split.body_start().line().get(), 4);
        Ok(())
    }

    #[test]
    fn split_accepts_closing_line_at_end_of_file() -> TestResult {
        let text = "---\nname: x\n---";
        let split = split(text)?;
        assert_eq!(split.yaml(), "name: x\n");
        assert_eq!(split.body_start().body(text), "");
        Ok(())
    }

    #[test]
    fn split_stops_at_first_closing_line() -> TestResult {
        let text = "---\nname: x\n---\nintro\n---\nmore\n";
        let split = split(text)?;
        assert_eq!(split.yaml(), "name: x\n");
        assert_eq!(split.body_start().body(text), "intro\n---\nmore\n");
        Ok(())
    }

    #[test]
    fn split_accepts_crlf_delimiters() -> TestResult {
        let text = "---\r\nname: x\r\n---\r\n# Title\r\n";
        let split = split(text)?;
        assert_eq!(split.yaml(), "name: x\r\n");
        assert_eq!(split.body_start().body(text), "# Title\r\n");
        assert_eq!(split.body_start().line().get(), 4);
        Ok(())
    }

    #[test]
    fn split_refuses_missing_frontmatter() {
        let cases = [
            "# Title\nbody\n",
            "\n---\nname: x\n---\n",
            "---\nname: x\nno closing line\n",
        ];
        for text in cases {
            let refused = Frontmatter::split(Path::new(FILE), text)
                .err()
                .map(|d| d.to_string());
            assert_eq!(
                refused.as_deref(),
                Some("SKILL.md: missing frontmatter"),
                "{text:?}"
            );
        }
    }
}
