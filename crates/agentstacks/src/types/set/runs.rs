//! The rule over a hook `run` that points into another pack.
//!
//! It is a set-level rule because only the set holds the other pack's installed files; a pack loaded
//! alone cannot tell whether the script it points at exists.
//!
//! # Responsibilities
//!
//! - `check_hook_runs` reads every Lua hook of every loaded pack, in `pack.toml` and in each
//!   `native/<tool>/hooks.toml`, whose `run` names another pack (`{{root <pack>}}/…`). When that
//!   pack is required and loaded, the script must be one of its installed root files: not
//!   excluded, and not under `skills/`, `agents/` or `native/`. Otherwise the hook is refused at
//!   its `run` field.
//! - A `run` into a pack that is not required is refused once, by the token check, for its `root`
//!   token. A `run` into a pack that is not loaded is not looked into.
//!
//! # Non-responsibilities
//!
//! A `run` of the pack's own (pack-relative, `{{root}}/…` or `{{root <this pack>}}/…`) is checked
//! when the pack is loaded, and is skipped here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::types::hooks::HookAction;
use crate::types::hooks::HookRun;
use crate::types::name::PackName;
use crate::types::pack::Pack;
use crate::types::pack::{MANIFEST_FILE, NATIVE_DIR, NATIVE_HOOKS_FILE};
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;
use crate::types::tokens::names_own_pack;
use crate::types::tree::TreePath;

/// A Lua hook's `run`, with the file that declares the hook and the hook's index in it.
struct DeclaredRun<'a> {
    file: PathBuf,
    index: usize,
    run: &'a HookRun,
}

/// Refuses each hook `run` of the loaded `packs` that points at a file another required, loaded
/// pack does not install at its root. `packs` are keyed by their `[packs.*]` name.
pub(crate) fn check_hook_runs(packs: &BTreeMap<PackName, Pack>) -> Vec<Diagnostic> {
    let mut refusals = Vec::new();
    for pack in packs.values() {
        for declared in declared_runs(pack) {
            let Some(target) = declared.run.pack() else {
                continue;
            };
            if names_own_pack(Some(target), pack.manifest().name()) {
                continue;
            }
            if !pack.manifest().requires().packs().contains(target) {
                continue;
            }
            let Some(other) = packs.get(target) else {
                continue;
            };
            let script = TreePath::from_parts(&[declared.run.path().as_str()]);
            if !other.root_files().contains(&script) {
                refusals.push(Diagnostic::new(
                    Location::at_key(declared.file, format!("hooks[{}].run", declared.index)),
                    SetError::RunMissing {
                        run: declared.run.written().to_owned(),
                        pack: target.clone(),
                    },
                ));
            }
        }
    }
    refusals
}

/// The `run` of every Lua hook of `pack`, those of `pack.toml` first.
fn declared_runs(pack: &Pack) -> Vec<DeclaredRun<'_>> {
    let manifest_file = pack.dir().join(MANIFEST_FILE);
    let mut declared: Vec<DeclaredRun<'_>> = pack
        .manifest()
        .hooks()
        .iter()
        .enumerate()
        .filter_map(|(index, hook)| lua_run(hook.action()).map(|run| (index, run)))
        .map(|(index, run)| DeclaredRun {
            file: manifest_file.clone(),
            index,
            run,
        })
        .collect();
    for native in pack.native() {
        let Some(hooks) = native.hooks() else {
            continue;
        };
        let file = pack
            .dir()
            .join(NATIVE_DIR)
            .join(native.tool().as_str())
            .join(NATIVE_HOOKS_FILE);
        declared.extend(
            hooks
                .hooks()
                .iter()
                .enumerate()
                .filter_map(|(index, hook)| lua_run(hook.action()).map(|run| (index, run)))
                .map(|(index, run)| DeclaredRun {
                    file: file.clone(),
                    index,
                    run,
                }),
        );
    }
    declared
}

