//! The messages of the pack manifest files with a wording of their own.
//!
//! The generic shapes (`required`, `expected <type>, got <type>`, `empty`, `unknown key`) belong to
//! [`FieldError`](crate::types::FieldError); every other refusal a manifest field can meet is one
//! variant here, so a caller can match on it and the wording lives in one place.
//!
//! It is one file because every reader of `pack.toml` and `native/<tool>/hooks.toml` raises these
//! refusals, and one variant per wording keeps the wording in one place instead of one copy per
//! reader.
//!
//! # Responsibilities
//!
//! - [`ManifestError`] has a variant per manifest refusal, each displaying exactly the message the
//!   author reads after the location.
//! - It wraps the error of each validating constructor (names, version, range, path, glob, policy,
//!   option type, hook event) through one `#[from]` variant, transparently, so a refused value
//!   reads the same wherever it is refused.
//!
//! # Non-responsibilities
//!
//! Where a refusal is located belongs to the [`Diagnostic`](crate::types::Diagnostic) that carries it, and
//! which field raises which refusal belongs to the types that read the fields.

use crate::types::hooks::HookEvent;
use crate::types::hooks::InvalidHookEvent;
use crate::types::hooks::InvalidPolicy;
use crate::types::manifest::InvalidAirslReq;
use crate::types::manifest::InvalidExcludeGlob;
use crate::types::manifest::InvalidOptionType;
use crate::types::manifest::InvalidPackPath;
use crate::types::manifest::InvalidVersion;
use crate::types::name::InvalidEnvName;
use crate::types::name::InvalidName;
use crate::types::name::InvalidOptionName;
use crate::types::name::PackName;
use crate::types::report::ListedTwice;
use crate::types::report::NotLuaFile;
use crate::types::report::Quoted;
use crate::types::report::TypeWord;
use crate::types::report::UnsupportedApi;

