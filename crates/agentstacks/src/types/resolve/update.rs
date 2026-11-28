//! What an update says about one git source.
//!
//! It is separate from the tag rules because it decides per source what to ask the remote and how
//! to word the answer, while the rules only choose among tags.
//!
//! # Responsibilities
//!
//! - [`UpdateNote`] is a note about a source that is not updated, displaying its text.
//! - [`UpdateEntry`] is the answer for one git source: a newer tag, or a note. [`UpdateOutcome`] is
//!   that choice.
//! - [`UpdateQuery::for_source`] decides what an update needs from a source: nothing for a `path`
//!   source, a note for a `rev` pin or a tag with no version, or the tags to list.
//!   [`UpdateQuery::finish`] turns the listed tags into the entry.
//!
//! # Non-responsibilities
//!
//! Listing the tags, ordering the entries across sources and writing the new tag into
//! `agentstacks.toml` belong to the flows and the command.

use std::fmt;

use crate::types::Flag;
use crate::types::api::API_MAJOR;
use crate::types::manifest::PackVersion;
use crate::types::project::CommitId;
use crate::types::project::GitTag;
use crate::types::project::GitUrl;
use crate::types::project::Pin;
use crate::types::project::Source;
use crate::types::resolve::RemoteTag;
use crate::types::resolve::SourceKey;
use crate::types::resolve::TagVersion;
use crate::types::resolve::latest_tag;
use crate::types::resolve::tag::precedence;
use crate::types::shorthand::{
    OFFICIAL_REPOSITORY, TAG_PREFIX, official_major, stream_major, stream_prefix,
};

/// Why a git source is not updated.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, PackName};
/// use agentstacks::types::resolve::{SourceKey, UpdateNote};
///
/// let note = UpdateNote::NoNewer {
///     key: SourceKey::Pack(PackName::new("core")?),
///     tag: GitTag::new("packs-v0.1.0")?,
/// };
/// assert_eq!(
///     note.to_string(),
///     "agentstacks.toml: packs.core: no newer tag than packs-v0.1.0",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateNote {
    /// The tag has no version to compare from.
    NoVersion {
        /// The entry.
        key: SourceKey,
        /// The tag as written.
        tag: GitTag,
    },
    /// No tag with the same prefix is newer than the current one.
    NoNewer {
        /// The entry.
        key: SourceKey,
        /// The current tag.
        tag: GitTag,
    },
    /// The source is pinned to a revision, which an update leaves alone.
    PinnedRev {
        /// The entry.
        key: SourceKey,
        /// The pinned revision.
        rev: CommitId,
    },
    /// The source is pinned to a retired bundle tag, which an update never rewrites.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{GitTag, GitUrl, PackName};
    /// use agentstacks::types::resolve::{SourceKey, UpdateNote};
    ///
    /// let note = UpdateNote::Frozen {
    ///     key: SourceKey::Pack(PackName::new("core")?),
    ///     bundle: GitTag::new("packs-v1.2.0")?,
    ///     url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
    ///     stream: "packs-core-v".to_owned(),
    ///     newest: Some(GitTag::new("packs-core-v1.0.3")?),
    /// };
    /// assert_eq!(
    ///     note.to_string(),
    ///     "agentstacks.toml: packs.core: pinned to bundle tag packs-v1.2.0 (frozen stream); packs-core-v1.0.3 is available — retag to migrate",
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    Frozen {
        /// The entry.
        key: SourceKey,
        /// The pinned bundle tag.
        bundle: GitTag,
        /// The repository the stream would be listed from.
        url: GitUrl,
        /// The artifact stream prefix, such as `packs-core-v`.
        stream: String,
        /// The newest stream tag, when the stream lists any tag.
        newest: Option<GitTag>,
    },
}

impl fmt::Display for UpdateNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoVersion { key, tag } => write!(
                f,
                "{}: tag \"{tag}\" has no version to update from",
                key.location()
            ),
            Self::NoNewer { key, tag } => {
                write!(f, "{}: no newer tag than {tag}", key.location())
            }
            Self::PinnedRev { key, rev } => {
                write!(f, "{}: pinned to rev {rev}, not updated", key.location())
            }
            Self::Frozen {
                key,
                bundle,
                url,
                stream,
                newest,
            } => match newest {
                Some(tag) => write!(
                    f,
                    "{}: pinned to bundle tag {bundle} (frozen stream); {tag} is available — retag to migrate",
                    key.location()
                ),
                None => write!(
                    f,
                    "{}: pinned to bundle tag {bundle} (frozen stream); no {stream}{API_MAJOR} release tag in {url} yet",
                    key.location()
                ),
            },
        }
    }
}