/// The `run` of `action` when it is a Lua hook.
const fn lua_run(action: &HookAction) -> Option<&HookRun> {
    match action {
        HookAction::Lua(lua) => Some(lua.run()),
        HookAction::Message(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PackSpec, loaded, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const MISSING: &str = "\"{{root core}}/hooks/enforc.lua\" does not exist in core";

    fn lua_hook(run: &str) -> String {
        format!("[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"{run}\"\n")
    }

    fn core() -> PackSpec {
        PackSpec::new("core")
            .airsl(">=0.1.4")
            .file("hooks/enforce.lua", "print(1)\n")
    }

    fn guideline(hooks: &[String]) -> PackSpec {
        PackSpec::new("guideline-rust")
            .airsl(">=0.1.4")
            .requires(&["core"])
            .tables(&hooks.concat())
    }

    fn found(specs: &[PackSpec]) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(shown(&check_hook_runs(&loaded(specs)?)))
    }

    #[test]
    fn other_pack_run_must_exist_in_that_pack() -> TestResult {
        let hooks = [
            lua_hook("{{root core}}/hooks/enforce.lua"),
            lua_hook("{{root core}}/hooks/enforc.lua"),
        ];
        assert_eq!(
            found(&[core(), guideline(&hooks)])?,
            [format!(
                "packs/guideline-rust/pack.toml: hooks[1].run: {MISSING}"
            )]
        );
        let fine = [lua_hook("{{root core}}/hooks/enforce.lua")];
        assert!(found(&[core(), guideline(&fine)])?.is_empty());
        Ok(())
    }

    #[test]
    fn other_pack_run_must_not_be_excluded_there() -> TestResult {
        let core = core()
            .top("exclude = [\"hooks/old.lua\"]\n")
            .file("hooks/old.lua", "print(1)\n");
        let hooks = [lua_hook("{{root core}}/hooks/old.lua")];
        assert_eq!(
            found(&[core, guideline(&hooks)])?,
            [
                "packs/guideline-rust/pack.toml: hooks[0].run: \"{{root core}}/hooks/old.lua\" does not exist in core"
            ]
        );
        Ok(())
    }

    #[test]
    fn other_pack_run_must_be_a_root_file_there() -> TestResult {
        let core = core()
            .skill("discuss", "body")
            .file("skills/discuss/x.lua", "print(1)\n");
        let hooks = [lua_hook("{{root core}}/skills/discuss/x.lua")];
        assert_eq!(
            found(&[core, guideline(&hooks)])?,
            [
                "packs/guideline-rust/pack.toml: hooks[0].run: \"{{root core}}/skills/discuss/x.lua\" does not exist in core"
            ]
        );
        Ok(())
    }

    #[test]
    fn other_pack_run_is_checked_in_native_hooks() -> TestResult {
        let guideline = guideline(&[]).skill("a", "body").file(
            "native/claude/hooks.toml",
            "[[hooks]]\nkind = \"lua\"\nevent = \"PreToolUse\"\nrun = \"{{root core}}/hooks/enforc.lua\"\n",
        );
        assert_eq!(
            found(&[core(), guideline])?,
            [format!(
                "packs/guideline-rust/native/claude/hooks.toml: hooks[0].run: {MISSING}"
            )]
        );
        Ok(())
    }

    #[test]
    fn other_pack_run_into_an_unrequired_pack_is_not_checked_here() -> TestResult {
        let unrequiring = PackSpec::new("guideline-rust")
            .airsl(">=0.1.4")
            .tables(&lua_hook("{{root core}}/hooks/enforc.lua"));
        assert!(found(&[core(), unrequiring])?.is_empty());
        Ok(())
    }

    #[test]
    fn own_pack_run_is_left_to_the_pack() -> TestResult {
        let hooks = [
            lua_hook("hooks/x.lua"),
            lua_hook("{{root}}/hooks/x.lua"),
            lua_hook("{{root guideline-rust}}/hooks/x.lua"),
        ];
        let own = guideline(&hooks).file("hooks/x.lua", "print(1)\n");
        assert!(found(&[own])?.is_empty());
        Ok(())
    }

    #[test]
    fn runs_into_packs_that_are_not_loaded_are_skipped() -> TestResult {
        // `core` failed to load, or `agentstacks.toml` never listed it: either way it is absent.
        let hooks = [lua_hook("{{root core}}/hooks/enforce.lua")];
        assert!(found(&[guideline(&hooks)])?.is_empty());
        Ok(())
    }
}
