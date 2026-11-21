//! Reads one source entry of a project or lock file.
//!
//! It is one reader shared by the project file and the lock, so the source rules are written once
//! and the two files differ only in the [`PinRule`] they pass.
//!
//! # Responsibilities
//!
//! - `SourceFields::read_into` applies every source rule to one raw entry in a single pass and
//!   returns the fields, recording every refusal found, so a bad URL does not hide a bad pin.
//! - It reads only the keys `git`, `path`, `tag`, `rev` and `subdir`. Each present key must be a
//!   string, built through its value type: `git` as [`GitUrl`], `tag` as [`GitTag`], `rev` as
//!   [`CommitId`], `subdir` as [`SubPath::new`] and `path` as [`LocalPath::new`].
//! - It decides which keys go together: exactly one of `git` and `path`; `tag`, `rev` and `subdir`
//!   only with `git`; and the [`PinRule`] the caller passes for the pin keys of a git entry.
//!
//! # Non-responsibilities
//!
//! Refusing the keys an entry may not hold at all, and the entry's name, belong to the caller,
//! which knows its own key set.

use crate::types::format::TomlFields;
use crate::types::project::CommitId;
use crate::types::project::GitTag;
use crate::types::project::GitUrl;
use crate::types::project::LocalPath;
use crate::types::project::Pin;
use crate::types::project::PinRule;
use crate::types::project::SourceError;
use crate::types::project::SubPath;
use crate::types::report::FieldError;

/// The checked source keys of one entry.
///
/// A `rev` wins over a `tag` written beside it as the pin, but the `tag` is kept as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::types) enum SourceFields {
    /// A git repository pinned to a tag or a commit.
    Git {
        /// The repository URL.
        url: GitUrl,
        /// What the source is pinned to; a `rev` wins over a `tag` written beside it.
        pin: Pin,
        /// The `tag` as written, whether or not it is the pin.
        tag: Option<GitTag>,
        /// The directory inside the repository.
        subdir: Option<SubPath>,
    },
    /// A directory inside the project.
    Path(LocalPath),
}

impl SourceFields {
    /// Reads the source keys through `fields`, the reader over the entry at `key_path`, recording
    /// every refusal in it. `None` when anything was refused.
    pub(in crate::types) fn read_into(
        fields: &mut TomlFields,
        key_path: &str,
        rule: PinRule,
    ) -> Option<Self> {
        let before = fields.refusal_count();
        let url = read_value(fields, "git", GitUrl::new);
        let path = read_value(fields, "path", LocalPath::new);
        let tag = read_value(fields, "tag", GitTag::new);
        let rev = read_value(fields, "rev", CommitId::new);
        let subdir = read_value(fields, "subdir", SubPath::new);

        let present = |key: &str| fields.raw(key).is_some();
        let (has_git, has_tag, has_rev) = (present("git"), present("tag"), present("rev"));
        let git_only = has_tag || has_rev || present("subdir");
        match (has_git, present("path")) {
            (true, true) | (false, false) => fields.refuse_path(key_path, SourceError::GitOrPath),
            (false, true) if git_only => fields.refuse_path(key_path, SourceError::GitOnlyFields),
            (true, false) => match rule {
                PinRule::ExactlyOneOfTagRev if has_tag == has_rev => {
                    fields.refuse_path(key_path, SourceError::TagOrRev);
                }
                PinRule::RevRequired if !has_rev => fields.refuse("rev", FieldError::Required),
                _ => {}
            },
            (false, true) => {}
        }
        if fields.refusal_count() != before {
            return None;
        }

        match (url, path, rev, tag.clone()) {
            (Some(url), None, Some(rev), _) => Some(Self::Git {
                url,
                pin: Pin::Rev(rev),
                tag,
                subdir,
            }),
            (Some(url), None, None, Some(pinned)) => Some(Self::Git {
                url,
                pin: Pin::Tag(pinned),
                tag,
                subdir,
            }),
            (None, Some(path), ..) => Some(Self::Path(path)),
            _ => None,
        }
    }
}

