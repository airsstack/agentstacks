//! The official source, and classifying one `init` or `ext add` argument as an official name, a
//! path or a git URL.
//!
//! `init --pack <arg>` and `init --tool <arg>` take either a bare name from the official
//! repository or a directory in the project. `ext add <source>` also takes a git URL. The decision
//! is a rule over the text alone, so it lives here and the flows that call it stay a sequence of
//! steps.
//!
//! # Responsibilities
//!
//! - [`Flag`]: which option or command the argument came from, for the refusal text and the noun it
//!   uses.
//! - [`OFFICIAL_REPOSITORY`], [`TAG_PREFIX`], [`OFFICIAL_PACKS`], [`OFFICIAL_TOOLS`]: the one place
//!   the official source is named.
//! - [`official_major`], [`release_prefix`]: the rule that the official release tags keep the major
//!   of the `api`, and the text of that prefix.
//! - [`Argument`]: the result, either the official repository with the directory inside it, a
//!   project path, or (for `ext add` only) a git URL.
//! - [`ShorthandError`]: the refusal of a bare name the official repository does not hold.
//! - [`classify`]: the rule. For `ext add`, an argument containing `://` or starting with `git@` is
//!   a git URL. Then an argument containing `/` or starting with `.` is a path; anything else is an
//!   official name, even when a directory of that name exists.
//!
//! # Non-responsibilities
//!
//! Choosing among the tags of a repository belongs to `types/resolve`. Reading the directory a path
//! names and turning duplicates into one key belong to the `init` flow.

use std::fmt;

use crate::types::api::API_MAJOR;
use crate::types::project::GitUrl;
use crate::types::project::LocalPath;
use crate::types::project::SourceError;
use crate::types::project::SubPath;
use crate::types::report::Diagnostic;
use crate::types::report::Location;

/// The official repository that holds the packs and the tool extensions.
pub const OFFICIAL_REPOSITORY: &str = "https://github.com/airsstack/agentstacks";

/// The prefix of the release tags that mark a version of the official packs and extensions.
pub const TAG_PREFIX: &str = "packs-v";

/// The release prefix with the major of the `api` this agentstacks speaks, such as `packs-v1`: the
/// tags `init` and `ext add` pick from the official repository start with it.
#[must_use]
pub fn release_prefix() -> String {
    format!("{TAG_PREFIX}{API_MAJOR}")
}

/// The major version a release tag must have, or `None` when any major is accepted.
///
/// It is the major of the `api` when `url` is the official repository and `prefix` is
/// [`TAG_PREFIX`], so a binary never moves to a release of a later `api`. Every other source
/// accepts all its tags.
///
/// The URL is compared byte for byte with [`OFFICIAL_REPOSITORY`], so a `.git` suffix or another
/// case makes it a third-party source.
///
/// # Examples
///
/// ```
/// use agentstacks::types::GitUrl;
/// use agentstacks::types::shorthand::official_major;
///
/// let official = GitUrl::new("https://github.com/airsstack/agentstacks")?;
/// assert_eq!(official_major(&official, "packs-v"), Some(1));
/// assert_eq!(official_major(&official, "rel-"), None);
/// let other = GitUrl::new("https://github.com/acme/packs")?;
/// assert_eq!(official_major(&other, "packs-v"), None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn official_major(url: &GitUrl, prefix: &str) -> Option<u64> {
    (url.as_str() == OFFICIAL_REPOSITORY && prefix == TAG_PREFIX).then_some(API_MAJOR)
}

/// The pack names the official repository holds under `packs/`.
pub const OFFICIAL_PACKS: [&str; 5] = [
    "core",
    "sdlc",
    "journal",
    "guideline-rust",
    "guideline-python",
];

/// The tool names the official repository holds under `extensions/`.
pub const OFFICIAL_TOOLS: [&str; 3] = ["claude", "musecode", "opencode"];