/// A refusal of a field of `pack.toml` or `native/<tool>/hooks.toml`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{InvalidVersion, ManifestError, PackVersion};
///
/// let kind = ManifestError::KindUnknown {
///     text: "sh".to_owned(),
/// };
/// assert_eq!(kind.to_string(), "\"sh\" (expected message, lua)");
/// assert_eq!(
///     ManifestError::RequiresItself.to_string(),
///     "a pack cannot require itself"
/// );
///
/// // A refusal of a validating constructor reads the same wrapped as it does bare.
/// let bare: InvalidVersion = PackVersion::new("1.2").err().ok_or("expected a refusal")?;
/// assert_eq!(ManifestError::from(bare).to_string(), "\"1.2\" is not semver");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// A name that is not a valid pack, binary or agent name.
    #[error(transparent)]
    Name(#[from] InvalidName),
    /// A name that is not a valid environment variable name.
    #[error(transparent)]
    EnvName(#[from] InvalidEnvName),
    /// A name that is not a valid option name.
    #[error(transparent)]
    OptionName(#[from] InvalidOptionName),
    /// A version that is not a semantic version (a `pack.version`).
    #[error(transparent)]
    Version(#[from] InvalidVersion),
    /// A `requires.airsl` value that cannot be held as a range.
    #[error(transparent)]
    AirslReq(#[from] InvalidAirslReq),
    /// A path that cannot be held as a pack-relative path.
    #[error(transparent)]
    PackPath(#[from] InvalidPackPath),
    /// An `exclude` entry that cannot be held as a glob.
    #[error(transparent)]
    ExcludeGlob(#[from] InvalidExcludeGlob),
    /// A grant policy a pack hook may not use.
    #[error(transparent)]
    Policy(#[from] InvalidPolicy),
    /// An option `type` outside the three types.
    #[error(transparent)]
    OptionType(#[from] InvalidOptionType),
    /// A hook `event` outside the vocabulary.
    #[error(transparent)]
    Event(#[from] InvalidHookEvent),
    /// A `pack.api` this agentstacks does not speak, for a pack with a valid name.
    #[error("{pack}: api {api}")]
    PackUnsupportedApi {
        /// The pack that asked for the api.
        pack: PackName,
        /// The api asked for.
        api: UnsupportedApi,
    },
    /// A `pack.api` this agentstacks does not speak, for a pack without a valid name.
    #[error(transparent)]
    UnsupportedApi(#[from] UnsupportedApi),
    /// A list entry that repeats an earlier one.
    #[error(transparent)]
    ListedTwice(#[from] ListedTwice<Quoted>),
    /// A pack listed in its own `requires.packs`.
    #[error("a pack cannot require itself")]
    RequiresItself,
    /// An option `default` of another type than the option declares.
    #[error("expects {expected}, got {got}")]
    DefaultType {
        /// The declared type.
        expected: TypeWord,
        /// The type of the default.
        got: TypeWord,
    },
    /// A hook `run` that is not a Lua file.
    #[error(transparent)]
    NotLuaFile(#[from] NotLuaFile),
    /// A hook `run` written in none of the three allowed forms.
    #[error("must be pack-relative, {{{{root}}}}/… or {{{{root <pack>}}}}/…")]
    RunForm,
    /// A segment of a hook `run` that cannot name a module under the hook runtime.
    #[error("\"{run}\" segment \"{segment}\" may use only letters, digits, _ and -")]
    RunSegment {
        /// The `run` as written.
        run: String,
        /// The first segment that is refused.
        segment: String,
    },
    /// A grant path that is neither absolute nor starts with a token.
    #[error("\"{text}\" must be absolute or start with a token")]
    GrantPath {
        /// The refused path, as written.
        text: String,
    },
    /// An `exec` grant for a binary the pack does not require.
    #[error("\"{text}\" is not in requires.binaries")]
    NotInBinaries {
        /// The refused name, as written.
        text: String,
    },
    /// A hook `kind` that is neither `message` nor `lua`.
    #[error("\"{text}\" (expected message, lua)")]
    KindUnknown {
        /// The refused text.
        text: String,
    },
    /// A `match` on an event that has no selectors.
    #[error("not allowed on {event}")]
    MatchNotAllowed {
        /// The event the hook is on.
        event: HookEvent,
    },
    /// A `match` value the event does not accept.
    #[error("\"{text}\" not valid for {event}")]
    MatchNotValid {
        /// The refused value, as written.
        text: String,
        /// The event the hook is on.
        event: HookEvent,
    },
    /// An empty `match` or `agents` list.
    #[error("empty list never fires")]
    EmptyList,
    /// `agents` on an event other than `agent.stop`.
    #[error("only allowed on agent.stop")]
    AgentsNotAllowed,
    /// A field a message hook may not have.
    #[error("not allowed on a message hook")]
    NotOnMessageHook,
    /// A field a lua hook may not have.
    #[error("not allowed on a lua hook")]
    NotOnLuaHook,
    /// A `may_block` on a message hook.
    #[error("message hooks cannot block")]
    MessageCannotBlock,
    /// A `may_block` that is not a boolean.
    #[error("expected true or false")]
    ExpectedBool,
    /// A `render` entry that is code.
    #[error("\"{text}\" is code; code is installed verbatim")]
    RenderIsCode {
        /// The refused path, as written.
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    type Cases = Vec<(ManifestError, &'static str)>;

    fn text(value: &str) -> String {
        value.to_owned()
    }

    fn value_cases() -> Cases {
        vec![
            (
                ManifestError::Name(InvalidName::new("Core")),
                "\"Core\" is not a valid name",
            ),
            (
                ManifestError::OptionName(InvalidOptionName::new("a-b")),
                "\"a-b\" is not a valid option name",
            ),
            (
                ManifestError::EnvName(InvalidEnvName::new("home")),
                "\"home\" is not an environment variable name",
            ),
            (
                ManifestError::from(InvalidVersion::new("1.2")),
                "\"1.2\" is not semver",
            ),
            (
                ManifestError::from(InvalidAirslReq::NotVersionRange { text: text("nope") }),
                "\"nope\" is not a version range",
            ),
            (
                ManifestError::from(InvalidPackPath::Empty),
                "\"\" must not be empty",
            ),
            (
                ManifestError::from(InvalidExcludeGlob::NotAGlob { text: text("[") }),
                "\"[\" is not a valid glob",
            ),
            (
                ManifestError::from(InvalidPolicy::Unknown { text: text("foo") }),
                "\"foo\" (expected confined)",
            ),
            (
                ManifestError::from(InvalidOptionType::new("number")),
                "\"number\" (expected boolean, integer, string)",
            ),
            (
                ManifestError::from(InvalidHookEvent::new("tool.pre")),
                "\"tool.pre\" unknown",
            ),
        ]
    }

    fn metadata_cases() -> Result<Cases, InvalidName> {
        Ok(vec![
            (
                ManifestError::PackUnsupportedApi {
                    pack: PackName::new("sdlc")?,
                    api: UnsupportedApi::new(1),
                },
                "sdlc: api 1 not supported (this agentstacks speaks 0)",
            ),
            (
                ManifestError::from(UnsupportedApi::new(1)),
                "1 not supported (this agentstacks speaks 0)",
            ),
            (
                ManifestError::from(ListedTwice::text("core")),
                "\"core\" listed twice",
            ),
            (
                ManifestError::RequiresItself,
                "a pack cannot require itself",
            ),
            (
                ManifestError::RenderIsCode {
                    text: text("x.lua"),
                },
                "\"x.lua\" is code; code is installed verbatim",
            ),
            (
                ManifestError::DefaultType {
                    expected: TypeWord::Integer,
                    got: TypeWord::String,
                },
                "expects integer, got string",
            ),
        ])
    }

    fn hook_cases() -> Cases {
        vec![
            (
                ManifestError::from(NotLuaFile::new("x.sh")),
                "\"x.sh\" is not a .lua file",
            ),
            (
                ManifestError::RunForm,
                "must be pack-relative, {{root}}/… or {{root <pack>}}/…",
            ),
            (
                ManifestError::RunSegment {
                    run: text("hooks/my.hook.lua"),
                    segment: text("my.hook"),
                },
                "\"hooks/my.hook.lua\" segment \"my.hook\" may use only letters, digits, _ and -",
            ),
            (
                ManifestError::GrantPath { text: text("data") },
                "\"data\" must be absolute or start with a token",
            ),
            (
                ManifestError::NotInBinaries { text: text("git") },
                "\"git\" is not in requires.binaries",
            ),
        ]
    }

    fn selector_cases() -> Cases {
        vec![
            (
                ManifestError::KindUnknown { text: text("sh") },
                "\"sh\" (expected message, lua)",
            ),
            (
                ManifestError::MatchNotAllowed {
                    event: HookEvent::SessionEnd,
                },
                "not allowed on session.end",
            ),
            (
                ManifestError::MatchNotValid {
                    text: text("bash"),
                    event: HookEvent::ToolBefore,
                },
                "\"bash\" not valid for tool.before",
            ),
            (ManifestError::EmptyList, "empty list never fires"),
            (
                ManifestError::AgentsNotAllowed,
                "only allowed on agent.stop",
            ),
            (
                ManifestError::NotOnMessageHook,
                "not allowed on a message hook",
            ),
            (ManifestError::NotOnLuaHook, "not allowed on a lua hook"),
            (
                ManifestError::MessageCannotBlock,
                "message hooks cannot block",
            ),
            (ManifestError::ExpectedBool, "expected true or false"),
        ]
    }

    #[test]
    fn manifest_error_messages_match_their_wording() -> Result<(), Box<dyn std::error::Error>> {
        let mut cases = value_cases();
        cases.extend(metadata_cases()?);
        cases.extend(hook_cases());
        cases.extend(selector_cases());
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
        Ok(())
    }
}
