//! The two checks on what a pack installs.
//!
//! They are apart from the install set because they judge the result of resolving it, together
//! with the native hooks, and refuse at other places than the resolution does.
//!
//! # Responsibilities
//!
//! - [`check_airsl_declared`] refuses a pack that installs a `.lua` file, anywhere (root, skill or
//!   `native/<tool>/`), and declares no `requires.airsl`, at `pack.toml`, key path
//!   `requires.airsl`. Excluded `.lua` files do not count.
//! - [`check_something_to_install`] refuses a pack with no skill, no agent, no `pack.toml` hook
//!   and no hook in a non-excluded `native/<tool>/hooks.toml`, at the pack directory. A skill or an
//!   agent always counts, because `exclude` cannot remove `SKILL.md` or an agent file. The check
//!   judges only what it is given: a caller that knows an item may have been refused before it was
//!   listed (see `PackLayout::may_hold_unseen_items`) does not call it, so the author is not told
//!   the pack is empty for a reason the refusal already named.
//!
//! # Non-responsibilities
//!
//! Resolving `exclude` belongs to the install set; reading the hooks of each native directory
//! belongs to the pack loader. Whether the airsl range is satisfiable by an installed airsl
//! belongs to the checks over a whole set of packs.

use std::path::Path;

use crate::types::manifest::PackManifest;
use crate::types::pack::error::PackError;
use crate::types::pack::install::set::InstallSet;
use crate::types::pack::layout::{MANIFEST_FILE, PackLayout};
use crate::types::pack::native::NativeDir;
use crate::types::report::Diagnostic;
use crate::types::report::Location;

/// Refuses an installed `.lua` file when the manifest declares no `requires.airsl`.
///
/// # Errors
///
/// The one refusal, located at `requires.airsl` in the `pack.toml` under `dir`.
pub(in crate::types::pack) fn check_airsl_declared(
    dir: &Path,
    manifest: &PackManifest,
    install: &InstallSet,
) -> Result<(), Diagnostic> {
    let has_lua = install.installed().any(|file| file.has_extension("lua"));
    if has_lua && manifest.requires().airsl().is_none() {
        return Err(Diagnostic::new(
            Location::at_key(dir.join(MANIFEST_FILE), "requires.airsl"),
            PackError::AirslRequired,
        ));
    }
    Ok(())
}

/// Refuses a pack that installs no skill, agent or hook.
///
/// # Errors
///
/// The one refusal, located at the pack directory `dir`.
pub(in crate::types::pack) fn check_something_to_install(
    dir: &Path,
    layout: &PackLayout,
    manifest: &PackManifest,
    native: &[NativeDir],
) -> Result<(), Diagnostic> {
    let installs = !layout.skills().is_empty()
        || !layout.agents().is_empty()
        || !manifest.hooks().is_empty()
        || native.iter().any(|dir| dir.hook_count() > 0);
    if installs {
        Ok(())
    } else {
        Err(Diagnostic::new(
            Location::file(dir),
            PackError::NothingToInstall,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::hooks::NativeHooks;
    use crate::types::name::ToolName;
    use crate::types::pack::testing::{DIR, layout, manifest, manifest_with};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const AIRSL: &str = "[requires]\nairsl = \">=0.1.4\"\n";
    const MESSAGE_HOOK: &str =
        "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"t\"\n";

    fn airsl_refusal(
        files: &[&str],
        top: &str,
        rest: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let manifest = manifest_with(top, rest)?;
        let (install, _) = InstallSet::resolve(Path::new(DIR), &layout(files), &manifest, &[]);
        Ok(check_airsl_declared(Path::new(DIR), &manifest, &install)
            .err()
            .map(|d| d.to_string()))
    }

    fn native_dir(hooks_text: Option<&str>) -> Result<NativeDir, Box<dyn Error>> {
        let hooks = hooks_text
            .map(|text| {
                NativeHooks::parse(
                    Path::new("native/claude/hooks.toml"),
                    text,
                    &crate::types::manifest::Requires::default(),
                )
            })
            .transpose()
            .map_err(|d| format!("{:?}", d.into_sorted()))?;
        Ok(NativeDir::new(ToolName::new("claude")?, Vec::new(), hooks))
    }

    fn nothing_refusal(
        files: &[&str],
        rest: &str,
        native: &[NativeDir],
    ) -> Result<Option<String>, Box<dyn Error>> {
        let manifest = manifest(rest)?;
        Ok(
            check_something_to_install(Path::new(DIR), &layout(files), &manifest, native)
                .err()
                .map(|d| d.to_string()),
        )
    }

    #[test]
    fn installed_lua_requires_requires_airsl() -> TestResult {
        assert_eq!(
            airsl_refusal(&["pack.toml", "scripts/x.lua"], "", "")?,
            Some(format!(
                "{DIR}/pack.toml: requires.airsl: required (the pack contains .lua files)"
            ))
        );
        Ok(())
    }

    #[test]
    fn lua_in_a_skill_or_native_dir_counts() -> TestResult {
        for only_lua in ["skills/a/scripts/x.lua", "native/claude/x.lua"] {
            assert_eq!(
                airsl_refusal(&["pack.toml", only_lua], "", "")?,
                Some(format!(
                    "{DIR}/pack.toml: requires.airsl: required (the pack contains .lua files)"
                )),
                "{only_lua}"
            );
        }
        Ok(())
    }

    #[test]
    fn excluded_lua_needs_no_airsl() -> TestResult {
        assert_eq!(
            airsl_refusal(
                &["pack.toml", "scripts/x.lua"],
                "exclude = [\"**/*.lua\"]",
                ""
            )?,
            None
        );
        assert_eq!(
            airsl_refusal(&["pack.toml", "scripts/x.lua"], "", AIRSL)?,
            None
        );
        Ok(())
    }

    #[test]
    fn pack_with_only_root_files_has_nothing_to_install() -> TestResult {
        assert_eq!(
            nothing_refusal(&["pack.toml", "scripts/x.md"], "", &[])?,
            Some(format!("{DIR}: pack declares nothing to install"))
        );
        Ok(())
    }

    #[test]
    fn each_installable_kind_counts() -> TestResult {
        let native_hook = native_dir(Some(
            "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"t\"\n",
        ))?;
        assert_eq!(
            nothing_refusal(&["pack.toml", "skills/a/SKILL.md"], "", &[])?,
            None
        );
        assert_eq!(
            nothing_refusal(&["agents/a.md", "pack.toml"], "", &[])?,
            None
        );
        assert_eq!(nothing_refusal(&["pack.toml"], MESSAGE_HOOK, &[])?, None);
        assert_eq!(nothing_refusal(&["pack.toml"], "", &[native_hook])?, None);
        Ok(())
    }

    #[test]
    fn excluded_or_empty_native_hooks_do_not_count() -> TestResult {
        let expected = Some(format!("{DIR}: pack declares nothing to install"));
        assert_eq!(
            nothing_refusal(&["pack.toml"], "", &[native_dir(None)?])?,
            expected
        );
        assert_eq!(
            nothing_refusal(&["pack.toml"], "", &[native_dir(Some(""))?])?,
            expected
        );
        Ok(())
    }
}
