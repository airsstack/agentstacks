//! The unified diff of an updated shared file.
//!
//! # Responsibilities
//!
//! - [`diff_lines`] turns the old and new bytes of a file into indented unified-diff lines.
//!
//! # Non-responsibilities
//!
//! Deciding which steps get a diff, and printing the lines, belong to the plan text.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use similar::TextDiff;

/// The unified diff from `old` to `new` as lines indented by four spaces: the `---` and `+++`
/// headers named `path`, then each hunk with three lines of context.
///
/// Both sides are decoded as UTF-8, with invalid bytes replaced. Identical texts give no lines.
pub(crate) fn diff_lines(path: &str, old: &[u8], new: &[u8]) -> Vec<String> {
    let old = String::from_utf8_lossy(old);
    let new = String::from_utf8_lossy(new);
    let diff = TextDiff::from_lines(old.as_ref(), new.as_ref());
    let text = diff
        .unified_diff()
        .context_radius(3)
        .header(path, path)
        .to_string();
    text.lines().map(|line| format!("    {line}")).collect()
}

#[cfg(test)]
mod tests {
    use super::diff_lines;

    #[test]
    fn identical_texts_give_no_lines() {
        assert!(diff_lines("a.json", b"same\n", b"same\n").is_empty());
    }

    #[test]
    fn changed_line_gives_headers_and_a_hunk() {
        let lines = diff_lines(".claude/settings.json", b"one\ntwo\n", b"one\n2\n");
        assert_eq!(lines[0], "    --- .claude/settings.json");
        assert_eq!(lines[1], "    +++ .claude/settings.json");
        assert!(lines[2].starts_with("    @@ "), "{lines:?}");
        assert!(lines.contains(&"    -two".to_owned()), "{lines:?}");
        assert!(lines.contains(&"    +2".to_owned()), "{lines:?}");
    }

    #[test]
    fn invalid_utf8_is_decoded_lossily() {
        let lines = diff_lines("f", &[0xff], b"a\n");
        assert!(
            lines.iter().any(|line| line.contains('\u{fffd}')),
            "{lines:?}"
        );
    }
}
