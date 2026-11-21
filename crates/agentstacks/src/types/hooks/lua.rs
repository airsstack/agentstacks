//! A hook that runs a Lua script under airsl.
//!
//! It is separate from `HookAction` so the lua-only fields (`run`, `args`, `may_block`, `grants`)
//! are read and refused in one place, apart from the message kind.
//!
//! # Responsibilities
//!
//! - [`LuaHook`] holds the script, its arguments, whether it may block, and its grants.
//! - `LuaHook::read` reads `run` through [`HookRun`], `args` (each item scanned once, located at
//!   `<hook>.args[<index>]`), `may_block` (a boolean, false by default) and the `grants` table.
//! - `text` is refused as not allowed on a lua hook. There is no field for fail-open: every lua
//!   hook runs fail-open, so `fail_open` is an unknown key.
//!
//! # Non-responsibilities
//!
//! The hook's kind, event and selectors belong to [`HookAction`](crate::types::HookAction) and
//! [`Hook`](crate::types::Hook), and unknown keys are reported there, where every allowed key is known.

use crate::types::format::TomlFields;
use crate::types::hooks::grants::Grants;
use crate::types::hooks::run::HookRun;
use crate::types::manifest::ManifestError;
use crate::types::name::BinaryName;
use crate::types::tokens::LocatedToken;
use toml::Value;