/// One per-artifact release stream: the tag prefix and the flag it serves.
///
/// Packs resolve `packs-<name>-v*` tags, tools resolve `ext-<name>-v*` tags.
/// The table is closed: eight official streams, and the retired `packs-v`
/// bundle prefix is deliberately absent from it.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Flag, Stream, STREAM_PREFIXES};
///
/// assert_eq!(STREAM_PREFIXES.len(), 8);
/// assert_eq!(Stream::PREFIX_CORE, "packs-core-v");
/// assert_eq!(Stream::PREFIX_CLAUDE, "ext-claude-v");
/// assert!(STREAM_PREFIXES.iter().any(|stream| stream.flag() == Flag::Pack));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stream {
    /// The tag prefix, such as `packs-core-v` or `ext-claude-v`.
    prefix: &'static str,
    /// The flag the stream serves; tool streams cover both `--tool` and `ext add`.
    flag: Flag,
}

impl Stream {
    /// The `packs/core` stream prefix.
    pub const PREFIX_CORE: &'static str = "packs-core-v";
    /// The `packs/sdlc` stream prefix.
    pub const PREFIX_SDLC: &'static str = "packs-sdlc-v";
    /// The `packs/journal` stream prefix.
    pub const PREFIX_JOURNAL: &'static str = "packs-journal-v";
    /// The `packs/guideline-rust` stream prefix.
    pub const PREFIX_GUIDELINE_RUST: &'static str = "packs-guideline-rust-v";
    /// The `packs/guideline-python` stream prefix.
    pub const PREFIX_GUIDELINE_PYTHON: &'static str = "packs-guideline-python-v";
    /// The `extensions/claude` stream prefix.
    pub const PREFIX_CLAUDE: &'static str = "ext-claude-v";
    /// The `extensions/musecode` stream prefix.
    pub const PREFIX_MUSECODE: &'static str = "ext-musecode-v";
    /// The `extensions/opencode` stream prefix.
    pub const PREFIX_OPENCODE: &'static str = "ext-opencode-v";

    /// The tag prefix, such as `packs-core-v`.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        self.prefix
    }

    /// The flag the stream serves.
    #[must_use]
    pub const fn flag(self) -> Flag {
        self.flag
    }

    /// The artifact name the prefix holds, such as `core` in `packs-core-v`.
    #[must_use]
    pub fn name(self) -> Option<&'static str> {
        let (head, tail) = match self.flag {
            Flag::Pack => ("packs-", "-v"),
            Flag::Tool | Flag::Ext => ("ext-", "-v"),
        };
        self.prefix.strip_prefix(head)?.strip_suffix(tail)
    }
}

/// The eight per-artifact release streams, in pack then tool order.
pub const STREAM_PREFIXES: [Stream; 8] = [
    Stream {
        prefix: Stream::PREFIX_CORE,
        flag: Flag::Pack,
    },
    Stream {
        prefix: Stream::PREFIX_SDLC,
        flag: Flag::Pack,
    },
    Stream {
        prefix: Stream::PREFIX_JOURNAL,
        flag: Flag::Pack,
    },
    Stream {
        prefix: Stream::PREFIX_GUIDELINE_RUST,
        flag: Flag::Pack,
    },
    Stream {
        prefix: Stream::PREFIX_GUIDELINE_PYTHON,
        flag: Flag::Pack,
    },
    Stream {
        prefix: Stream::PREFIX_CLAUDE,
        flag: Flag::Tool,
    },
    Stream {
        prefix: Stream::PREFIX_MUSECODE,
        flag: Flag::Tool,
    },
    Stream {
        prefix: Stream::PREFIX_OPENCODE,
        flag: Flag::Tool,
    },
];

/// The stream prefix of an official name under a flag, or `None` for anything else.
///
/// `ext add` shares the tool streams. A pack name under a tool flag, or any
/// unofficial name, is not a stream.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Flag, stream_prefix};
///
/// assert_eq!(stream_prefix(Flag::Pack, "sdlc"), Some("packs-sdlc-v"));
/// assert_eq!(stream_prefix(Flag::Ext, "musecode"), Some("ext-musecode-v"));
/// assert_eq!(stream_prefix(Flag::Pack, "cmux"), None);
/// ```
#[must_use]
pub fn stream_prefix(flag: Flag, name: &str) -> Option<&'static str> {
    let kind = match flag {
        Flag::Pack => Flag::Pack,
        Flag::Tool | Flag::Ext => Flag::Tool,
    };
    STREAM_PREFIXES
        .iter()
        .find(|stream| stream.flag == kind && stream.name() == Some(name))
        .map(|stream| stream.prefix())
}

