//! A hook written for one tool's own event vocabulary.
//!
//! It is separate from `Hook` because a native hook keeps its tool's event and `match` as opaque
//! text, where the neutral hook checks them against a vocabulary; one type for both would hold the
//! wrong thing for one of them.
//!
//! # Responsibilities
//!
//! - [`NativeHook`] holds a tool-specific event and `match` as opaque strings, and the same
//!   [`HookAction`] a neutral hook has.
//! - `NativeHook::read` reads one `[[hooks]]` item of `native/<tool>/hooks.toml`: `event` is
//!   required text, `match` is an optional list of strings, and the action, with its grants,
//!   policy, `may_block`, `exec` and `run` rules, is read exactly as for a neutral hook.
//! - `agents` is not accepted and is refused as an unknown key.
//!
//! # Non-responsibilities
//!
//! Whether the tool knows the event or the `match` value belongs to the tool's extension; the
//! core keeps both opaque.

use crate::types::format::TomlFields;
use crate::types::hooks::action::HookAction;
use crate::types::name::BinaryName;
use crate::types::tokens::LocatedToken;

/// A hook of a tool's `native/<tool>/hooks.toml`.
///
/// # Examples
///
/// A native hook keeps its tool's event and `match` as opaque text:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{HookAction, NativeHooks, Requires};
///
/// let text = r#"
/// [[hooks]]
/// kind = "message"
/// event = "PreToolUse"
/// match = ["Bash"]
/// text = "Mind the plan."
/// "#;
/// let file = Path::new("packs/core/native/claude/hooks.toml");
/// let hooks = NativeHooks::parse(file, text, &Requires::default())
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = hooks.hooks().first().ok_or("one hook")?;
/// assert_eq!(hook.event(), "PreToolUse");
/// assert_eq!(hook.matches(), Some(["Bash".to_owned()].as_slice()));
/// assert!(matches!(hook.action(), HookAction::Message(_)));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// `agents` is not a native key, so it is refused as unknown:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{NativeHooks, Requires};
///
/// let text = r#"
/// [[hooks]]
/// kind = "message"
/// event = "Stop"
/// agents = ["reviewer"]
/// text = "Done."
/// "#;
/// let file = Path::new("packs/core/native/claude/hooks.toml");
/// let refused = NativeHooks::parse(file, text, &Requires::default())
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/native/claude/hooks.toml: hooks[0].agents: unknown key"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHook {
    event: String,
    matches: Option<Vec<String>>,
    action: HookAction,
}

impl NativeHook {
    /// Reads the `[[hooks]]` item behind `fields`, refusing what is wrong in it.
    ///
    /// `binaries` are the pack's `requires.binaries`; every token found is appended to `tokens`.
    pub(in crate::types) fn read(
        fields: &mut TomlFields,
        binaries: &[BinaryName],
        tokens: &mut Vec<LocatedToken>,
    ) -> Option<Self> {
        let event = fields.required_text("event");
        let matches = fields
            .optional_string_list("match")
            .map(|listed| listed.into_iter().map(|(_, text)| text).collect());
        let before = fields.refusal_count();
        let action = HookAction::read(fields, binaries, tokens);
        let valid = fields.refusal_count() == before;
        let mut known = vec!["kind", "event", "match"];
        known.extend(HookAction::KEYS);
        fields.refuse_unknown_keys(&known);
        match (event, action) {
            (Some(event), Some(action)) if valid => Some(Self {
                event,
                matches,
                action,
            }),
            _ => None,
        }
    }

    /// The tool's event, as written.
    #[must_use]
    pub fn event(&self) -> &str {
        &self.event
    }

    /// The tool's `match` values, as written; `None` when the hook has no `match`.
    #[must_use]
    pub fn matches(&self) -> Option<&[String]> {
        self.matches.as_deref()
    }

    /// What the hook does.
    #[must_use]
    pub const fn action(&self) -> &HookAction {
        &self.action
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "packs/core/native/claude/hooks.toml";

    fn read(text: &str) -> Result<(Option<NativeHook>, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new(FILE);
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[0]", table);
        let hook = NativeHook::read(&mut fields, &[], &mut Vec::new());
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((hook, shown))
    }

    #[test]
    fn native_hook_accepts_tool_specific_event_and_match() -> TestResult {
        let (hook, shown) = read(
            "kind = \"lua\"\nevent = \"PreToolUse\"\nmatch = [\"SubagentHandback\"]\nmay_block = true\nrun = \"hooks/handoff-check.lua\"\n",
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        let hook = hook.ok_or("a hook")?;
        assert_eq!(hook.event(), "PreToolUse");
        assert_eq!(
            hook.matches(),
            Some(["SubagentHandback".to_owned()].as_slice())
        );
        assert!(matches!(
            hook.action(),
            HookAction::Lua(lua) if lua.may_block() && lua.run().written() == "hooks/handoff-check.lua"
        ));
        let (hook, _) = read("kind = \"message\"\nevent = \"Stop\"\ntext = \"x\"\n")?;
        assert_eq!(hook.ok_or("a hook")?.matches(), None);
        Ok(())
    }

    #[test]
    fn native_hook_keeps_the_neutral_rules() -> TestResult {
        let cases = [
            (
                "kind = \"lua\"\nevent = \"E\"\nrun = \"hooks/x.lua\"\n[grants]\npolicy = \"trusted\"\n",
                "hooks[0].grants.policy: \"trusted\" is not allowed for pack hooks",
            ),
            (
                "kind = \"lua\"\nevent = \"E\"\nrun = \"hooks/x.lua\"\n[grants]\nexec = [\"git\"]\n",
                "hooks[0].grants.exec[0]: \"git\" is not in requires.binaries",
            ),
            (
                "kind = \"lua\"\nevent = \"E\"\nrun = \"x.sh\"\n",
                "hooks[0].run: \"x.sh\" is not a .lua file",
            ),
            (
                "kind = \"message\"\nevent = \"E\"\ntext = \"x\"\nmay_block = true\n",
                "hooks[0].may_block: message hooks cannot block",
            ),
            (
                "kind = \"lua\"\nevent = \"E\"\nrun = \"hooks/x.lua\"\ntext = \"x\"\n",
                "hooks[0].text: not allowed on a lua hook",
            ),
            (
                "kind = \"sh\"\nevent = \"E\"\n",
                "hooks[0].kind: \"sh\" (expected message, lua)",
            ),
        ];
        for (text, message) in cases {
            let (hook, shown) = read(text)?;
            assert!(hook.is_none(), "{text}");
            assert_eq!(shown, [format!("{FILE}: {message}")], "{text}");
        }
        Ok(())
    }

    #[test]
    fn native_hook_refuses_event_and_match_shapes() -> TestResult {
        let cases = [
            (
                "kind = \"message\"\ntext = \"x\"\n",
                "hooks[0].event: required",
            ),
            (
                "kind = \"message\"\ntext = \"x\"\nevent = \"\"\n",
                "hooks[0].event: empty",
            ),
            (
                "kind = \"message\"\ntext = \"x\"\nevent = \"E\"\nmatch = [1]\n",
                "hooks[0].match[0]: expected string, got integer",
            ),
            (
                "kind = \"message\"\ntext = \"x\"\nevent = \"E\"\nagents = [\"coder\"]\n",
                "hooks[0].agents: unknown key",
            ),
        ];
        for (text, message) in cases {
            let (_, shown) = read(text)?;
            assert_eq!(shown, [format!("{FILE}: {message}")], "{text}");
        }
        Ok(())
    }
}
