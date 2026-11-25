//! The module name a hook is required by.
//!
//! # Responsibilities
//!
//! - [`HookModule`] turns a hook's `run` into the dotted name the runtime passes to `require`.
//!
//! # Non-responsibilities
//!
//! Whether every segment can be named was settled when the pack was loaded, so building a module
//! cannot fail.

use std::fmt;

use crate::types::hooks::HookRun;
use crate::types::name::PackName;

/// The dotted module name of a hook's script, resolved from the root of the extension.
///
/// The pack's files sit under `<root>/<pack>/`, so the module is the pack's name followed by each
/// segment of the script's path, with `.lua` removed from the last.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::protocol::HookModule;
/// use agentstacks::types::{HookAction, PackManifest, PackName};
///
/// let text = r#"
/// [pack]
/// name = "team-lint"
/// version = "1.0.0"
/// api = 1
/// description = "Lint rules"
///
/// [[hooks]]
/// kind = "lua"
/// event = "session.start"
/// run = "{{root core}}/hooks/enforce.lua"
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/team-lint/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// let hook = manifest.hooks().first().ok_or("one hook")?;
/// let HookAction::Lua(lua) = hook.action() else {
///     return Err("a lua hook".into());
/// };
/// let module = HookModule::new(&PackName::new("team-lint")?, lua.run());
/// assert_eq!(module.as_str(), "core.hooks.enforce");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookModule(String);

impl HookModule {
    /// The module of `run`, a script of the pack `owner` declares the hook in.
    #[must_use]
    pub fn new(owner: &PackName, run: &HookRun) -> Self {
        let pack = run.pack().unwrap_or(owner);
        let path = run.path().as_str();
        let script = path.strip_suffix(".lua").unwrap_or(path);
        let segments = script.split('/').collect::<Vec<_>>().join(".");
        Self(format!("{pack}.{segments}"))
    }

    /// The module name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HookModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::{HookAction, PackManifest};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn module_of(pack: &str, run: &str) -> Result<String, Box<dyn std::error::Error>> {
        let text = format!(
            "[pack]\nname = \"{pack}\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n\n\
             [[hooks]]\nkind = \"lua\"\nevent = \"session.start\"\nrun = \"{run}\"\n"
        );
        let manifest = PackManifest::parse(Path::new("packs/p/pack.toml"), &text)
            .map_err(|found| format!("{found:?}"))?;
        let hook = manifest.hooks().first().ok_or("one hook")?;
        let HookAction::Lua(lua) = hook.action() else {
            return Err("a lua hook".into());
        };
        Ok(HookModule::new(&PackName::new(pack)?, lua.run()).to_string())
    }

    #[test]
    fn module_from_another_packs_root() -> TestResult {
        assert_eq!(
            module_of("team-lint", "{{root core}}/hooks/enforce.lua")?,
            "core.hooks.enforce"
        );
        Ok(())
    }

    #[test]
    fn module_from_own_pack_relative() -> TestResult {
        assert_eq!(
            module_of("team-lint", "hooks/my-file.lua")?,
            "team-lint.hooks.my-file"
        );
        Ok(())
    }

    #[test]
    fn module_from_own_root_token() -> TestResult {
        assert_eq!(
            module_of("core", "{{root}}/hooks/a/b.lua")?,
            "core.hooks.a.b"
        );
        Ok(())
    }
}
