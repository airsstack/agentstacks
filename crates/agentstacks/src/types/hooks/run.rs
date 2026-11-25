//! The script a Lua hook runs.
//!
//! It is separate from `LuaHook` because the three allowed `run` forms and their token handling are
//! a self-contained parse that yields three facts: as written, the pack named and the pack-relative
//! path.
//!
//! # Responsibilities
//!
//! - [`HookRun`] holds a hook's `run` as written, the pack it names (none for the pack's own
//!   files) and the pack-relative path of the script.
//! - `HookRun::read` reads the `run` field: it is required, ends in `.lua`, and is written in
//!   one of three forms: pack-relative (`hooks/x.lua`), `{{root}}/hooks/x.lua`, or
//!   `{{root core}}/hooks/x.lua`. Absolute paths, `..`, `{{skill_dir ...}}`, `{{var ...}}` and a
//!   token anywhere but the start are refused.
//! - Every `/` segment of the pack-relative path, with `.lua` removed from the last, may use only
//!   letters, digits, `_` and `-`: the hook runtime requires the script as a dotted module name
//!   (`core.hooks.enforce`), so a `.` or any other character in a segment cannot be named. The
//!   first bad segment is refused, quoting the `run` as written; only a `.lua` run is checked.
//! - The field is scanned once; its tokens are kept located at the field's key path, and a token
//!   refusal is reported without a second, form refusal on top of it.
//!
//! # Non-responsibilities
//!
//! Whether the script exists, in this pack or in another, belongs to the checks over a loaded pack
//! and over a whole set of packs; a `pack` equal to the pack's own name is treated as its own
//! file there.

use std::path::Path;

use crate::types::format::TomlFields;
use crate::types::manifest::ManifestError;
use crate::types::manifest::PackPath;
use crate::types::name::PackName;
use crate::types::report::NotLuaFile;
use crate::types::tokens::LocatedToken;
use crate::types::tokens::TokenTarget;

