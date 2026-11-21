//! What a Lua hook is allowed to touch.
//!
//! It is separate from `LuaHook` because a grants table has its own entry-by-entry rules and
//! refusals, and the extension host reads it apart from the rest of the hook.
//!
//! # Responsibilities
//!
//! - [`Grants`] holds a hook's policy and the paths, environment variables and binaries it may
//!   use. The policy is `confined` unless the hook says `pure`; nothing is granted by default.
//! - `Grants::read` reads a `grants` table. A read or write path must be absolute or start with
//!   a `{{root}}`, `{{skill_dir}}` or `{{var}}` token; an environment variable must be a valid
//!   name; an exec grant must be `airsl` or a binary the pack requires. An environment variable or
//!   binary listed twice is refused, like a repeat in `requires`.
//! - Every refusal about one entry of `read`, `write`, `env` or `exec` is located at that entry's
//!   indexed key path (`hooks[1].grants.read[0]`). Every path is scanned once; a path with a token
//!   refusal gets no second, path refusal.
//!
//! # Non-responsibilities
//!
//! Turning grants into airsl flags belongs to the extension host.

use crate::types::format::TomlFields;
use crate::types::hooks::policy::Policy;
use crate::types::manifest::ManifestError;
use crate::types::name::BinaryName;
use crate::types::name::EnvName;
use crate::types::report::ListedTwice;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TokenKind;

