//! A neutral pack hook: an event, its selectors and an action.
//!
//! It is separate from `NativeHook` because a neutral hook is checked against the closed event
//! vocabulary, which is what lets an extension translate it for any tool.
//!
//! # Responsibilities
//!
//! - [`Hook`] holds the event, the `match` values (none means every value), the `agents` (none
//!   means any agent) and the [`HookAction`].
//! - `Hook::read` reads one `[[hooks]]` item. The event must be in the vocabulary; `match` only
//!   where the event has selectors, with values from the event's set, not empty and without
//!   repeats; `agents` only on `agent.stop`, not empty, each a valid name. A refusal about the
//!   whole list (`match`, `agents`) is located at the list's key path, and a refusal about one
//!   entry at its indexed path (`hooks[1].match[0]`).
//! - With the event missing or unknown, `match` and `agents` are not checked.
//! - Any key that is not `kind`, `event`, `match`, `agents` or one of the action's keys is
//!   refused as unknown.
//!
//! # Non-responsibilities
//!
//! Whether an `agents` entry names an agent of the pack belongs to the checks over a loaded pack.

use toml::Value;

use crate::types::format::TomlFields;
use crate::types::hooks::MatchValue;
use crate::types::hooks::action::HookAction;
use crate::types::hooks::event::HookEvent;
use crate::types::manifest::ManifestError;
use crate::types::name::BinaryName;
use crate::types::name::ItemName;
use crate::types::report::ListedTwice;
use crate::types::tokens::LocatedToken;

/// A neutral hook of a pack.
///
/// # Examples
///
/// Hooks are read from the `[[hooks]]` tables of a [`PackManifest`](crate::types::PackManifest):
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{HookEvent, MatchValue, PackManifest};
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
/// event = "tool.before"
/// match = ["read", "edit"]
/// text = "Mind the plan."
///
/// [[hooks]]
/// kind = "message"
/// event = "agent.stop"
/// agents = ["reviewer"]
/// text = "Report your findings."
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let [tools, agent] = manifest.hooks() else {
///     return Err("two hooks".into());
/// };
/// assert_eq!(tools.event(), HookEvent::ToolBefore);
/// assert_eq!(tools.matches(), Some([MatchValue::Read, MatchValue::Edit].as_slice()));
/// assert_eq!(tools.agents(), None);
/// assert_eq!(agent.event(), HookEvent::AgentStop);
/// let agents = agent.agents().ok_or("an agents list")?;
/// assert_eq!(agents.first().map(|name| name.as_str()), Some("reviewer"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A `match` value the event does not accept is refused at its entry:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::PackManifest;
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
/// match = ["read"]
/// text = "Hello."
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/pack.toml: hooks[0].match[0]: \"read\" not valid for session.start"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    event: HookEvent,
    matches: Option<Vec<MatchValue>>,
    agents: Option<Vec<ItemName>>,
    action: HookAction,
}

impl Hook {
    /// Reads the `[[hooks]]` item behind `fields`, refusing what is wrong in it.
    ///
    /// `binaries` are the pack's `requires.binaries`; every token found is appended to `tokens`.
    pub(in crate::types) fn read(
        fields: &mut TomlFields,
        binaries: &[BinaryName],
        tokens: &mut Vec<LocatedToken>,
    ) -> Option<Self> {
        let event = fields.required_string("event").and_then(|text| {
            HookEvent::new(&text)
                .map_err(|error| fields.refuse("event", ManifestError::from(error)))
                .ok()
        });
        let before = fields.refusal_count();
        let matches = event.and_then(|event| read_matches(fields, event));
        let agents = event.and_then(|event| read_agents(fields, event));
        let action = HookAction::read(fields, binaries, tokens);
        let valid = fields.refusal_count() == before;
        let mut known = vec!["kind", "event", "match", "agents"];
        known.extend(HookAction::KEYS);
        fields.refuse_unknown_keys(&known);
        match (event, action) {
            (Some(event), Some(action)) if valid => Some(Self {
                event,
                matches,
                agents,
                action,
            }),
            _ => None,
        }
    }

    /// The event the hook is attached to.
    #[must_use]
    pub const fn event(&self) -> HookEvent {
        self.event
    }

