//! What a hook does, chosen by its kind.
//!
//! It is a sum type of its own, not optional fields on a hook, so neutral and native hooks share one
//! set of action rules and a field that does not apply to a kind cannot be held.
//!
//! # Responsibilities
//!
//! - [`HookAction`] is a message or a lua script, so a field that does not apply to a kind has no
//!   place to live: a message hook has no `run` and a lua hook has no `text`.
//! - `HookAction::read` reads the `kind` field and hands the table to the reader of that kind.
//!   With `kind` missing or unknown, the kind-specific keys are neither checked nor reported as
//!   unknown.
//! - [`HookAction::KEYS`] lists the keys the two kinds read, so a caller reports every other key
//!   as unknown. All five are always known: a key on the wrong kind gets its own refusal instead.
//!
//! # Non-responsibilities
//!
//! The event, its selectors and the keys outside the action belong to the hook that holds the
//! action: [`Hook`](crate::types::Hook) for neutral hooks, [`NativeHook`](crate::types::NativeHook) for native
//! ones.

use crate::types::format::TomlFields;
use crate::types::hooks::lua::LuaHook;
use crate::types::hooks::message::MessageHook;
use crate::types::manifest::ManifestError;
use crate::types::name::BinaryName;
use crate::types::tokens::LocatedToken;

/// The action of a hook.
///
/// # Examples
///
/// Actions are read from the hooks of a [`PackManifest`](crate::types::PackManifest):
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{HookAction, PackManifest};
///
/// let text = r#"
/// [pack]
/// name = "core"
/// version = "1.0.0"
/// api = 0
/// description = "Shared skills"
///
/// [[hooks]]
/// kind = "message"
/// event = "session.start"
/// text = "Read the plan before you edit."
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Message(message) = hook.action() else {
///     return Err("a message hook".into());
/// };
/// assert_eq!(message.text(), "Read the plan before you edit.");
/// // Every key an action reads, whichever the kind.
/// assert_eq!(HookAction::KEYS, ["text", "run", "args", "grants", "may_block"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAction {
    /// Shows the agent a message.
    Message(MessageHook),
    /// Runs a Lua script.
    Lua(LuaHook),
}

impl HookAction {
    /// The keys the action readers know, whichever the kind.
    pub const KEYS: [&'static str; 5] = ["text", "run", "args", "grants", "may_block"];

    /// Reads the action of the hook behind `fields`, refusing what is wrong in it.
    ///
    /// `binaries` are the pack's `requires.binaries`; every token found is appended to `tokens`.
    pub(in crate::types) fn read(
        fields: &mut TomlFields,
        binaries: &[BinaryName],
        tokens: &mut Vec<LocatedToken>,
    ) -> Option<Self> {
        let kind = fields.required_string("kind")?;
        match kind.as_str() {
            "message" => MessageHook::read(fields, tokens).map(Self::Message),
            "lua" => LuaHook::read(fields, binaries, tokens).map(Self::Lua),
            _ => {
                fields.refuse("kind", ManifestError::KindUnknown { text: kind });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn read(text: &str) -> Result<(Option<HookAction>, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[0]", table);
        let action = HookAction::read(&mut fields, &[], &mut Vec::new());
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((action, shown))
    }

    #[test]
    fn hook_action_dispatches_on_kind() -> TestResult {
        let (message, shown) = read("kind = \"message\"\ntext = \"hi\"")?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(matches!(message, Some(HookAction::Message(_))));
        let (lua, shown) = read("kind = \"lua\"\nrun = \"hooks/x.lua\"")?;
        assert!(shown.is_empty(), "{shown:?}");
        assert!(matches!(lua, Some(HookAction::Lua(_))));
        Ok(())
    }

    #[test]
    fn hook_action_refuses_a_missing_or_unknown_kind() -> TestResult {
        let (action, shown) = read("kind = \"sh\"\nrun = \"x.sh\"\ntext = \"x\"")?;
        assert!(action.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[0].kind: \"sh\" (expected message, lua)"]
        );
        let (action, shown) = read("run = \"x.sh\"\ntext = \"x\"")?;
        assert!(action.is_none());
        assert_eq!(shown, ["packs/core/pack.toml: hooks[0].kind: required"]);
        Ok(())
    }

    #[test]
    fn hook_action_keys_name_every_kind_specific_key() {
        assert_eq!(
            HookAction::KEYS,
            ["text", "run", "args", "grants", "may_block"]
        );
    }
}