/// The major a stream tag must have on the official repository, or `None` when any major is accepted.
///
/// It is the major of the `api` when `url` is the official repository and `prefix` is one of the
/// eight stream prefixes, so a binary never moves to a release of a later `api`. The retired
/// `packs-v` bundle prefix is not a stream and keeps its own rule in [`official_major`].
///
/// # Examples
///
/// ```
/// use agentstacks::types::{GitUrl, stream_major};
/// use agentstacks::types::shorthand::OFFICIAL_REPOSITORY;
///
/// let official = GitUrl::new(OFFICIAL_REPOSITORY)?;
/// assert_eq!(stream_major(&official, "packs-core-v"), Some(1));
/// assert_eq!(stream_major(&official, "packs-v"), None);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub fn stream_major(url: &GitUrl, prefix: &str) -> Option<u64> {
    (url.as_str() == OFFICIAL_REPOSITORY && STREAM_PREFIXES.iter().any(|s| s.prefix == prefix))
        .then_some(API_MAJOR)
}

/// The `init` option or the command an argument came from.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Flag;
///
/// assert_eq!(Flag::Pack.to_string(), "--pack");
/// assert_eq!(Flag::Tool.noun(), "tool");
/// assert_eq!(Flag::Ext.to_string(), "ext add");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// `--pack`.
    Pack,
    /// `--tool`.
    Tool,
    /// The source of `ext add`: a tool, from a git URL as well as a path or an official name.
    Ext,
}

impl Flag {
    /// The word for what the option names: `pack` or `tool`.
    #[must_use]
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Pack => "pack",
            Self::Tool | Self::Ext => "tool",
        }
    }

    const fn official(self) -> &'static [&'static str] {
        match self {
            Self::Pack => &OFFICIAL_PACKS,
            Self::Tool | Self::Ext => &OFFICIAL_TOOLS,
        }
    }

    const fn directory(self) -> &'static str {
        match self {
            Self::Pack => "packs",
            Self::Tool | Self::Ext => "extensions",
        }
    }

    const fn takes_git_url(self) -> bool {
        matches!(self, Self::Ext)
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack | Self::Tool => write!(f, "--{}", self.noun()),
            Self::Ext => f.write_str("ext add"),
        }
    }
}

/// Where an `init` or `ext add` argument says a pack or tool comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// A name from the official repository: the repository and the directory inside it.
    Official {
        /// The official repository.
        url: GitUrl,
        /// `packs/<name>` for a pack, `extensions/<name>` for a tool.
        subdir: SubPath,
    },
    /// A directory in the project, as written.
    Path(LocalPath),
    /// A git URL, as written. Only [`Flag::Ext`] produces it.
    Git(GitUrl),
}

/// The refusal of an argument that is a bare name but not an official one.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Flag, ShorthandError};
///
/// assert_eq!(
///     ShorthandError::NotOfficial(Flag::Tool).to_string(),
///     "not an official tool; pass a path, or add a git source to agentstacks.toml",
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ShorthandError {
    /// The name is not one the official repository holds for this option.
    #[error("not an official {}; {}", .0.noun(), hint(*.0))]
    NotOfficial(Flag),
}

/// What to do instead of a name that is not official.
const fn hint(flag: Flag) -> &'static str {
    match flag {
        Flag::Pack | Flag::Tool => "pass a path, or add a git source to agentstacks.toml",
        Flag::Ext => "pass a path or a git URL",
    }
}

