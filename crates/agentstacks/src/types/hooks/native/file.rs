//! The hooks a pack declares for one tool in `native/<tool>/hooks.toml`.
//!
//! It is separate from `NativeHook` because it owns the file-level rules (syntax, the single `hooks`
//! key, every refusal returned at once), where `NativeHook` reads one item.
//!
//! # Responsibilities
//!
//! - [`NativeHooks::parse`] reads the whole file and returns its hooks and the tokens found in
//!   their fields, or every refusal of the file at once.
//! - A syntax error stops only this file and is its only diagnostic. The only top-level key is
//!   `hooks`; empty or comment-only text means no hooks.
//! - Each hook is read by `NativeHook::read` against the `requires.binaries` of the pack's
//!   manifest, so `exec` is checked against what the pack declares.
//!
//! # Non-responsibilities
//!
//! Which `native/<tool>` directory the file sits in, and whether the tool exists, belong to the
//! pack loader.

use std::path::Path;

use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::hooks::NativeHook;
use crate::types::manifest::Requires;
use crate::types::report::Diagnostics;
use crate::types::tokens::LocatedToken;

/// The parsed `native/<tool>/hooks.toml` of a pack.
///
/// # Examples
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
/// text = "Run the checks before you stop."
/// "#;
/// let file = Path::new("packs/core/native/claude/hooks.toml");
/// let hooks = NativeHooks::parse(file, text, &Requires::default())
///     .map_err(|found| format!("{found:?}"))?;
/// assert_eq!(hooks.hooks().len(), 1);
/// assert_eq!(hooks.hooks().first().map(|hook| hook.event()), Some("Stop"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct NativeHooks {
    hooks: Vec<NativeHook>,
    tokens: Vec<LocatedToken>,
}

impl NativeHooks {
    /// Parses `text`, the content of the `hooks.toml` at `file`, against the pack's `requires`.
    ///
    /// # Errors
    ///
    /// Returns every refusal of the file when any field is refused; the [`Diagnostics`] hand them
    /// back sorted.
    ///
    /// # Examples
    ///
    /// A refusal is located at the key path of the entry it is about:
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{NativeHooks, Requires};
    ///
    /// let text = "[[hooks]]\nkind = \"lua\"\nevent = \"E\"\nrun = \"hooks/x.lua\"\n[hooks.grants]\nexec = [\"git\"]\n";
    /// let file = Path::new("packs/core/native/claude/hooks.toml");
    /// let refused = NativeHooks::parse(file, text, &Requires::default())
    ///     .err()
    ///     .ok_or("expected a refusal")?;
    /// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
    /// assert_eq!(
    ///     lines,
    ///     ["packs/core/native/claude/hooks.toml: hooks[0].grants.exec[0]: \"git\" is not in requires.binaries"]
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(file: &Path, text: &str, requires: &Requires) -> Result<Self, Diagnostics> {
        let table = match parse_toml_document(file, text) {
            Ok(table) => table,
            Err(syntax) => {
                let mut diagnostics = Diagnostics::new();
                diagnostics.push(syntax);
                return Err(diagnostics);
            }
        };
        let mut root = TomlFields::new(file, "", table);
        root.refuse_unknown_keys(&["hooks"]);
        let mut hooks = Vec::new();
        let mut tokens = Vec::new();
        for mut item in root.table_array("hooks") {
            hooks.extend(NativeHook::read(
                &mut item,
                requires.binaries(),
                &mut tokens,
            ));
            root.absorb(item);
        }
        let diagnostics = root.into_diagnostics();
        if diagnostics.is_empty() {
            Ok(Self { hooks, tokens })
        } else {
            Err(diagnostics)
        }
    }

    /// The hooks, in file order.
    #[must_use]
    pub fn hooks(&self) -> &[NativeHook] {
        &self.hooks
    }

    /// Every token found in the hook fields, located at its key path.
    #[must_use]
    pub fn tokens(&self) -> &[LocatedToken] {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "packs/core/native/claude/hooks.toml";

    fn shown(diagnostics: Diagnostics) -> Vec<String> {
        diagnostics
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn parse(text: &str) -> Result<NativeHooks, Vec<String>> {
        NativeHooks::parse(Path::new(FILE), text, &Requires::default()).map_err(shown)
    }

    #[test]
    fn native_hooks_parse_a_file_of_hooks() -> TestResult {
        let hooks = parse(
            r#"
[[hooks]]
kind  = "message"
event = "Stop"
text  = "bye"

[[hooks]]
kind  = "lua"
event = "PreToolUse"
match = ["SubagentHandback"]
run   = "hooks/handoff-check.lua"
args  = ["{{root}}/data"]
"#,
        )
        .map_err(|e| e.join("; "))?;
        assert_eq!(hooks.hooks().len(), 2);
        let located: Vec<String> = hooks
            .tokens()
            .iter()
            .map(|t| format!("{} {}", t.token(), t.location()))
            .collect();
        assert_eq!(
            located,
            ["{{root}} packs/core/native/claude/hooks.toml: hooks[1].args[0]"]
        );
        Ok(())
    }

    #[test]
    fn native_hooks_read_an_empty_file_as_no_hooks() -> TestResult {
        for text in ["", "# nothing here\n"] {
            assert!(
                parse(text).map_err(|e| e.join("; "))?.hooks().is_empty(),
                "{text:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn native_hooks_refuse_unknown_top_level_keys() {
        assert_eq!(
            parse("tools = 1").err(),
            Some(vec![format!("{FILE}: tools: unknown key")])
        );
    }

    #[test]
    fn native_hooks_report_a_syntax_error_by_line() {
        assert_eq!(
            parse("a = [1,").err(),
            Some(vec![format!("{FILE}:1: unclosed array, expected `]`")])
        );
    }

    #[test]
    fn native_hooks_check_exec_against_the_manifest_binaries() -> TestResult {
        let text = "[[hooks]]\nkind = \"lua\"\nevent = \"E\"\nrun = \"hooks/x.lua\"\n[hooks.grants]\nexec = [\"git\"]\n";
        assert_eq!(
            parse(text).err(),
            Some(vec![format!(
                "{FILE}: hooks[0].grants.exec[0]: \"git\" is not in requires.binaries"
            )])
        );
        let manifest = crate::types::PackManifest::parse(
            Path::new("packs/core/pack.toml"),
            "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n[requires]\nbinaries = [\"git\"]\n",
        )
        .map_err(|d| shown(d).join("; "))?;
        let hooks = NativeHooks::parse(Path::new(FILE), text, manifest.requires())
            .map_err(|d| shown(d).join("; "))?;
        assert_eq!(hooks.hooks().len(), 1);
        Ok(())
    }
}
