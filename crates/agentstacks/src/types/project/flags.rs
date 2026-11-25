//! The `--tag`, `--rev` and `--subdir` options of `ext add` and the refusals they raise.
//!
//! # Responsibilities
//!
//! - [`GitFlags`] holds the three options as written: it refuses them beside a path or an official
//!   name, and builds the git [`Source`] from them beside a git URL.
//! - [`GitOption`] names one of those three options in the refusals.
//!
//! # Non-responsibilities
//!
//! Writing the tool into `agentstacks.toml` belongs to `add`, and the refusal texts to
//! [`ProjectError`].

use std::fmt;

use crate::types::project::{
    CommitId, GitTag, GitUrl, Pin, ProjectError, Source, SourceError, SubPath,
};

/// One of the git options of `ext add`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitOption {
    /// `--tag`.
    Tag,
    /// `--rev`.
    Rev,
    /// `--subdir`.
    Subdir,
}

impl fmt::Display for GitOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tag => "--tag",
            Self::Rev => "--rev",
            Self::Subdir => "--subdir",
        })
    }
}

/// The `--tag`, `--rev` and `--subdir` of `ext add`, as written.
///
/// # Examples
///
/// ```
/// use agentstacks::types::GitUrl;
/// use agentstacks::types::project::GitFlags;
///
/// let flags = GitFlags { tag: Some("v1.2.0"), rev: None, subdir: Some("ext/mytool") };
/// let source = flags.source(GitUrl::new("https://example.com/x.git")?);
/// assert!(source.is_ok());
/// let refused = GitFlags { tag: Some("v1"), rev: None, subdir: None }.without_git();
/// assert_eq!(refused[0].to_string(), "--tag applies only to a git URL");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct GitFlags<'a> {
    /// `--tag`.
    pub tag: Option<&'a str>,
    /// `--rev`.
    pub rev: Option<&'a str>,
    /// `--subdir`.
    pub subdir: Option<&'a str>,
}

impl GitFlags<'_> {
    /// One refusal for each option given, for a source that is a path or an official name.
    #[must_use]
    pub fn without_git(&self) -> Vec<ProjectError> {
        [
            (GitOption::Tag, self.tag),
            (GitOption::Rev, self.rev),
            (GitOption::Subdir, self.subdir),
        ]
        .into_iter()
        .filter(|(_, value)| value.is_some())
        .map(|(flag, _)| ProjectError::FlagNeedsGit { flag })
        .collect()
    }

    /// The git source of `url` with the pin and `subdir` the options give.
    ///
    /// # Errors
    ///
    /// Every refusal, in the order `--tag` or `--rev`, then `--subdir`: neither `--tag` nor
    /// `--rev`, both, or a value that is empty, not a commit id, or a directory outside the
    /// repository.
    pub fn source(&self, url: GitUrl) -> Result<Source, Vec<ProjectError>> {
        let mut refused = Vec::new();
        let pin = match (self.tag, self.rev) {
            (Some(_), Some(_)) => {
                refused.push(ProjectError::PinConflict);
                None
            }
            (Some(tag), None) => {
                value(GitOption::Tag, GitTag::new(tag), &mut refused).map(Pin::Tag)
            }
            (None, Some(rev)) => {
                value(GitOption::Rev, CommitId::new(rev), &mut refused).map(Pin::Rev)
            }
            (None, None) => {
                refused.push(ProjectError::GitNeedsPin);
                None
            }
        };
        let subdir = self.subdir.map_or(Some(None), |text| {
            value(GitOption::Subdir, SubPath::new(text), &mut refused).map(Some)
        });
        match (pin, subdir) {
            (Some(pin), Some(subdir)) if refused.is_empty() => Ok(Source::Git { url, pin, subdir }),
            _ => Err(refused),
        }
    }
}

/// `result`'s value, or `None` with its refusal added to `refused` under `flag`.
fn value<T, E: Into<SourceError>>(
    flag: GitOption,
    result: Result<T, E>,
    refused: &mut Vec<ProjectError>,
) -> Option<T> {
    match result {
        Ok(found) => Some(found),
        Err(error) => {
            refused.push(ProjectError::FlagValue {
                flag,
                error: error.into(),
            });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn mytool() -> Result<Source, Box<dyn std::error::Error>> {
        Ok(Source::Git {
            url: GitUrl::new("https://example.com/x.git")?,
            pin: Pin::Tag(GitTag::new("v1.2.0")?),
            subdir: Some(SubPath::new("ext/mytool")?),
        })
    }

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    fn flags_source(flags: GitFlags<'_>) -> Result<Source, Vec<String>> {
        let url = GitUrl::new("https://example.com/x.git").map_err(|e| vec![e.to_string()])?;
        flags
            .source(url)
            .map_err(|refused| refused.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn git_flags_build_the_source() -> TestResult {
        let flags = GitFlags {
            tag: Some("v1.2.0"),
            rev: None,
            subdir: Some("ext/mytool"),
        };
        assert_eq!(flags_source(flags), Ok(mytool()?));
        let by_rev = flags_source(GitFlags {
            tag: None,
            rev: Some(REV),
            subdir: None,
        });
        assert_eq!(
            by_rev,
            Ok(Source::Git {
                url: GitUrl::new("https://example.com/x.git")?,
                pin: Pin::Rev(CommitId::new(REV)?),
                subdir: None,
            })
        );
        Ok(())
    }

    #[test]
    fn git_flags_refuse_each_bad_value_under_its_flag() {
        let cases = [
            (GitFlags::default(), "a git URL needs --tag or --rev"),
            (
                GitFlags {
                    tag: Some("v1"),
                    rev: Some(REV),
                    subdir: None,
                },
                "--tag and --rev cannot be used together",
            ),
            (
                GitFlags {
                    tag: Some(" "),
                    ..GitFlags::default()
                },
                "--tag: empty",
            ),
            (
                GitFlags {
                    rev: Some("abc"),
                    ..GitFlags::default()
                },
                "--rev: not a commit id",
            ),
            (
                GitFlags {
                    tag: Some("v1"),
                    subdir: Some("../x"),
                    ..GitFlags::default()
                },
                "--subdir: must stay inside the repository",
            ),
            (
                GitFlags {
                    tag: Some("v1"),
                    subdir: Some(""),
                    ..GitFlags::default()
                },
                "--subdir: empty",
            ),
        ];
        for (flags, message) in cases {
            assert_eq!(flags_source(flags), Err(vec![message.to_owned()]));
        }
    }

    #[test]
    fn git_flags_report_the_pin_and_the_subdir_together() {
        let flags = GitFlags {
            rev: Some("abc"),
            subdir: Some("/x"),
            ..GitFlags::default()
        };
        assert_eq!(
            flags_source(flags),
            Err(vec![
                "--rev: not a commit id".to_owned(),
                "--subdir: must stay inside the repository".to_owned(),
            ])
        );
    }

    #[test]
    fn git_flags_without_git_refuse_each_option_given() {
        let flags = GitFlags {
            tag: Some("v1"),
            rev: None,
            subdir: Some(""),
        };
        let shown: Vec<String> = flags
            .without_git()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            shown,
            [
                "--tag applies only to a git URL",
                "--subdir applies only to a git URL"
            ]
        );
        assert!(GitFlags::default().without_git().is_empty());
    }
}