/// The grants of one Lua hook.
///
/// # Examples
///
/// Grants are read from a hook of a [`PackManifest`](crate::types::PackManifest):
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
/// event = "session.end"
/// run = "hooks/flush.lua"
///
/// [hooks.grants]
/// policy = "pure"
/// read = ["/etc", "{{root}}/data"]
/// env = ["HOME"]
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Lua(lua) = hook.action() else {
///     return Err("a lua hook".into());
/// };
/// assert_eq!(lua.grants().policy(), Policy::Pure);
/// assert_eq!(lua.grants().read_paths(), ["/etc", "{{root}}/data"]);
/// assert!(lua.grants().exec().is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A refusal about one entry names its index:
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
/// event = "session.end"
/// run = "hooks/flush.lua"
///
/// [hooks.grants]
/// read = ["/", "data"]
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/pack.toml: hooks[0].grants.read[1]: \"data\" must be absolute or start with a token"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants {
    policy: Policy,
    read: Vec<String>,
    write: Vec<String>,
    env: Vec<EnvName>,
    exec: Vec<BinaryName>,
}

impl Grants {
    /// Reads the `grants` table behind `fields`, refusing what is wrong in it.
    ///
    /// `binaries` are the pack's `requires.binaries`; every token found is appended to `tokens`.
    pub(in crate::types) fn read(
        fields: &mut TomlFields,
        binaries: &[BinaryName],
        tokens: &mut Vec<LocatedToken>,
    ) -> Self {
        let policy = fields
            .optional_string("policy")
            .and_then(|text| {
                Policy::new(&text)
                    .map_err(|error| fields.refuse("policy", ManifestError::from(error)))
                    .ok()
            })
            .unwrap_or_default();
        let read = read_paths(fields, "read", tokens);
        let write = read_paths(fields, "write", tokens);
        let env = read_env(fields);
        let exec = read_exec(fields, binaries);
        fields.refuse_unknown_keys(&["policy", "read", "write", "env", "exec"]);
        Self {
            policy,
            read,
            write,
            env,
            exec,
        }
    }

    /// The sandbox policy.
    #[must_use]
    pub const fn policy(&self) -> Policy {
        self.policy
    }

    /// The paths the hook may read, as written.
    #[must_use]
    pub fn read_paths(&self) -> &[String] {
        &self.read
    }

    /// The paths the hook may write, as written.
    #[must_use]
    pub fn write_paths(&self) -> &[String] {
        &self.write
    }

    /// The environment variables the hook may read.
    #[must_use]
    pub fn env(&self) -> &[EnvName] {
        &self.env
    }

    /// The binaries the hook may run.
    #[must_use]
    pub fn exec(&self) -> &[BinaryName] {
        &self.exec
    }
}

/// The binary every hook may run without the pack requiring it.
const AIRSL: &str = "airsl";

/// Reads the path list at `key`: each path is scanned once and must be absolute or start with a
/// `{{root}}`, `{{skill_dir}}` or `{{var}}` token. A path with a token refusal is not checked
/// further.
fn read_paths(fields: &mut TomlFields, key: &str, tokens: &mut Vec<LocatedToken>) -> Vec<String> {
    let mut paths = Vec::new();
    for (index, text) in fields.optional_string_list(key).unwrap_or_default() {
        let item_path = fields.item_path(key, index);
        let first = tokens.len();
        if !fields.scan_field(&item_path, &text, tokens) {
            continue;
        }
        if starts_a_path(&text, &tokens[first..]) {
            paths.push(text);
        } else {
            fields.refuse_path(&item_path, ManifestError::GrantPath { text });
        }
    }
    paths
}

/// Whether `text`, holding `found`, is absolute or starts with a token that stands for a path.
fn starts_a_path(text: &str, found: &[LocatedToken]) -> bool {
    text.starts_with('/')
        || found.first().is_some_and(|located| {
            let token = located.token();
            token.span().start == 0
                && matches!(
                    token.kind(),
                    TokenKind::Root | TokenKind::SkillDir | TokenKind::Var
                )
        })
}

/// Reads `env`: each entry must be an environment variable name, listed once.
fn read_env(fields: &mut TomlFields) -> Vec<EnvName> {
    let mut names = Vec::new();
    for (index, text) in fields.optional_string_list("env").unwrap_or_default() {
        let item_path = fields.item_path("env", index);
        match EnvName::new(&text) {
            Ok(name) if names.contains(&name) => {
                fields.refuse_path(&item_path, ManifestError::from(ListedTwice::text(text)));
            }
            Ok(name) => names.push(name),
            Err(error) => fields.refuse_path(&item_path, ManifestError::from(error)),
        }
    }
    names
}

/// Reads `exec`: each entry must be `airsl` or one of the pack's required `binaries`, listed once.
fn read_exec(fields: &mut TomlFields, binaries: &[BinaryName]) -> Vec<BinaryName> {
    let mut granted = Vec::new();
    for (index, text) in fields.optional_string_list("exec").unwrap_or_default() {
        let item_path = fields.item_path("exec", index);
        let allowed = text == AIRSL || binaries.iter().any(|binary| binary.as_str() == text);
        match BinaryName::new(&text) {
            Ok(name) if allowed && granted.contains(&name) => {
                fields.refuse_path(&item_path, ManifestError::from(ListedTwice::text(text)));
            }
            Ok(name) if allowed => granted.push(name),
            _ => fields.refuse_path(&item_path, ManifestError::NotInBinaries { text }),
        }
    }
    granted
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type Read<T> = Result<(T, Vec<LocatedToken>, Vec<String>), Box<dyn std::error::Error>>;

    fn read(text: &str, binaries: &[BinaryName]) -> Read<Grants> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[1].grants", table);
        let mut tokens = Vec::new();
        let grants = Grants::read(&mut fields, binaries, &mut tokens);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((grants, tokens, shown))
    }

    #[test]
    fn grants_parse_a_typical_example() -> TestResult {
        let (grants, tokens, shown) = read(
            "policy = \"confined\"\nread = [\"/\"]\nwrite = [\"{{var tmpdir}}\"]\nenv = [\"HOME\", \"TMPDIR\"]\nexec = []\n",
            &[],
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(grants.policy(), Policy::Confined);
        assert_eq!(grants.read_paths(), ["/"]);
        assert_eq!(grants.write_paths(), ["{{var tmpdir}}"]);
        assert_eq!(
            grants.env().iter().map(EnvName::as_str).collect::<Vec<_>>(),
            ["HOME", "TMPDIR"]
        );
        assert!(grants.exec().is_empty());
        assert_eq!(tokens.len(), 1);
        let token = tokens.first().ok_or("a token")?;
        assert_eq!(token.token().kind(), TokenKind::Var);
        assert_eq!(
            token.location().to_string(),
            "packs/core/pack.toml: hooks[1].grants.write[0]"
        );
        Ok(())
    }

    #[test]
    fn grants_default_to_confined_and_nothing() -> TestResult {
        for grants in [Grants::default(), read("", &[])?.0] {
            assert_eq!(grants.policy(), Policy::Confined);
            assert!(grants.read_paths().is_empty());
            assert!(grants.write_paths().is_empty());
            assert!(grants.env().is_empty());
            assert!(grants.exec().is_empty());
        }
        Ok(())
    }

    #[test]
    fn grants_refuse_each_rule() -> TestResult {
        let cases = [
            (
                "policy = \"trusted\"",
                "hooks[1].grants.policy: \"trusted\" is not allowed for pack hooks",
            ),
            (
                "read = [\"data\"]",
                "hooks[1].grants.read[0]: \"data\" must be absolute or start with a token",
            ),
            (
                "write = [\"data\"]",
                "hooks[1].grants.write[0]: \"data\" must be absolute or start with a token",
            ),
            (
                "read = [\"{{option keep}}/x\"]",
                "hooks[1].grants.read[0]: \"{{option keep}}/x\" must be absolute or start with a token",
            ),
            (
                "read = [\"x/{{root}}\"]",
                "hooks[1].grants.read[0]: \"x/{{root}}\" must be absolute or start with a token",
            ),
            (
                "env = [\"home\"]",
                "hooks[1].grants.env[0]: \"home\" is not an environment variable name",
            ),
            (
                "exec = [\"git\"]",
                "hooks[1].grants.exec[0]: \"git\" is not in requires.binaries",
            ),
            ("net = []", "hooks[1].grants.net: unknown key"),
            (
                "env = [\"HOME\", \"HOME\"]",
                "hooks[1].grants.env[1]: \"HOME\" listed twice",
            ),
            (
                "exec = [\"airsl\", \"airsl\"]",
                "hooks[1].grants.exec[1]: \"airsl\" listed twice",
            ),
            (
                "read = [\"/\", \"data\"]",
                "hooks[1].grants.read[1]: \"data\" must be absolute or start with a token",
            ),
            (
                "env = [\"HOME\", \"home\"]",
                "hooks[1].grants.env[1]: \"home\" is not an environment variable name",
            ),
            (
                "policy = \"foo\"",
                "hooks[1].grants.policy: \"foo\" (expected confined, pure)",
            ),
        ];
        for (text, message) in cases {
            let (_, _, shown) = read(text, &[])?;
            assert_eq!(
                shown,
                [format!("packs/core/pack.toml: {message}")],
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn grants_accept_airsl_without_declaring_it() -> TestResult {
        let (grants, _, shown) = read("exec = [\"airsl\"]", &[])?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(grants.exec().len(), 1);
        Ok(())
    }

    #[test]
    fn grants_accept_declared_binaries() -> TestResult {
        let (grants, _, shown) = read("exec = [\"git\"]", &[BinaryName::new("git")?])?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(grants.exec(), [BinaryName::new("git")?]);
        Ok(())
    }

    #[test]
    fn grants_accept_each_token_kind_that_starts_a_path() -> TestResult {
        let (grants, _, shown) = read(
            "read = [\"{{root}}/data\", \"{{skill_dir discuss}}\", \"{{var home}}\"]",
            &[],
        )?;
        assert!(shown.is_empty(), "{shown:?}");
        assert_eq!(grants.read_paths().len(), 3);
        Ok(())
    }

    #[test]
    fn grants_report_token_errors_by_indexed_key() -> TestResult {
        let (_, _, shown) = read("read = [\"/\", \"{{skil x}}/a\"]", &[])?;
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[1].grants.read[1]: unknown token kind \"skil\""]
        );
        Ok(())
    }
}