/// A hook of kind `lua`.
///
/// # Examples
///
/// A lua hook is read from a `[[hooks]]` table of kind `lua`:
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{HookAction, PackManifest, Policy};
///
/// let text = r#"
/// [pack]
/// name = "core"
/// version = "1.0.0"
/// api = 0
/// description = "Shared skills"
///
/// [[hooks]]
/// kind = "lua"
/// event = "tool.after"
/// run = "hooks/format.lua"
/// args = ["--check"]
/// may_block = true
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Lua(lua) = hook.action() else {
///     return Err("a lua hook".into());
/// };
/// assert_eq!(lua.run().path().as_str(), "hooks/format.lua");
/// assert_eq!(lua.args(), ["--check"]);
/// assert!(lua.may_block());
/// assert_eq!(lua.grants().policy(), Policy::Confined);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Every lua hook runs fail-open, so there is no `fail_open` field:
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
/// kind = "lua"
/// event = "tool.after"
/// run = "hooks/format.lua"
/// fail_open = false
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(lines, ["packs/core/pack.toml: hooks[0].fail_open: unknown key"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaHook {
    run: HookRun,
    args: Vec<String>,
    may_block: bool,
    grants: Grants,
}

impl LuaHook {
    /// Reads the lua hook behind `fields`, refusing what is wrong in it.
    ///
    /// `binaries` are the pack's `requires.binaries`; every token found is appended to `tokens`.
    pub(crate) fn read(
        fields: &mut TomlFields,
        binaries: &[BinaryName],
        tokens: &mut Vec<LocatedToken>,
    ) -> Option<Self> {
        let before = fields.refusal_count();
        if fields.raw("text").is_some() {
            fields.refuse("text", ManifestError::NotOnLuaHook);
        }
        let run = HookRun::read(fields, tokens);
        let args = read_args(fields, tokens);
        let may_block = read_may_block(fields);
        let grants = fields.optional_table("grants").map(|mut table| {
            let grants = Grants::read(&mut table, binaries, tokens);
            fields.absorb(table);
            grants
        });
        let run = run?;
        (fields.refusal_count() == before).then(|| Self {
            run,
            args,
            may_block,
            grants: grants.unwrap_or_default(),
        })
    }

    /// The script.
    #[must_use]
    pub const fn run(&self) -> &HookRun {
        &self.run
    }

    /// The script's arguments, as written.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Whether the hook may block the call it is attached to.
    #[must_use]
    pub const fn may_block(&self) -> bool {
        self.may_block
    }

    /// What the hook may touch.
    #[must_use]
    pub const fn grants(&self) -> &Grants {
        &self.grants
    }
}

/// Reads `args`: each item is scanned once and kept as written.
fn read_args(fields: &mut TomlFields, tokens: &mut Vec<LocatedToken>) -> Vec<String> {
    let mut args = Vec::new();
    for (index, text) in fields.optional_string_list("args").unwrap_or_default() {
        let item_path = fields.item_path("args", index);
        if fields.scan_field(&item_path, &text, tokens) {
            args.push(text);
        }
    }
    args
}

/// Reads `may_block`: a boolean, false when absent. A value of another type is refused with this
/// field's own wording.
fn read_may_block(fields: &mut TomlFields) -> bool {
    match fields.raw("may_block") {
        None => false,
        Some(Value::Boolean(flag)) => *flag,
        Some(_) => {
            fields.refuse("may_block", ManifestError::ExpectedBool);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::{Policy, TokenKind, parse_toml_document};

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type Read<T> = Result<(T, Vec<LocatedToken>, Vec<String>), Box<dyn std::error::Error>>;

    fn read(text: &str) -> Read<Option<LuaHook>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[1]", table);
        let mut tokens = Vec::new();
        let hook = LuaHook::read(&mut fields, &[], &mut tokens);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((hook, tokens, shown))
    }

    const TYPICAL_EXAMPLE: &str = r#"
run       = "{{root core}}/hooks/enforce.lua"
args      = ["--manifest", "{{root}}/enforcement.json"]
may_block = false
[grants]
policy = "confined"
read   = ["/"]
write  = ["{{var tmpdir}}"]
env    = ["HOME", "TMPDIR"]
exec   = []
"#;

    #[test]
    fn lua_hook_parses_a_typical_example() -> TestResult {
        let (hook, tokens, shown) = read(TYPICAL_EXAMPLE)?;
        assert!(shown.is_empty(), "{shown:?}");
        let hook = hook.ok_or("a hook")?;
        assert_eq!(hook.run().written(), "{{root core}}/hooks/enforce.lua");
        assert_eq!(hook.args(), ["--manifest", "{{root}}/enforcement.json"]);
        assert!(!hook.may_block());
        assert_eq!(hook.grants().policy(), Policy::Confined);
        assert_eq!(hook.grants().read_paths(), ["/"]);
        let located: Vec<(TokenKind, String)> = tokens
            .iter()
            .map(|t| (t.token().kind(), t.location().to_string()))
            .collect();
        assert_eq!(
            located,
            [
                (
                    TokenKind::Root,
                    "packs/core/pack.toml: hooks[1].run".to_owned()
                ),
                (
                    TokenKind::Root,
                    "packs/core/pack.toml: hooks[1].args[1]".to_owned()
                ),
                (
                    TokenKind::Var,
                    "packs/core/pack.toml: hooks[1].grants.write[0]".to_owned()
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn lua_hook_defaults_may_block_to_false() -> TestResult {
        let (hook, _, shown) = read("run = \"hooks/x.lua\"")?;
        assert!(shown.is_empty(), "{shown:?}");
        let hook = hook.ok_or("a hook")?;
        assert!(!hook.may_block());
        assert!(hook.args().is_empty());
        assert_eq!(hook.grants(), &Grants::default());
        let (hook, _, _) = read("run = \"hooks/x.lua\"\nmay_block = true")?;
        assert!(hook.ok_or("a hook")?.may_block());
        Ok(())
    }

    #[test]
    fn lua_hook_refuses_each_rule() -> TestResult {
        let cases = [
            (
                "may_block = \"yes\"",
                "hooks[1].may_block: expected true or false",
            ),
            (
                "args = [1]",
                "hooks[1].args[0]: expected string, got integer",
            ),
            ("text = \"x\"", "hooks[1].text: not allowed on a lua hook"),
            ("grants = 1", "hooks[1].grants: expected table, got integer"),
        ];
        for (extra, message) in cases {
            let (hook, _, shown) = read(&format!("run = \"hooks/x.lua\"\n{extra}"))?;
            assert!(hook.is_none(), "{extra}");
            assert_eq!(
                shown,
                [format!("packs/core/pack.toml: {message}")],
                "{extra}"
            );
        }
        Ok(())
    }
}