    /// The `match` values; `None` means every value.
    #[must_use]
    pub fn matches(&self) -> Option<&[MatchValue]> {
        self.matches.as_deref()
    }

    /// The agents the hook is limited to; `None` means any agent.
    #[must_use]
    pub fn agents(&self) -> Option<&[ItemName]> {
        self.agents.as_deref()
    }

    /// What the hook does.
    #[must_use]
    pub const fn action(&self) -> &HookAction {
        &self.action
    }
}

/// Reads `match` for a hook on `event`; `None` when absent or refused.
fn read_matches(fields: &mut TomlFields, event: HookEvent) -> Option<Vec<MatchValue>> {
    let written = fields.raw("match")?;
    let written_empty = matches!(written, Value::Array(items) if items.is_empty());
    let Some(allowed) = event.match_values() else {
        fields.refuse("match", ManifestError::MatchNotAllowed { event });
        return None;
    };
    if written_empty {
        fields.refuse("match", ManifestError::EmptyList);
        return None;
    }
    let before = fields.refusal_count();
    let listed = fields.optional_string_list("match")?;
    let mut values: Vec<MatchValue> = Vec::new();
    for (index, text) in listed {
        let item_path = fields.item_path("match", index);
        match MatchValue::new(&text) {
            Ok(value) if allowed.contains(&value) => {
                if values.contains(&value) {
                    fields.refuse_path(&item_path, ManifestError::from(ListedTwice::text(text)));
                } else {
                    values.push(value);
                }
            }
            _ => fields.refuse_path(&item_path, ManifestError::MatchNotValid { text, event }),
        }
    }
    (fields.refusal_count() == before).then_some(values)
}