/// Reads the string at `key` and builds its value with `build`, refusing at the key what is not
/// a string or what `build` refuses. An absent key is `None` with no refusal.
fn read_value<T, E: Into<SourceError>>(
    fields: &mut TomlFields,
    key: &str,
    build: impl FnOnce(&str) -> Result<T, E>,
) -> Option<T> {
    let text = fields.optional_string(key)?;
    match build(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            fields.refuse(key, error.into());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use toml::Table;

    use super::*;
    use crate::types::project::testing::read_fields;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";

    fn read(
        file: &str,
        key_path: &str,
        rule: PinRule,
        text: &str,
    ) -> Result<Result<SourceFields, Vec<String>>, Box<dyn std::error::Error>> {
        let entry: Table = text.parse()?;
        Ok(
            read_fields(&entry, Path::new(file), key_path, rule).map_err(|diagnostics| {
                diagnostics
                    .into_sorted()
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            }),
        )
    }

    fn project_read(
        key_path: &str,
        text: &str,
    ) -> Result<Result<SourceFields, Vec<String>>, Box<dyn std::error::Error>> {
        read(
            "agentstacks.toml",
            key_path,
            PinRule::ExactlyOneOfTagRev,
            text,
        )
    }

    fn refusals(key_path: &str, text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match project_read(key_path, text)? {
            Ok(source) => Err(format!("expected a refusal, read {source:?}").into()),
            Err(shown) => Ok(shown),
        }
    }

    #[test]
    fn source_fields_reads_git_and_path_forms() -> TestResult {
        let core = project_read(
            "packs.core",
            "git = \"https://github.com/airsstack/agentstacks\"\n\
             tag = \"packs-v0.1.0\"\n\
             subdir = \"packs/core\"\n",
        )?;
        assert_eq!(
            core,
            Ok(SourceFields::Git {
                url: GitUrl::new("https://github.com/airsstack/agentstacks")?,
                pin: Pin::Tag(GitTag::new("packs-v0.1.0")?),
                tag: Some(GitTag::new("packs-v0.1.0")?),
                subdir: Some(SubPath::new("packs/core")?),
            })
        );

        let by_rev = project_read(
            "packs.core",
            &format!("git = \"https://example.org/r\"\nrev = \"{HEX40}\"\n"),
        )?;
        assert_eq!(
            by_rev,
            Ok(SourceFields::Git {
                url: GitUrl::new("https://example.org/r")?,
                pin: Pin::Rev(CommitId::new(HEX40)?),
                tag: None,
                subdir: None,
            })
        );

        let team_lint = project_read("packs.team-lint", "path = \"packs/team-lint\"\n")?;
        assert_eq!(
            team_lint,
            Ok(SourceFields::Path(LocalPath::new("packs/team-lint")?))
        );
        Ok(())
    }

    #[test]
    fn source_fields_refuses_git_and_path_together_or_neither() -> TestResult {
        let both = "git = \"https://example.org/r\"\npath = \"packs/x\"\n";
        let neither = "subdir = \"packs/x\"\n";
        for text in [both, neither] {
            assert_eq!(
                refusals("packs.core", text)?,
                ["agentstacks.toml: packs.core: needs exactly one of git, path"],
                "{text}"
            );
        }
        assert_eq!(
            refusals("packs.core", "")?,
            ["agentstacks.toml: packs.core: needs exactly one of git, path"]
        );
        Ok(())
    }

    #[test]
    fn source_fields_both_git_and_path_gets_only_that_refusal() -> TestResult {
        let text = "git = \"https://example.org/r\"\npath = \"packs/x\"\nsubdir = \"packs/x\"\n";
        assert_eq!(
            refusals("packs.core", text)?,
            ["agentstacks.toml: packs.core: needs exactly one of git, path"]
        );
        let with_rev = format!("{text}rev = \"main\"\n");
        let shown: BTreeSet<String> = refusals("packs.core", &with_rev)?.into_iter().collect();
        let expected: BTreeSet<String> = [
            "agentstacks.toml: packs.core: needs exactly one of git, path",
            "agentstacks.toml: packs.core.rev: not a commit id",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn source_fields_refuses_git_only_fields_on_a_path_source() -> TestResult {
        let cases = [
            "path = \"packs/x\"\ntag = \"v1\"\n".to_owned(),
            format!("path = \"packs/x\"\nrev = \"{HEX40}\"\n"),
            "path = \"packs/x\"\nsubdir = \"y\"\n".to_owned(),
            format!("path = \"packs/x\"\ntag = \"v1\"\nrev = \"{HEX40}\"\nsubdir = \"y\"\n"),
        ];
        for text in cases {
            assert_eq!(
                refusals("packs.team-lint", &text)?,
                [
                    "agentstacks.toml: packs.team-lint: tag, rev and subdir apply only to git sources"
                ],
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn source_fields_locates_field_errors_at_their_key_path() -> TestResult {
        let git = "git = \"https://example.org/r\"\n";
        let cases = [
            (
                "packs.core",
                "git = \"-x\"\ntag = \"v1\"\n".to_owned(),
                "agentstacks.toml: packs.core.git: URL must not start with \"-\"",
            ),
            (
                "packs.core",
                format!("{git}rev = \"main\"\n"),
                "agentstacks.toml: packs.core.rev: not a commit id",
            ),
            (
                "packs.core",
                format!("{git}tag = \"v1\"\nsubdir = \"../x\"\n"),
                "agentstacks.toml: packs.core.subdir: must stay inside the repository",
            ),
            (
                "packs.team-lint",
                "path = \"/abs\"\n".to_owned(),
                "agentstacks.toml: packs.team-lint.path: absolute paths are not portable",
            ),
            (
                "packs.team-lint",
                "path = \"../x\"\n".to_owned(),
                "agentstacks.toml: packs.team-lint.path: must stay inside the project",
            ),
            (
                "packs.core",
                format!("{git}tag = \"\"\n"),
                "agentstacks.toml: packs.core.tag: empty",
            ),
            (
                "packs.core",
                "git = \" \"\ntag = \"v1\"\n".to_owned(),
                "agentstacks.toml: packs.core.git: empty",
            ),
            (
                "packs.core",
                format!("{git}tag = 5\n"),
                "agentstacks.toml: packs.core.tag: expected string, got integer",
            ),
        ];
        for (key_path, text, message) in cases {
            assert_eq!(refusals(key_path, &text)?, [message], "{text}");
        }
        Ok(())
    }

    #[test]
    fn exactly_one_of_tag_rev_refuses_neither_or_both() -> TestResult {
        let git = "git = \"https://example.org/r\"\n";
        let cases = [
            git.to_owned(),
            format!("{git}tag = \"v1\"\nrev = \"{HEX40}\"\n"),
        ];
        for text in cases {
            assert_eq!(
                refusals("packs.core", &text)?,
                ["agentstacks.toml: packs.core: needs exactly one of tag, rev"],
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn rev_required_keeps_tag_optional() -> TestResult {
        let lock = |text: &str| read("agentstacks.lock", "pack[1]", PinRule::RevRequired, text);
        let git = "git = \"https://example.org/r\"\n";

        let rev_only = lock(&format!("{git}rev = \"{HEX40}\"\n"))?.map_err(|s| s.join("; "))?;
        assert!(matches!(rev_only, SourceFields::Git { tag: None, .. }));

        let both =
            lock(&format!("{git}tag = \"v1\"\nrev = \"{HEX40}\"\n"))?.map_err(|s| s.join("; "))?;
        let v1 = GitTag::new("v1")?;
        assert!(matches!(both, SourceFields::Git { tag: Some(tag), .. } if tag == v1));

        for text in [format!("{git}tag = \"v1\"\n"), git.to_owned()] {
            assert_eq!(
                lock(&text)?.err(),
                Some(vec!["agentstacks.lock: pack[1].rev: required".to_owned()]),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn source_fields_reports_every_error_in_one_entry() -> TestResult {
        let shown: BTreeSet<String> =
            refusals("packs.core", "git = \"-x\"\ntag = \"v1\"\nrev = \"main\"\n")?
                .into_iter()
                .collect();
        let expected: BTreeSet<String> = [
            "agentstacks.toml: packs.core.git: URL must not start with \"-\"",
            "agentstacks.toml: packs.core.rev: not a commit id",
            "agentstacks.toml: packs.core: needs exactly one of tag, rev",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(shown, expected);
        Ok(())
    }

    #[test]
    fn source_fields_counts_a_malformed_pin_key_as_present() -> TestResult {
        let shown = refusals("packs.core", "git = \"https://example.org/r\"\ntag = 5\n")?;
        assert_eq!(
            shown,
            ["agentstacks.toml: packs.core.tag: expected string, got integer",]
        );
        Ok(())
    }
}
