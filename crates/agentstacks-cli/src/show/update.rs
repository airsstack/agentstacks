//! The update list as text.
//!
//! # Responsibilities
//!
//! - [`update_text`] prints the `api` move first when there is one, then a row for each newer tag
//!   and for each note.
//! - [`has_newer`] says whether the `api` moves or any entry names a newer tag.
//!
//! # Non-responsibilities
//!
//! Asking the remotes, writing the new tags and choosing the exit code belong to the update flow
//! and the commands.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use agentstacks::app::update::Update;
use agentstacks::types::resolve::UpdateOutcome;
use agentstacks::types::{API, ApiCheck, LEGACY_API};

/// `  api: <LEGACY_API> → <API>` when the `api` moves, then one row per entry, lines joined by `\n`
/// with no trailing newline: `  <key>: <current> → <new>` for a newer tag, `  note     <note>` for
/// a note. Empty when the `api` stays and there are no entries.
pub(crate) fn update_text(found: &Update) -> String {
    let api =
        (found.api == ApiCheck::Legacy).then(|| format!("  api: {LEGACY_API} \u{2192} {API}"));
    let rows = found
        .entries
        .iter()
        .map(|entry| match (&entry.outcome, &entry.current) {
            (UpdateOutcome::NewTag(new), Some(current)) => {
                format!("  {}: {current} \u{2192} {new}", entry.key)
            }
            (UpdateOutcome::NewTag(new), None) => format!("  {}: {new}", entry.key),
            (UpdateOutcome::Note(note), _) => format!("  note     {note}"),
        });
    api.into_iter().chain(rows).collect::<Vec<_>>().join("\n")
}

/// Whether the `api` moves or any entry names a newer tag: whether there is anything to write.
pub(crate) fn has_newer(found: &Update) -> bool {
    found.api == ApiCheck::Legacy
        || found
            .entries
            .iter()
            .any(|entry| matches!(entry.outcome, UpdateOutcome::NewTag(_)))
}

#[cfg(test)]
mod tests {
    use agentstacks::types::name::PackName;
    use agentstacks::types::project::GitTag;
    use agentstacks::types::resolve::{SourceKey, UpdateEntry, UpdateNote};

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

    fn tags(entries: Vec<UpdateEntry>) -> Update {
        Update {
            api: ApiCheck::Current,
            entries,
        }
    }

    #[test]
    fn newer_tag_row_has_an_arrow() -> TestResult {
        assert_eq!(
            update_text(&tags(vec![newer()?])),
            "  packs.core: packs-v0.1.0 \u{2192} packs-v0.2.0"
        );
        Ok(())
    }

    #[test]
    fn note_row_is_prefixed() -> TestResult {
        assert_eq!(
            update_text(&tags(vec![note()?])),
            "  note     agentstacks.toml: packs.core: no newer tag than packs-v0.1.0"
        );
        Ok(())
    }

    #[test]
    fn update_shows_frozen_note_verbatim() -> TestResult {
        use agentstacks::types::project::GitUrl;
        let frozen = UpdateEntry {
            key: key()?,
            current: Some(GitTag::new("packs-v1.2.0")?),
            outcome: UpdateOutcome::Note(UpdateNote::Frozen {
                key: key()?,
                bundle: GitTag::new("packs-v1.2.0")?,
                url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
                stream: "packs-core-v".to_owned(),
                newest: Some(GitTag::new("packs-core-v1.0.3")?),
            }),
        };
        assert_eq!(
            update_text(&tags(vec![frozen])),
            "  note     agentstacks.toml: packs.core: pinned to bundle tag packs-v1.2.0 (frozen stream); packs-core-v1.0.3 is available — retag to migrate"
        );
        Ok(())
    }

    #[test]
    fn rows_keep_the_entry_order() -> TestResult {
        let text = update_text(&tags(vec![note()?, newer()?]));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("  note     "));
        assert!(lines[1].starts_with("  packs.core: "));
        Ok(())
    }

    #[test]
    fn has_newer_is_false_for_notes_only() -> TestResult {
        assert!(!has_newer(&tags(vec![note()?])));
        assert!(!has_newer(&tags(Vec::new())));
        assert!(has_newer(&tags(vec![note()?, newer()?])));
        Ok(())
    }

    #[test]
    fn update_text_puts_api_first() -> TestResult {
        let found = Update {
            api: ApiCheck::Legacy,
            entries: vec![newer()?],
        };
        assert_eq!(
            update_text(&found),
            "  api: 0 \u{2192} 1\n  packs.core: packs-v0.1.0 \u{2192} packs-v0.2.0"
        );
        Ok(())
    }

    #[test]
    fn api_move_alone_is_something_to_write() {
        let found = Update {
            api: ApiCheck::Legacy,
            entries: Vec::new(),
        };
        assert_eq!(update_text(&found), "  api: 0 \u{2192} 1");
        assert!(has_newer(&found));
    }
}
