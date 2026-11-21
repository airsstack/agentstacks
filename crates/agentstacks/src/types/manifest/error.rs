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

use crate::types::api::API;
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
use crate::types::report::TypeWord;

/// A refusal of a field of `pack.toml` or `native/<tool>/hooks.toml`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{InvalidVersion, ManifestError, PackVersion};
///
/// let repeated = ManifestError::ListedTwice {
///     text: "core".to_owned(),
/// };
/// assert_eq!(repeated.to_string(), "\"core\" listed twice");
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
    #[error("{pack}: api {api} not supported (this agentstacks speaks {})", API)]
    ApiUnsupported {
        /// The pack that asked for the api.
        pack: PackName,
        /// The api asked for.
        api: i64,
    },
    /// A `pack.api` this agentstacks does not speak, for a pack without a valid name.
    #[error("{api} not supported (this agentstacks speaks {})", API)]
    ApiUnsupportedUnnamed {
        /// The api asked for.
        api: i64,
    },
    /// A list entry that repeats an earlier one.
    #[error("\"{text}\" listed twice")]
    ListedTwice {
        /// The repeated entry.
        text: String,
    },
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
    #[error("\"{run}\" is not a .lua file")]
    NotLuaFile {
        /// The refused text.
        run: String,
    },
    /// A hook `run` written in none of the three allowed forms.
    #[error("must be pack-relative, {{{{root}}}}/… or {{{{root <pack>}}}}/…")]
    RunForm,
    /// A grant path that is neither absolute nor starts with a token.
    #[error("\"{path}\" must be absolute or start with a token")]
    GrantPath {
        /// The refused path.
        path: String,
    },
    /// An `exec` grant for a binary the pack does not require.
    #[error("\"{name}\" is not in requires.binaries")]
    NotInBinaries {
        /// The refused name.
        name: String,
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
    #[error("\"{value}\" not valid for {event}")]
    MatchNotValid {
        /// The refused value.
        value: String,
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
    #[error("\"{path}\" is code; code is installed verbatim")]
    RenderIsCode {
        /// The refused path.
        path: String,
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
                ManifestError::from(InvalidExcludeGlob::NotAGlob { pattern: text("[") }),
                "\"[\" is not a valid glob",
            ),
            (
                ManifestError::from(InvalidPolicy::Unknown { text: text("foo") }),
                "\"foo\" (expected confined, pure)",
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
                ManifestError::ApiUnsupported {
                    pack: PackName::new("sdlc")?,
                    api: 1,
                },
                "sdlc: api 1 not supported (this agentstacks speaks 0)",
            ),
            (
                ManifestError::ApiUnsupportedUnnamed { api: 1 },
                "1 not supported (this agentstacks speaks 0)",
            ),
            (
                ManifestError::ListedTwice { text: text("core") },
                "\"core\" listed twice",
            ),
            (
                ManifestError::RequiresItself,
                "a pack cannot require itself",
            ),
            (
                ManifestError::RenderIsCode {
                    path: text("x.lua"),
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
                ManifestError::NotLuaFile { run: text("x.sh") },
                "\"x.sh\" is not a .lua file",
            ),
            (
                ManifestError::RunForm,
                "must be pack-relative, {{root}}/… or {{root <pack>}}/…",
            ),
            (
                ManifestError::GrantPath { path: text("data") },
                "\"data\" must be absolute or start with a token",
            ),
            (
                ManifestError::NotInBinaries { name: text("git") },
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
                    value: text("bash"),
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
