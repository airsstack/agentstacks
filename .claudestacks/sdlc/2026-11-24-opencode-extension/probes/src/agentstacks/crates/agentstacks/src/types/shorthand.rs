//! Classifying one `init` argument as an official name or a path.
//!
//! `init --pack <arg>` and `init --tool <arg>` take either a bare name from the official
//! repository or a directory in the project. The decision is a rule over the text alone, so it
//! lives here and the flow that calls it stays a sequence of steps.
//!
//! # Responsibilities
//!
//! - [`Flag`]: which option the argument came from, for the refusal text and the noun it uses.
//! - [`OFFICIAL_REPOSITORY`], [`TAG_PREFIX`], [`OFFICIAL_PACKS`], [`OFFICIAL_TOOLS`]: the one place
//!   the official source is named.
//! - [`Argument`]: the result, either the official repository with the directory inside it, or a
//!   project path.
//! - [`ShorthandError`]: the refusal of a bare name the official repository does not hold.
//! - [`classify`]: the rule. An argument containing `/` or starting with `.` is a path; anything
//!   else is an official name, even when a directory of that name exists.
//!
//! # Non-responsibilities
//!
//! Choosing the tag, reading the directory a path names, and turning duplicates into one key
//! belong to the `init` flow.

use std::fmt;

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

/// The pack names the official repository holds under `packs/`.
pub const OFFICIAL_PACKS: [&str; 5] = [
    "core",
    "sdlc",
    "journal",
    "guideline-rust",
    "guideline-python",
];

/// The tool names the official repository holds under `extensions/`.
pub const OFFICIAL_TOOLS: [&str; 1] = ["claude"];

/// The `init` option an argument came from.
///
/// # Examples
///
/// ```
/// use agentstacks::types::Flag;
///
/// assert_eq!(Flag::Pack.to_string(), "--pack");
/// assert_eq!(Flag::Tool.noun(), "tool");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// `--pack`.
    Pack,
    /// `--tool`.
    Tool,
}

impl Flag {
    /// The word for what the option names: `pack` or `tool`.
    #[must_use]
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Pack => "pack",
            Self::Tool => "tool",
        }
    }

    const fn official(self) -> &'static [&'static str] {
        match self {
            Self::Pack => &OFFICIAL_PACKS,
            Self::Tool => &OFFICIAL_TOOLS,
        }
    }

    const fn directory(self) -> &'static str {
        match self {
            Self::Pack => "packs",
            Self::Tool => "extensions",
        }
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "--{}", self.noun())
    }
}

/// Where an `init` argument says a pack or tool comes from.
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
}

/// The refusal of an `init` argument that is a bare name but not an official one.
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
    #[error("not an official {}; pass a path, or add a git source to agentstacks.toml", .0.noun())]
    NotOfficial(Flag),
}

/// Classifies `arg`, given with `flag`, as an official name or a project path.
///
/// An argument containing `/` or starting with `.` is a path. Anything else is a name and must be
/// official.
///
/// # Errors
///
/// A [`Diagnostic`] located at the flag and the argument as written (`--pack "x"`): an unknown
/// name, or a path that is absolute or climbs out of the project.
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
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn classify(flag: Flag, arg: &str) -> Result<Argument, Diagnostic> {
    let at = || Location::file(format!("{flag} \"{arg}\""));
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
}