/// Reads `agents` for a hook on `event`; `None` when absent or refused. An empty list is refused
/// like an empty `match`: it names no agent, so the hook could never fire.
fn read_agents(fields: &mut TomlFields, event: HookEvent) -> Option<Vec<ItemName>> {
    let written = fields.raw("agents")?;
    let written_empty = matches!(written, Value::Array(items) if items.is_empty());
    if !event.allows_agents() {
        fields.refuse("agents", ManifestError::AgentsNotAllowed);
        return None;
    }
    if written_empty {
        fields.refuse("agents", ManifestError::EmptyList);
        return None;
    }
    let before = fields.refusal_count();
    let listed = fields.optional_string_list("agents")?;
    let mut names = Vec::new();
    for (index, text) in listed {
        match ItemName::new(&text) {
            Ok(name) => names.push(name),
            Err(error) => {
                fields.refuse_path(
                    &fields.item_path("agents", index),
                    ManifestError::from(error),
                );
            }
        }
    }
    (fields.refusal_count() == before).then_some(names)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn read(
        index: usize,
        text: &str,
    ) -> Result<(Option<Hook>, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, &format!("hooks[{index}]"), table);
        let hook = Hook::read(&mut fields, &[], &mut Vec::new());
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((hook, shown))
    }

    fn message(event_line: &str, extra: &str) -> String {
        format!("kind = \"message\"\ntext = \"x\"\n{event_line}\n{extra}\n")
    }

    #[test]
    fn hook_accepts_every_allowed_selector() -> TestResult {
        let cases = [
            message(
                "event = \"session.start\"",
                "match = [\"startup\", \"resume\", \"clear\", \"compact\"]",
            ),
            message(
                "event = \"tool.after\"",
                "match = [\"read\", \"edit\", \"write\"]",
            ),
            message("event = \"agent.stop\"", "agents = [\"coder\"]"),
            message("event = \"tool.before\"", ""),
            message("event = \"session.end\"", ""),
        ];
        for text in cases {
            let (hook, shown) = read(0, &text)?;
            assert!(shown.is_empty(), "{text}: {shown:?}");
            assert!(hook.is_some(), "{text}");
        }
        let (hook, _) = read(
            0,
            &message("event = \"session.start\"", "match = [\"startup\"]"),
        )?;
        let hook = hook.ok_or("a hook")?;
        assert_eq!(hook.event(), HookEvent::SessionStart);
        assert_eq!(hook.matches(), Some([MatchValue::Startup].as_slice()));
        assert_eq!(hook.agents(), None);
        assert!(matches!(hook.action(), HookAction::Message(_)));
        let (hook, _) = read(0, &message("event = \"tool.before\"", ""))?;
        assert_eq!(hook.ok_or("a hook")?.matches(), None);
        let (hook, _) = read(
            0,
            &message("event = \"agent.stop\"", "agents = [\"coder\"]"),
        )?;
        assert_eq!(
            hook.ok_or("a hook")?
                .agents()
                .map(|names| names.iter().map(ItemName::as_str).collect::<Vec<_>>()),
            Some(vec!["coder"])
        );
        Ok(())
    }

    #[test]
    fn hook_refuses_each_selector_rule() -> TestResult {
        let cases = [
            (
                1,
                message("event = \"session.end\"", "match = [\"startup\"]"),
                "hooks[1].match: not allowed on session.end",
            ),
            (
                1,
                message("event = \"prompt.submit\"", "match = [\"startup\"]"),
                "hooks[1].match: not allowed on prompt.submit",
            ),
            (
                1,
                message("event = \"agent.stop\"", "match = [\"startup\"]"),
                "hooks[1].match: not allowed on agent.stop",
            ),
            (
                1,
                message("event = \"tool.before\"", "match = [\"bash\"]"),
                "hooks[1].match[0]: \"bash\" not valid for tool.before",
            ),
            (
                1,
                message("event = \"tool.before\"", "match = []"),
                "hooks[1].match: empty list never fires",
            ),
            (
                1,
                message("event = \"tool.before\"", "match = [\"read\", \"read\"]"),
                "hooks[1].match[1]: \"read\" listed twice",
            ),
            (
                0,
                message("event = \"tool.before\"", "agents = [\"coder\"]"),
                "hooks[0].agents: only allowed on agent.stop",
            ),
            (
                0,
                message("event = \"agent.stop\"", "agents = []"),
                "hooks[0].agents: empty list never fires",
            ),
            (
                0,
                message("event = \"tool.before\"", "agents = []"),
                "hooks[0].agents: only allowed on agent.stop",
            ),
            (
                0,
                message("event = \"agent.stop\"", "agents = [\"Coder\"]"),
                "hooks[0].agents[0]: \"Coder\" is not a valid name",
            ),
            (
                0,
                "kind = \"message\"\ntext = \"x\"\n".to_owned(),
                "hooks[0].event: required",
            ),
        ];
        for (index, text, message) in cases {
            let (hook, shown) = read(index, &text)?;
            assert!(hook.is_none(), "{text}");
            assert_eq!(
                shown,
                [format!("packs/core/pack.toml: {message}")],
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn hook_refuses_only_the_item_error_for_a_non_empty_match_of_wrong_types() -> TestResult {
        let (hook, shown) = read(0, &message("event = \"tool.before\"", "match = [1]"))?;
        assert!(hook.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[0].match[0]: expected string, got integer"]
        );
        Ok(())
    }

    #[test]
    fn hook_skips_selectors_when_the_event_is_unknown() -> TestResult {
        let (hook, shown) = read(
            0,
            &message(
                "event = \"tool.pre\"",
                "match = [\"bash\"]\nagents = [\"coder\"]",
            ),
        )?;
        assert!(hook.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[0].event: \"tool.pre\" unknown"]
        );
        Ok(())
    }

    #[test]
    fn hook_refuses_unknown_keys_but_knows_every_action_key() -> TestResult {
        let (hook, shown) = read(0, &message("event = \"session.end\"", "fail_open = false"))?;
        assert!(hook.is_some());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[0].fail_open: unknown key"]
        );
        let (hook, shown) = read(
            1,
            "kind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\nfail_open = false\n",
        )?;
        assert!(hook.is_some());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[1].fail_open: unknown key"]
        );
        let (_, shown) = read(
            0,
            "kind = \"sh\"\nevent = \"session.end\"\nrun = \"x\"\ntext = \"x\"\nargs = []\ngrants = {}\nmay_block = true\n",
        )?;
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[0].kind: \"sh\" (expected message, lua)"]
        );
        Ok(())
    }
}
