//! A hook that shows the agent a message.
//!
//! It is separate from `LuaHook` because the two kinds share no fields and each refuses the other's,
//! which keeps each kind's reader small.
//!
//! # Responsibilities
//!
//! - [`MessageHook`] holds the message text, which may carry tokens.
//! - `MessageHook::read` reads `text` (required, not empty) and refuses the fields a message
//!   hook cannot have: `run`, `args` and `grants` are not allowed, and `may_block` is refused
//!   whichever value it holds, because a message cannot block.
//! - The text is scanned once and its tokens are kept located at `<hook>.text`.
//!
//! # Non-responsibilities
//!
//! The hook's kind, event and selectors belong to [`HookAction`](crate::types::HookAction) and
//! [`Hook`](crate::types::Hook).

use crate::types::format::TomlFields;
use crate::types::manifest::ManifestError;
use crate::types::tokens::LocatedToken;

/// A hook of kind `message`.
///
/// # Examples
///
/// A message hook is read from a `[[hooks]]` table of kind `message`:
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
/// text = "Use {{skill discuss}} before you design."
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Message(message) = hook.action() else {
///     return Err("a message hook".into());
/// };
/// assert_eq!(message.text(), "Use {{skill discuss}} before you design.");
/// // The token in the text is kept, located at its key path.
/// assert_eq!(manifest.tokens().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A message cannot block, so `may_block` is refused whichever value it holds:
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
/// text = "Hello."
/// may_block = false
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/pack.toml: hooks[0].may_block: message hooks cannot block"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHook {
    text: String,
}

impl MessageHook {
    /// Reads the message hook behind `fields`, refusing what is wrong in it.
    ///
    /// Every token found in `text` is appended to `tokens`.
    pub(crate) fn read(fields: &mut TomlFields, tokens: &mut Vec<LocatedToken>) -> Option<Self> {
        let before = fields.refusal_count();
        let text = fields.required_text("text").filter(|text| {
            let key_path = fields.key_path("text");
            fields.scan_field(&key_path, text, tokens)
        });
        for key in ["run", "args", "grants"] {
            if fields.raw(key).is_some() {
                fields.refuse(key, ManifestError::NotOnMessageHook);
            }
        }
        if fields.raw("may_block").is_some() {
            fields.refuse("may_block", ManifestError::MessageCannotBlock);
        }
        let text = text?;
        (fields.refusal_count() == before).then_some(Self { text })
    }

    /// The message text, as written.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::{TokenKind, parse_toml_document};

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type Read<T> = Result<(T, Vec<LocatedToken>, Vec<String>), Box<dyn std::error::Error>>;

    fn read(text: &str) -> Read<Option<MessageHook>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[0]", table);
        let mut tokens = Vec::new();
        let hook = MessageHook::read(&mut fields, &mut tokens);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((hook, tokens, shown))
    }

    #[test]
    fn message_hook_parses_a_typical_example() -> TestResult {
        let (hook, tokens, shown) = read(
            "kind = \"message\"\nevent = \"session.end\"\ntext = \"Reminder: run {{skill snapshot-save}} to flush durable session learnings.\"\n",
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(
            hook.ok_or("a hook")?.text(),
            "Reminder: run {{skill snapshot-save}} to flush durable session learnings."
        );
        assert_eq!(tokens.len(), 1);
        let token = tokens.first().ok_or("a token")?;
        assert_eq!(token.token().kind(), TokenKind::Skill);
        assert_eq!(
            token.location().to_string(),
            "packs/core/pack.toml: hooks[0].text"
        );
        Ok(())
    }

    #[test]
    fn message_hook_refuses_each_rule() -> TestResult {
        let cases = [
            ("", "hooks[0].text: required"),
            ("text = \"\"", "hooks[0].text: empty"),
            (
                "text = \"x\"\nrun = \"x.lua\"",
                "hooks[0].run: not allowed on a message hook",
            ),
            (
                "text = \"x\"\nargs = []",
                "hooks[0].args: not allowed on a message hook",
            ),
            (
                "text = \"x\"\ngrants = {}",
                "hooks[0].grants: not allowed on a message hook",
            ),
            (
                "text = \"x\"\nmay_block = true",
                "hooks[0].may_block: message hooks cannot block",
            ),
            (
                "text = \"x\"\nmay_block = false",
                "hooks[0].may_block: message hooks cannot block",
            ),
        ];
        for (text, message) in cases {
            let (hook, _, shown) = read(text)?;
            assert!(hook.is_none(), "{text}");
            assert_eq!(
                shown,
                [format!("packs/core/pack.toml: {message}")],
                "{text}"
            );
        }
        Ok(())
    }
}
