//! The update list as text.
//!
//! # Responsibilities
//!
//! - [`update_text`] prints a row for each newer tag and for each note.
//! - [`has_newer`] says whether any entry names a newer tag.
//!
//! # Non-responsibilities
//!
//! Asking the remotes, writing the new tags and choosing the exit code belong to the update flow
//! and the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::types::resolve::{UpdateEntry, UpdateOutcome};

/// One row per entry, lines joined by `\n` with no trailing newline: `  <key>: <current> →
/// <new>` for a newer tag, `  note     <note>` for a note. Empty for no entries.
pub(crate) fn update_text(entries: &[UpdateEntry]) -> String {
    entries
        .iter()
        .map(|entry| match (&entry.outcome, &entry.current) {
            (UpdateOutcome::NewTag(new), Some(current)) => {
                format!("  {}: {current} \u{2192} {new}", entry.key)
            }
            (UpdateOutcome::NewTag(new), None) => format!("  {}: {new}", entry.key),
            (UpdateOutcome::Note(note), _) => format!("  note     {note}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether any entry names a newer tag.
pub(crate) fn has_newer(entries: &[UpdateEntry]) -> bool {
    entries
        .iter()
        .any(|entry| matches!(entry.outcome, UpdateOutcome::NewTag(_)))
}

#[cfg(test)]
mod tests {
    use agentstacks::types::name::PackName;
    use agentstacks::types::project::GitTag;
    use agentstacks::types::resolve::{SourceKey, UpdateNote};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn key() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("core")?))
    }

    fn newer() -> Result<UpdateEntry, Box<dyn std::error::Error>> {
        Ok(UpdateEntry {
            key: key()?,
            current: Some(GitTag::new("packs-v0.1.0")?),
            outcome: UpdateOutcome::NewTag(GitTag::new("packs-v0.2.0")?),
        })
    }

    fn note() -> Result<UpdateEntry, Box<dyn std::error::Error>> {
        Ok(UpdateEntry {
            key: key()?,
            current: Some(GitTag::new("packs-v0.1.0")?),
            outcome: UpdateOutcome::Note(UpdateNote::NoNewer {
                key: key()?,
                tag: GitTag::new("packs-v0.1.0")?,
            }),
        })
    }

    #[test]
    fn newer_tag_row_has_an_arrow() -> TestResult {
        assert_eq!(
            update_text(&[newer()?]),
            "  packs.core: packs-v0.1.0 \u{2192} packs-v0.2.0"
        );
        Ok(())
    }

    #[test]
    fn note_row_is_prefixed() -> TestResult {
        assert_eq!(
            update_text(&[note()?]),
            "  note     agentstacks.toml: packs.core: no newer tag than packs-v0.1.0"
        );
        Ok(())
    }

    #[test]
    fn rows_keep_the_entry_order() -> TestResult {
        let text = update_text(&[note()?, newer()?]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("  note     "));
        assert!(lines[1].starts_with("  packs.core: "));
        Ok(())
    }

    #[test]
    fn has_newer_is_false_for_notes_only() -> TestResult {
        assert!(!has_newer(&[note()?]));
        assert!(!has_newer(&[]));
        assert!(has_newer(&[note()?, newer()?]));
        Ok(())
    }
}