/// What an update found for a git source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// A newer tag with the same prefix.
    NewTag(GitTag),
    /// The source stays as it is, for the reason the note gives.
    Note(UpdateNote),
}

/// The answer for one git source.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitTag, GitUrl, PackName, Pin, Source};
/// use agentstacks::types::resolve::{SourceKey, UpdateOutcome, UpdateQuery};
///
/// let source = Source::Git {
///     url: GitUrl::new("https://github.com/acme/packs")?,
///     pin: Pin::Tag(GitTag::new("packs-v0.1.0")?),
///     subdir: None,
/// };
/// let key = SourceKey::Pack(PackName::new("core")?);
/// let query = UpdateQuery::for_source(key, &source)
///     .ok_or("a git source")?
///     .map_err(|entry| format!("{entry:?}"))?;
/// assert_eq!(query.prefix, "packs-v");
/// assert_eq!(query.frozen, None);
/// // With no newer tag listed, the entry notes it.
/// let entry = query.finish(&[]);
/// assert!(matches!(entry.outcome, UpdateOutcome::Note(_)));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEntry {
    /// The entry.
    pub key: SourceKey,
    /// The tag the entry is pinned to now; `None` for a revision pin.
    pub current: Option<GitTag>,
    /// What the update found.
    pub outcome: UpdateOutcome,
}

/// The artifact stream of a source pinned to the retired bundle prefix on the official
/// repository, or `None` for anything else.
///
/// A bundle pin carries no artifact name in the tag, so the entry's key supplies it: pack names
/// resolve under `packs-<name>-v`, tool names under `ext-<name>-v`. A name the official
/// repository does not hold keeps the legacy query.
fn bundle_stream(key: &SourceKey, url: &GitUrl, prefix: &str) -> Option<String> {
    if url.as_str() != OFFICIAL_REPOSITORY || prefix != TAG_PREFIX {
        return None;
    }
    let (flag, name) = match key {
        SourceKey::Pack(name) => (Flag::Pack, name.as_str()),
        SourceKey::Tool(name) => (Flag::Tool, name.as_str()),
    };
    stream_prefix(flag, name).map(str::to_owned)
}

/// What an update asks the remote of one git source pinned to a versioned tag: the tags that start
/// with `prefix`.
///
/// For the official repository and a stream prefix, [`UpdateQuery::finish`] only considers the
/// tags of the major version of the `api`. A query built from a retired bundle pin instead
/// produces the frozen note, never a newer tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateQuery {
    /// The entry.
    pub key: SourceKey,
    /// The repository URL.
    pub url: GitUrl,
    /// The text before the version, which the listed tags must start with.
    pub prefix: String,
    /// The tag the entry is pinned to now.
    pub tag: GitTag,
    /// The version of that tag.
    pub version: PackVersion,
    /// The retired bundle tag the entry is pinned to, when the listed prefix is its stream.
    pub frozen: Option<GitTag>,
}

impl UpdateQuery {
    /// What an update needs from `source`: `None` for a `path` source, which an update does not
    /// touch; `Err` with the finished entry when the answer needs no remote (a `rev` pin, or a tag
    /// with no version); `Ok` with the query otherwise.
    #[must_use]
    pub fn for_source(key: SourceKey, source: &Source) -> Option<Result<Self, UpdateEntry>> {
        let Source::Git { url, pin, .. } = source else {
            return None;
        };
        let tag = match pin {
            Pin::Rev(rev) => {
                let note = UpdateNote::PinnedRev {
                    key: key.clone(),
                    rev: rev.clone(),
                };
                return Some(Err(UpdateEntry {
                    key,
                    current: None,
                    outcome: UpdateOutcome::Note(note),
                }));
            }
            Pin::Tag(tag) => tag,
        };
        let Some(TagVersion { prefix, version }) = TagVersion::split(tag) else {
            let note = UpdateNote::NoVersion {
                key: key.clone(),
                tag: tag.clone(),
            };
            return Some(Err(UpdateEntry {
                key,
                current: Some(tag.clone()),
                outcome: UpdateOutcome::Note(note),
            }));
        };
        let frozen = bundle_stream(&key, url, &prefix).map(|stream| (stream, tag.clone()));
        let (prefix, frozen) = match frozen {
            Some((stream, bundle)) => (stream, Some(bundle)),
            None => (prefix, None),
        };
        Some(Ok(Self {
            key,
            url: url.clone(),
            prefix,
            tag: tag.clone(),
            version,
            frozen,
        }))
    }