/// Classifies `arg`, given with `flag`, as an official name, a project path or a git URL.
///
/// For [`Flag::Ext`], an argument containing `://` or starting with `git@` is a git URL; `init`
/// takes none. Then an argument containing `/` or starting with `.` is a path. Anything else is a
/// name and must be official.
///
/// # Errors
///
/// A [`Diagnostic`] located at the flag and the argument as written (`--pack "x"`, `ext add "x"`):
/// an unknown name, a git URL that leads with `-`, or a path that is absolute or climbs
/// out of the project.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{Argument, Flag, classify_argument};
///
/// let Argument::Official { subdir, .. } = classify_argument(Flag::Pack, "core").map_err(|refused| refused.to_string())? else {
///     return Err("expected an official name".into());
/// };
/// assert_eq!(subdir.as_str(), "packs/core");
///
/// let refused = classify_argument(Flag::Pack, "x").err().ok_or("expected a refusal")?;
/// assert_eq!(
///     refused.to_string(),
///     "--pack \"x\": not an official pack; pass a path, or add a git source to agentstacks.toml",
/// );
///
/// let refused = classify_argument(Flag::Ext, "x").err().ok_or("expected a refusal")?;
/// assert_eq!(
///     refused.to_string(),
///     "ext add \"x\": not an official tool; pass a path or a git URL",
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn classify(flag: Flag, arg: &str) -> Result<Argument, Diagnostic> {
    let at = || Location::verbatim(format!("{flag} \"{arg}\""));
    if flag.takes_git_url() && (arg.contains("://") || arg.starts_with("git@")) {
        return GitUrl::new(arg)
            .map(Argument::Git)
            .map_err(|error| Diagnostic::new(at(), SourceError::from(error)));
    }
    if arg.contains('/') || arg.starts_with('.') {
        return LocalPath::new(arg)
            .map(Argument::Path)
            .map_err(|error| Diagnostic::new(at(), SourceError::from(error)));
    }
    if !flag.official().contains(&arg) {
        return Err(Diagnostic::new(at(), ShorthandError::NotOfficial(flag)));
    }
    let url = GitUrl::new(OFFICIAL_REPOSITORY)
        .map_err(|e| Diagnostic::new(at(), SourceError::from(e)))?;
    let subdir = SubPath::new(&format!("{}/{arg}", flag.directory()))
        .map_err(|e| Diagnostic::new(at(), SourceError::from(e)))?;
    Ok(Argument::Official { url, subdir })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FormatError;
    use crate::types::PackVersion;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn accepted(flag: Flag, arg: &str) -> Result<Argument, String> {
        classify(flag, arg).map_err(|refused| refused.to_string())
    }

    fn refusal(flag: Flag, arg: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(classify(flag, arg)
            .err()
            .ok_or("expected a refusal")?
            .to_string())
    }

    #[test]
    fn official_pack_expands_to_the_repository_and_packs_subdir() -> TestResult {
        let Argument::Official { url, subdir } = accepted(Flag::Pack, "core")? else {
            return Err("expected an official name".into());
        };
        assert_eq!(url.as_str(), "https://github.com/airsstack/agentstacks");
        assert_eq!(subdir.as_str(), "packs/core");
        Ok(())
    }

    #[test]
    fn official_tool_expands_to_extensions_subdir() -> TestResult {
        let Argument::Official { url, subdir } = accepted(Flag::Tool, "claude")? else {
            return Err("expected an official name".into());
        };
        assert_eq!(url.as_str(), OFFICIAL_REPOSITORY);
        assert_eq!(subdir.as_str(), "extensions/claude");
        Ok(())
    }

    #[test]
    fn official_tool_opencode_expands_to_extensions_subdir() -> TestResult {
        let Argument::Official { url, subdir } = accepted(Flag::Tool, "opencode")? else {
            return Err("expected an official name".into());
        };
        assert_eq!(url.as_str(), OFFICIAL_REPOSITORY);
        assert_eq!(subdir.as_str(), "extensions/opencode");
        Ok(())
    }

    #[test]
    fn official_tool_musecode_expands_to_extensions_subdir() -> TestResult {
        let Argument::Official { url, subdir } = accepted(Flag::Tool, "musecode")? else {
            return Err("expected an official name".into());
        };
        assert_eq!(url.as_str(), OFFICIAL_REPOSITORY);
        assert_eq!(subdir.as_str(), "extensions/musecode");
        Ok(())
    }

    #[test]
    fn every_official_pack_name_is_accepted() -> TestResult {
        for name in OFFICIAL_PACKS {
            let Argument::Official { subdir, .. } = accepted(Flag::Pack, name)? else {
                return Err(format!("{name} is not official").into());
            };
            assert_eq!(subdir.as_str(), format!("packs/{name}"));
        }
        assert_eq!(
            OFFICIAL_PACKS,
            [
                "core",
                "sdlc",
                "journal",
                "guideline-rust",
                "guideline-python"
            ]
        );
        Ok(())
    }

    #[test]
    fn cmux_is_not_an_official_pack() -> TestResult {
        assert_eq!(
            refusal(Flag::Pack, "cmux")?,
            "--pack \"cmux\": not an official pack; pass a path, or add a git source to agentstacks.toml"
        );
        Ok(())
    }

    #[test]
    fn slash_or_leading_dot_makes_a_path() -> TestResult {
        for text in ["./packs/core", "packs/x", ".hidden"] {
            assert_eq!(
                accepted(Flag::Pack, text)?,
                Argument::Path(LocalPath::new(text)?)
            );
        }
        Ok(())
    }

    #[test]
    fn bare_name_is_never_a_path() -> TestResult {
        assert!(matches!(
            accepted(Flag::Pack, "core")?,
            Argument::Official { .. }
        ));
        Ok(())
    }

    #[test]
    fn unknown_pack_is_refused_with_the_flag_location() -> TestResult {
        assert_eq!(
            refusal(Flag::Pack, "x")?,
            "--pack \"x\": not an official pack; pass a path, or add a git source to agentstacks.toml"
        );
        Ok(())
    }

    #[test]
    fn unknown_name_is_carried_as_the_shorthand_variant() -> TestResult {
        let refused = classify(Flag::Tool, "x")
            .err()
            .ok_or("expected a refusal")?;
        assert!(matches!(
            refused.error(),
            FormatError::Shorthand(ShorthandError::NotOfficial(Flag::Tool))
        ));
        Ok(())
    }

    #[test]
    fn unknown_tool_is_refused_with_the_tool_wording() -> TestResult {
        assert_eq!(
            refusal(Flag::Tool, "x")?,
            "--tool \"x\": not an official tool; pass a path, or add a git source to agentstacks.toml"
        );
        Ok(())
    }

    #[test]
    fn absolute_path_is_refused() -> TestResult {
        assert_eq!(
            refusal(Flag::Pack, "/x")?,
            "--pack \"/x\": absolute paths are not portable"
        );
        Ok(())
    }

    #[test]
    fn climbing_path_is_refused() -> TestResult {
        assert_eq!(
            refusal(Flag::Pack, "../x")?,
            "--pack \"../x\": must stay inside the project"
        );
        Ok(())
    }

    #[test]
    fn ext_classifies_url_path_and_official() -> TestResult {
        for text in ["https://example.com/x.git", "git@host:x.git"] {
            assert_eq!(
                accepted(Flag::Ext, text)?,
                Argument::Git(GitUrl::new(text)?)
            );
        }
        for text in ["./ext/x", "ext/x"] {
            assert_eq!(
                accepted(Flag::Ext, text)?,
                Argument::Path(LocalPath::new(text)?)
            );
        }
        let Argument::Official { url, subdir } = accepted(Flag::Ext, "claude")? else {
            return Err("expected an official name".into());
        };
        assert_eq!(url.as_str(), OFFICIAL_REPOSITORY);
        assert_eq!(subdir.as_str(), "extensions/claude");
        Ok(())
    }

    #[test]
    fn ext_not_official_text() -> TestResult {
        assert_eq!(
            refusal(Flag::Ext, "x")?,
            "ext add \"x\": not an official tool; pass a path or a git URL"
        );
        Ok(())
    }

    #[test]
    fn dash_name_is_not_official() -> TestResult {
        assert_eq!(
            refusal(Flag::Ext, "-x")?,
            "ext add \"-x\": not an official tool; pass a path or a git URL"
        );
        Ok(())
    }

    #[test]
    fn ext_dash_url_refused_verbatim() -> TestResult {
        assert_eq!(
            refusal(Flag::Ext, "--upload-pack=x://y")?,
            "ext add \"--upload-pack=x://y\": URL must not start with \"-\""
        );
        Ok(())
    }

    #[test]
    fn stream_prefixes_map_all_eight_to_api_major() -> TestResult {
        let official = GitUrl::new(OFFICIAL_REPOSITORY)?;
        for (flag, name, prefix) in [
            (Flag::Pack, "core", "packs-core-v"),
            (Flag::Pack, "sdlc", "packs-sdlc-v"),
            (Flag::Pack, "journal", "packs-journal-v"),
            (Flag::Pack, "guideline-rust", "packs-guideline-rust-v"),
            (Flag::Pack, "guideline-python", "packs-guideline-python-v"),
            (Flag::Tool, "claude", "ext-claude-v"),
            (Flag::Tool, "musecode", "ext-musecode-v"),
            (Flag::Tool, "opencode", "ext-opencode-v"),
        ] {
            assert_eq!(stream_prefix(flag, name), Some(prefix));
            assert_eq!(stream_major(&official, prefix), Some(API_MAJOR));
        }
        assert_eq!(stream_prefix(Flag::Ext, "claude"), Some("ext-claude-v"));
        Ok(())
    }

    #[test]
    fn unknown_prefix_maps_to_none() -> TestResult {
        let official = GitUrl::new(OFFICIAL_REPOSITORY)?;
        assert_eq!(stream_prefix(Flag::Pack, "cmux"), None);
        assert_eq!(stream_prefix(Flag::Pack, ""), None);
        assert_eq!(stream_prefix(Flag::Tool, "core"), None);
        assert_eq!(stream_prefix(Flag::Tool, "packs-core-v1.0.1"), None);
        assert_eq!(stream_major(&official, "packs-evil-v"), None);
        assert_eq!(stream_major(&official, ""), None);
        let other = GitUrl::new("https://github.com/acme/packs")?;
        assert_eq!(stream_major(&other, "packs-core-v"), None);
        Ok(())
    }

    #[test]
    fn git_suffix_is_third_party() -> TestResult {
        let suffixed = GitUrl::new("https://github.com/airsstack/agentstacks.git")?;
        for prefix in [
            "packs-core-v",
            "packs-sdlc-v",
            "packs-journal-v",
            "packs-guideline-rust-v",
            "packs-guideline-python-v",
            "ext-claude-v",
            "ext-musecode-v",
            "ext-opencode-v",
        ] {
            assert_eq!(stream_major(&suffixed, prefix), None);
        }
        Ok(())
    }

    #[test]
    fn legacy_bundle_rule_is_untouched() -> TestResult {
        let official = GitUrl::new(OFFICIAL_REPOSITORY)?;
        assert_eq!(official_major(&official, "packs-v"), Some(API_MAJOR));
        assert_eq!(stream_major(&official, "packs-v"), None);
        Ok(())
    }

    #[test]
    fn existing_refusals_still_hold() -> TestResult {
        assert!(refusal(Flag::Pack, "cmux")?.contains("not an official pack"));
        assert!(refusal(Flag::Tool, "x")?.contains("not an official tool"));
        assert!(PackVersion::new("1.2").is_err());
        Ok(())
    }

    #[test]
    fn init_still_takes_no_url() -> TestResult {
        for flag in [Flag::Pack, Flag::Tool] {
            let text = "https://example.com/x.git";
            assert_eq!(
                accepted(flag, text)?,
                Argument::Path(LocalPath::new(text)?),
                "{flag}"
            );
            assert_eq!(
                refusal(flag, "git@host:x.git")?,
                format!(
                    "{flag} \"git@host:x.git\": not an official {}; pass a path, or add a git source to agentstacks.toml",
                    flag.noun()
                )
            );
        }
        Ok(())
    }
}