/// A hook's script, as written and as understood.
///
/// # Examples
///
/// A `run` keeps its script as written, the pack it comes from and its pack-relative path:
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
/// api = 1
/// description = "Shared skills"
///
/// [[hooks]]
/// kind = "lua"
/// event = "session.start"
/// run = "{{root core}}/hooks/enforce.lua"
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Lua(lua) = hook.action() else {
///     return Err("a lua hook".into());
/// };
/// let run = lua.run();
/// assert_eq!(run.written(), "{{root core}}/hooks/enforce.lua");
/// assert_eq!(run.pack().map(|pack| pack.as_str()), Some("core"));
/// assert_eq!(run.path().as_str(), "hooks/enforce.lua");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A script that is not a Lua file is refused:
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
/// api = 1
/// description = "Shared skills"
///
/// [[hooks]]
/// kind = "lua"
/// event = "session.start"
/// run = "hooks/x.sh"
/// "#;
/// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .err()
///     .ok_or("expected a refusal")?;
/// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
/// assert_eq!(
///     lines,
///     ["packs/core/pack.toml: hooks[0].run: \"hooks/x.sh\" is not a .lua file"]
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRun {
    written: String,
    pack: Option<PackName>,
    path: PackPath,
}

impl HookRun {
    /// Reads the `run` field of the hook behind `fields`, refusing what is wrong in it.
    ///
    /// Every token found in the field is appended to `tokens`.
    pub(in crate::types) fn read(
        fields: &mut TomlFields,
        tokens: &mut Vec<LocatedToken>,
    ) -> Option<Self> {
        let written = fields.required_text("run")?;
        let key_path = fields.key_path("run");
        let first = tokens.len();
        if !fields.scan_field(&key_path, &written, tokens) {
            return None;
        }
        let is_lua = Path::new(&written)
            .extension()
            .is_some_and(|extension| extension == "lua");
        if !is_lua {
            fields.refuse(
                "run",
                ManifestError::from(NotLuaFile::new(written.as_str())),
            );
        }
        let Some((pack, path)) = split_form(&written, &tokens[first..]) else {
            fields.refuse("run", ManifestError::RunForm);
            return None;
        };
        if !is_lua {
            return None;
        }
        if let Some(segment) = first_unrequirable_segment(&path) {
            fields.refuse(
                "run",
                ManifestError::RunSegment {
                    run: written,
                    segment,
                },
            );
            return None;
        }
        Some(Self {
            written,
            pack,
            path,
        })
    }

    /// The `run` text as written.
    #[must_use]
    pub fn written(&self) -> &str {
        &self.written
    }

    /// The pack named by a `{{root <pack>}}` token; `None` for the hook's own pack.
    #[must_use]
    pub const fn pack(&self) -> Option<&PackName> {
        self.pack.as_ref()
    }

    /// The script's path relative to its pack's root.
    #[must_use]
    pub const fn path(&self) -> &PackPath {
        &self.path
    }
}

/// The first segment of the Lua script at `path`, with `.lua` removed from the last, that is not
/// made of ASCII letters, digits, `_` and `-` alone.
fn first_unrequirable_segment(path: &PackPath) -> Option<String> {
    let text = path.as_str();
    let module = text.strip_suffix(".lua").unwrap_or(text);
    module
        .split('/')
        .find(|segment| {
            segment.is_empty()
                || !segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        })
        .map(str::to_owned)
}

/// Splits `text` into the pack it names and its pack-relative path, when it is written in one of
/// the three allowed forms. `found` are the tokens the text holds.
fn split_form(text: &str, found: &[LocatedToken]) -> Option<(Option<PackName>, PackPath)> {
    if !text.contains("{{") {
        return PackPath::new(text).ok().map(|path| (None, path));
    }
    let token = found.first()?.token();
    let TokenTarget::Root(pack) = token.target() else {
        return None;
    };
    let span = token.span();
    if span.start != 0 {
        return None;
    }
    let rest = text[span.end..].strip_prefix('/')?;
    if rest.contains("{{") {
        return None;
    }
    PackPath::new(rest).ok().map(|path| (pack.clone(), path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenKind;
    use crate::types::format::parse_toml_document;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type Read<T> = Result<(T, Vec<LocatedToken>, Vec<String>), Box<dyn std::error::Error>>;

    const RUN_FORM: &str = "packs/core/pack.toml: hooks[1].run: must be pack-relative, {{root}}/… or {{root <pack>}}/…";

    fn read(text: &str) -> Read<Option<HookRun>> {
        let file = Path::new("packs/core/pack.toml");
        let table = parse_toml_document(file, text).map_err(|d| d.to_string())?;
        let mut fields = TomlFields::new(file, "hooks[1]", table);
        let mut tokens = Vec::new();
        let run = HookRun::read(&mut fields, &mut tokens);
        let shown = fields
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((run, tokens, shown))
    }

    fn run_of(value: &str) -> String {
        format!("run = '{value}'\n")
    }

    #[test]
    fn hook_run_accepts_the_three_forms() -> TestResult {
        for value in [
            "hooks/x.lua",
            "{{root}}/hooks/x.lua",
            "{{ root }}/hooks/x.lua",
        ] {
            let (run, _, shown) = read(&run_of(value))?;
            assert!(shown.is_empty(), "{value}: {shown:?}");
            let run = run.ok_or("a run")?;
            assert_eq!(run.pack(), None, "{value}");
            assert_eq!(run.path().as_str(), "hooks/x.lua", "{value}");
            assert_eq!(run.written(), value);
        }
        let (run, tokens, shown) = read(&run_of("{{root core}}/hooks/enforce.lua"))?;
        assert!(shown.is_empty(), "{shown:?}");
        let run = run.ok_or("a run")?;
        assert_eq!(run.pack(), Some(&PackName::new("core")?));
        assert_eq!(run.path().as_str(), "hooks/enforce.lua");
        assert_eq!(run.written(), "{{root core}}/hooks/enforce.lua");
        assert_eq!(tokens.len(), 1);
        let token = tokens.first().ok_or("a token")?;
        assert_eq!(token.token().kind(), TokenKind::Root);
        assert_eq!(
            token.location().to_string(),
            "packs/core/pack.toml: hooks[1].run"
        );
        Ok(())
    }

    #[test]
    fn hook_run_refuses_a_missing_run() -> TestResult {
        let (run, _, shown) = read("")?;
        assert!(run.is_none());
        assert_eq!(shown, ["packs/core/pack.toml: hooks[1].run: required"]);
        Ok(())
    }

    #[test]
    fn hook_run_refuses_a_non_lua_file() -> TestResult {
        let (run, _, shown) = read(&run_of("x.sh"))?;
        assert!(run.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[1].run: \"x.sh\" is not a .lua file"]
        );
        Ok(())
    }

    #[test]
    fn hook_run_refuses_other_forms() -> TestResult {
        for value in [
            "/abs/x.lua",
            "../x.lua",
            "{{skill_dir discuss}}/x.lua",
            "{{var tmpdir}}/x.lua",
            "hooks/{{root}}/x.lua",
            "{{root}}x.lua",
            "{{root}}/{{var tmpdir}}/x.lua",
        ] {
            let (run, _, shown) = read(&run_of(value))?;
            assert!(run.is_none(), "{value}");
            assert_eq!(shown, [RUN_FORM], "{value}");
        }
        Ok(())
    }

    #[test]
    fn hook_run_reports_a_token_error_without_a_form_error() -> TestResult {
        let (run, _, shown) = read(&run_of("{{root Core}}/x.lua"))?;
        assert!(run.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[1].run: \"Core\" is not a valid name"]
        );
        Ok(())
    }

    fn segment_refusal(run: &str, segment: &str) -> String {
        format!(
            "packs/core/pack.toml: hooks[1].run: \"{run}\" segment \"{segment}\" may use only letters, digits, _ and -"
        )
    }

    #[test]
    fn hook_run_refuses_a_segment_that_cannot_be_required() -> TestResult {
        let (run, _, shown) = read(&run_of("hooks/my.hook.lua"))?;
        assert!(run.is_none());
        assert_eq!(shown, [segment_refusal("hooks/my.hook.lua", "my.hook")]);
        Ok(())
    }

    #[test]
    fn hook_run_quotes_a_token_form_as_written() -> TestResult {
        let run_text = "{{root core}}/hooks/my.hook.lua";
        let (run, _, shown) = read(&run_of(run_text))?;
        assert!(run.is_none());
        assert_eq!(shown, [segment_refusal(run_text, "my.hook")]);
        Ok(())
    }

    #[test]
    fn hook_run_reports_only_the_first_bad_segment() -> TestResult {
        let (run, _, shown) = read(&run_of("a.b/c.d.lua"))?;
        assert!(run.is_none());
        assert_eq!(shown, [segment_refusal("a.b/c.d.lua", "a.b")]);
        Ok(())
    }

    #[test]
    fn hook_run_checks_segments_of_lua_runs_only() -> TestResult {
        let (run, _, shown) = read(&run_of("hooks/my.hook.sh"))?;
        assert!(run.is_none());
        assert_eq!(
            shown,
            ["packs/core/pack.toml: hooks[1].run: \"hooks/my.hook.sh\" is not a .lua file"]
        );
        Ok(())
    }

    #[test]
    fn hook_run_accepts_letters_digits_underscore_and_hyphen() -> TestResult {
        for value in [
            "hooks/handoff-check_2.lua",
            "Hooks/A9.lua",
            "{{root core}}/hooks/enforce.lua",
        ] {
            let (run, _, shown) = read(&run_of(value))?;
            assert!(shown.is_empty(), "{value}: {shown:?}");
            assert!(run.is_some(), "{value}");
        }
        Ok(())
    }
}