    /// The entry for the `tags` the remote listed: the latest tag when it is newer than the
    /// current one, otherwise the note that nothing is. Tags under an official prefix, bundle or
    /// stream, are limited to the major version of the `api`; the tags of any other source are
    /// all considered. A query built from a retired bundle pin reports the frozen note, never a
    /// newer tag.
    #[must_use]
    pub fn finish(self, tags: &[RemoteTag]) -> UpdateEntry {
        if let Some(bundle) = self.frozen {
            let newest = latest_tag(tags, &self.prefix, stream_major(&self.url, &self.prefix))
                .map(|(tag, _)| tag);
            let note = UpdateNote::Frozen {
                key: self.key.clone(),
                bundle,
                url: self.url.clone(),
                stream: self.prefix.clone(),
                newest,
            };
            return UpdateEntry {
                key: self.key,
                current: Some(self.tag),
                outcome: UpdateOutcome::Note(note),
            };
        }
        let major = official_major(&self.url, &self.prefix)
            .or_else(|| stream_major(&self.url, &self.prefix));
        let newer = latest_tag(tags, &self.prefix, major)
            .filter(|(_, version)| precedence(version, &self.version).is_gt())
            .map(|(tag, _)| tag);
        let outcome = newer.map_or_else(
            || {
                UpdateOutcome::Note(UpdateNote::NoNewer {
                    key: self.key.clone(),
                    tag: self.tag.clone(),
                })
            },
            UpdateOutcome::NewTag,
        );
        UpdateEntry {
            key: self.key,
            current: Some(self.tag),
            outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;
    use crate::types::name::ToolName;
    use crate::types::project::LocalPath;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";
    const OLD: &str = "aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd";
    const NEW: &str = "90e2a535db371753890122ed088c81bf3867be7a";

    fn core() -> Result<SourceKey, Box<dyn std::error::Error>> {
        Ok(SourceKey::Pack(PackName::new("core")?))
    }

    fn source_at(url: &str, tag: &str) -> Result<Source, Box<dyn std::error::Error>> {
        Ok(Source::Git {
            url: GitUrl::new(url)?,
            pin: Pin::Tag(GitTag::new(tag)?),
            subdir: None,
        })
    }

    fn tag_source(tag: &str) -> Result<Source, Box<dyn std::error::Error>> {
        source_at("https://github.com/acme/packs", tag)
    }

    fn remotes(names: &[&str]) -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        names
            .iter()
            .map(|name| {
                Ok(RemoteTag {
                    rev: CommitId::new(NEW)?,
                    tag: GitTag::new(name)?,
                })
            })
            .collect()
    }

    fn outcome_at(
        url: &str,
        current: &str,
        listed: &[&str],
    ) -> Result<UpdateOutcome, Box<dyn std::error::Error>> {
        let query = UpdateQuery::for_source(core()?, &source_at(url, current)?)
            .ok_or("a git source")?
            .map_err(|entry| format!("{entry:?}"))?;
        Ok(query.finish(&remotes(listed)?).outcome)
    }

    fn g1() -> Result<Vec<RemoteTag>, Box<dyn std::error::Error>> {
        [
            (OLD, "packs-v0.1.0"),
            (NEW, "packs-v0.10.0"),
            (NEW, "packs-v0.2.0"),
            (NEW, "packs-v0.9.0"),
        ]
        .iter()
        .map(|(rev, tag)| {
            Ok(RemoteTag {
                rev: CommitId::new(rev)?,
                tag: GitTag::new(tag)?,
            })
        })
        .collect()
    }

    fn query(tag: &str) -> Result<UpdateQuery, Box<dyn std::error::Error>> {
        UpdateQuery::for_source(core()?, &tag_source(tag)?)
            .ok_or("a git source")?
            .map_err(|entry| format!("{entry:?}").into())
    }

    fn note_of(entry: &UpdateEntry) -> Result<String, Box<dyn std::error::Error>> {
        match &entry.outcome {
            UpdateOutcome::Note(note) => Ok(note.to_string()),
            UpdateOutcome::NewTag(tag) => Err(format!("a new tag {tag}").into()),
        }
    }

    #[test]
    fn update_note_texts_match_the_spec() -> TestResult {
        let no_version = UpdateNote::NoVersion {
            key: core()?,
            tag: GitTag::new("main-latest")?,
        };
        assert_eq!(
            no_version.to_string(),
            "agentstacks.toml: packs.core: tag \"main-latest\" has no version to update from"
        );
        let no_newer = UpdateNote::NoNewer {
            key: core()?,
            tag: GitTag::new("packs-v0.1.0")?,
        };
        assert_eq!(
            no_newer.to_string(),
            "agentstacks.toml: packs.core: no newer tag than packs-v0.1.0"
        );
        let pinned = UpdateNote::PinnedRev {
            key: core()?,
            rev: CommitId::new(REV)?,
        };
        assert_eq!(
            pinned.to_string(),
            format!("agentstacks.toml: packs.core: pinned to rev {REV}, not updated")
        );
        Ok(())
    }

    #[test]
    fn update_skips_a_path_source() -> TestResult {
        let source = Source::Path(LocalPath::new("packs/core")?);
        assert_eq!(UpdateQuery::for_source(core()?, &source), None);
        Ok(())
    }

    #[test]
    fn update_notes_a_rev_pin() -> TestResult {
        let source = Source::Git {
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            pin: Pin::Rev(CommitId::new(REV)?),
            subdir: None,
        };
        let entry = UpdateQuery::for_source(core()?, &source)
            .ok_or("a git source")?
            .err()
            .ok_or("expected a note")?;
        assert_eq!(entry.current, None);
        assert_eq!(
            note_of(&entry)?,
            format!("agentstacks.toml: packs.core: pinned to rev {REV}, not updated")
        );
        Ok(())
    }

    #[test]
    fn update_notes_a_tag_without_version() -> TestResult {
        let entry = UpdateQuery::for_source(core()?, &tag_source("main-latest")?)
            .ok_or("a git source")?
            .err()
            .ok_or("expected a note")?;
        assert_eq!(entry.current, Some(GitTag::new("main-latest")?));
        assert_eq!(
            note_of(&entry)?,
            "agentstacks.toml: packs.core: tag \"main-latest\" has no version to update from"
        );
        Ok(())
    }

    #[test]
    fn update_finds_a_newer_tag() -> TestResult {
        let query = query("packs-v0.1.0")?;
        assert_eq!(query.prefix, "packs-v");
        let entry = query.finish(&g1()?);
        assert_eq!(entry.current, Some(GitTag::new("packs-v0.1.0")?));
        assert_eq!(
            entry.outcome,
            UpdateOutcome::NewTag(GitTag::new("packs-v0.10.0")?)
        );
        Ok(())
    }

    #[test]
    fn update_notes_nothing_newer() -> TestResult {
        let expected = "agentstacks.toml: packs.core: no newer tag than packs-v0.10.0";
        let entry = query("packs-v0.10.0")?.finish(&g1()?);
        assert_eq!(entry.current, Some(GitTag::new("packs-v0.10.0")?));
        assert_eq!(note_of(&entry)?, expected);
        let entry = query("packs-v0.10.0")?.finish(&[]);
        assert_eq!(note_of(&entry)?, expected);
        Ok(())
    }

    #[test]
    fn frozen_note_text_is_pinned() -> TestResult {
        let note = UpdateNote::Frozen {
            key: core()?,
            bundle: GitTag::new("packs-v1.2.0")?,
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            stream: "packs-core-v".to_owned(),
            newest: Some(GitTag::new("packs-core-v1.0.3")?),
        };
        assert_eq!(
            note.to_string(),
            "agentstacks.toml: packs.core: pinned to bundle tag packs-v1.2.0 (frozen stream); packs-core-v1.0.3 is available — retag to migrate"
        );
        let empty = UpdateNote::Frozen {
            key: core()?,
            bundle: GitTag::new("packs-v1.2.0")?,
            url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
            stream: "packs-core-v".to_owned(),
            newest: None,
        };
        assert_eq!(
            empty.to_string(),
            "agentstacks.toml: packs.core: pinned to bundle tag packs-v1.2.0 (frozen stream); no packs-core-v1 release tag in https://github.com/airsstack/agentstacks yet"
        );
        Ok(())
    }

    #[test]
    fn official_bundle_pin_reports_frozen_stream() -> TestResult {
        let query = UpdateQuery::for_source(
            core()?,
            &source_at("https://github.com/airsstack/agentstacks", "packs-v0.2.0")?,
        )
        .ok_or("a git source")?
        .map_err(|entry| format!("{entry:?}"))?;
        assert_eq!(query.prefix, "packs-core-v");
        let entry = query.finish(&remotes(&["packs-core-v1.0.1", "packs-core-v1.0.2"])?);
        assert_eq!(entry.current, Some(GitTag::new("packs-v0.2.0")?));
        assert_eq!(
            note_of(&entry)?,
            "agentstacks.toml: packs.core: pinned to bundle tag packs-v0.2.0 (frozen stream); packs-core-v1.0.2 is available — retag to migrate"
        );
        Ok(())
    }

    #[test]
    fn frozen_stream_without_tags_names_the_stream() -> TestResult {
        let entry = UpdateQuery::for_source(
            core()?,
            &source_at("https://github.com/airsstack/agentstacks", "packs-v1.0.0")?,
        )
        .ok_or("a git source")?
        .map_err(|entry| format!("{entry:?}"))?
        .finish(&[]);
        assert_eq!(
            note_of(&entry)?,
            "agentstacks.toml: packs.core: pinned to bundle tag packs-v1.0.0 (frozen stream); no packs-core-v1 release tag in https://github.com/airsstack/agentstacks yet"
        );
        Ok(())
    }

    #[test]
    fn official_stream_update_finds_newer_tag() -> TestResult {
        let outcome = outcome_at(
            "https://github.com/airsstack/agentstacks",
            "packs-core-v1.0.1",
            &[
                "packs-core-v1.0.1",
                "packs-core-v1.0.2",
                "packs-core-v2.0.0",
            ],
        )?;
        assert_eq!(
            outcome,
            UpdateOutcome::NewTag(GitTag::new("packs-core-v1.0.2")?)
        );
        Ok(())
    }

    #[test]
    fn mixed_pins_resolve_independently() -> TestResult {
        let url = "https://github.com/airsstack/agentstacks";
        let bundle = UpdateQuery::for_source(core()?, &source_at(url, "packs-v0.2.0")?)
            .ok_or("a git source")?
            .map_err(|entry| format!("{entry:?}"))?;
        assert_eq!(bundle.prefix, "packs-core-v");
        assert!(bundle.frozen.is_some());
        let stream = UpdateQuery::for_source(core()?, &source_at(url, "packs-core-v1.0.1")?)
            .ok_or("a git source")?
            .map_err(|entry| format!("{entry:?}"))?;
        assert_eq!(stream.prefix, "packs-core-v");
        assert_eq!(stream.frozen, None);
        let listed = remotes(&["packs-core-v1.0.1", "packs-core-v1.0.2"])?;
        assert!(matches!(
            bundle.finish(&listed).outcome,
            UpdateOutcome::Note(UpdateNote::Frozen { .. })
        ));
        assert_eq!(
            stream.finish(&listed).outcome,
            UpdateOutcome::NewTag(GitTag::new("packs-core-v1.0.2")?)
        );
        Ok(())
    }

    #[test]
    fn third_party_tags_unchanged() -> TestResult {
        let outcome = outcome_at(
            "https://github.com/acme/packs",
            "packs-v0.2.0",
            &["packs-v0.2.0", "packs-v1.0.0", "packs-v2.0.0"],
        )?;
        assert_eq!(outcome, UpdateOutcome::NewTag(GitTag::new("packs-v2.0.0")?));
        Ok(())
    }

    #[test]
    fn dot_git_url_is_third_party() -> TestResult {
        let outcome = outcome_at(
            "https://github.com/airsstack/agentstacks.git",
            "packs-v0.2.0",
            &["packs-v0.2.0", "packs-v1.0.0", "packs-v2.0.0"],
        )?;
        assert_eq!(outcome, UpdateOutcome::NewTag(GitTag::new("packs-v2.0.0")?));
        Ok(())
    }

    #[test]
    fn official_url_with_another_prefix_keeps_every_major() -> TestResult {
        let outcome = outcome_at(
            "https://github.com/airsstack/agentstacks",
            "rel-0.2.0",
            &["rel-0.2.0", "rel-2.0.0"],
        )?;
        assert_eq!(outcome, UpdateOutcome::NewTag(GitTag::new("rel-2.0.0")?));
        Ok(())
    }

    #[test]
    fn update_entry_for_a_tool_uses_its_key() -> TestResult {
        let key = SourceKey::Tool(ToolName::new("claude")?);
        let entry = UpdateQuery::for_source(key, &tag_source("main-latest")?)
            .ok_or("a git source")?
            .err()
            .ok_or("expected a note")?;
        assert!(note_of(&entry)?.starts_with("agentstacks.toml: tools.claude: "));
        Ok(())
    }
}
