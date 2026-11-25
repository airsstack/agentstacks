//! The command line a tool runs for a Lua hook.
//!
//! # Responsibilities
//!
//! - [`HookCommand`] holds a hook's already-rendered values and builds the exact `airsl` argument
//!   vector that runs the hook through the runtime.
//!
//! # Non-responsibilities
//!
//! Rendering tokens in the arguments and grant paths, quoting the vector into the string a tool
//! stores, and choosing the working directory (the project directory, so relative paths resolve)
//! belong to the tool's extension. Values pass through unchanged.

use crate::types::name::{BinaryName, EnvName};
use crate::types::project::SubPath;
use crate::types::protocol::{Blocking, HookKind, HookModule, RUNTIME_FILE};

/// What it takes to run one Lua hook through the runtime.
///
/// # Examples
///
/// ```
/// use agentstacks::types::SubPath;
/// use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, HookModule};
/// # use std::path::Path;
/// # use agentstacks::types::{HookAction, PackManifest, PackName};
/// # let text = "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n\n\
/// #     [[hooks]]\nkind = \"lua\"\nevent = \"session.start\"\nrun = \"hooks/enforce.lua\"\n";
/// # let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
/// #     .map_err(|found| format!("{found:?}"))?;
/// # let hook = manifest.hooks().first().ok_or("one hook")?;
/// # let HookAction::Lua(lua) = hook.action() else { return Err("a lua hook".into()) };
/// let module = HookModule::new(&PackName::new("core")?, lua.run());
///
/// let command = HookCommand::new(
///     SubPath::new(".claude/agentstacks")?,
///     module,
///     Blocking::MayNot,
///     HookKind::Neutral,
/// )
/// .with_write(vec!["$TMPDIR".to_owned()]);
///
/// assert_eq!(
///     command.argv().join(" "),
///     "airsl run --fail-open --policy confined --allow-write $TMPDIR \
///      .claude/agentstacks/runtime.lua core.hooks.enforce 0 neutral"
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookCommand {
    root: SubPath,
    module: HookModule,
    blocking: Blocking,
    kind: HookKind,
    args: Vec<String>,
    read: Vec<String>,
    write: Vec<String>,
    env: Vec<EnvName>,
    exec: Vec<BinaryName>,
}

impl HookCommand {
    /// A command for the hook `module` under the extension root `root`, with no arguments and no
    /// grants.
    #[must_use]
    pub const fn new(
        root: SubPath,
        module: HookModule,
        blocking: Blocking,
        kind: HookKind,
    ) -> Self {
        Self {
            root,
            module,
            blocking,
            kind,
            args: Vec::new(),
            read: Vec::new(),
            write: Vec::new(),
            env: Vec::new(),
            exec: Vec::new(),
        }
    }

    /// Sets the hook's own arguments, already rendered.
    #[must_use]
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Sets the paths the hook may read, already rendered.
    #[must_use]
    pub fn with_read(mut self, read: Vec<String>) -> Self {
        self.read = read;
        self
    }

    /// Sets the paths the hook may write, already rendered.
    #[must_use]
    pub fn with_write(mut self, write: Vec<String>) -> Self {
        self.write = write;
        self
    }

    /// Sets the environment variables the hook may read.
    #[must_use]
    pub fn with_env(mut self, env: Vec<EnvName>) -> Self {
        self.env = env;
        self
    }

    /// Sets the binaries the hook may run.
    #[must_use]
    pub fn with_exec(mut self, exec: Vec<BinaryName>) -> Self {
        self.exec = exec;
        self
    }

    /// The argument vector, program first.
    ///
    /// The runtime is always run fail-open under the `confined` policy. Grants follow as one flag
    /// per entry: reads, then writes, then environment variables, then binaries, each in the order
    /// given. The runtime's path (`<root>/runtime.lua`, or `runtime.lua` alone for a root of `.`),
    /// the module, the blocking argument and the kind argument come next, and the hook's arguments
    /// last.
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        let mut argv: Vec<String> = ["airsl", "run", "--fail-open", "--policy", "confined"]
            .map(str::to_owned)
            .into();
        for path in &self.read {
            argv.extend(["--allow-read".to_owned(), path.clone()]);
        }
        for path in &self.write {
            argv.extend(["--allow-write".to_owned(), path.clone()]);
        }
        for name in &self.env {
            argv.extend(["--allow-env".to_owned(), name.as_str().to_owned()]);
        }
        for name in &self.exec {
            argv.extend(["--allow-exec".to_owned(), name.as_str().to_owned()]);
        }
        let root = self.root.normalized();
        argv.push(if root.is_empty() {
            RUNTIME_FILE.to_owned()
        } else {
            format!("{root}/{RUNTIME_FILE}")
        });
        argv.push(self.module.as_str().to_owned());
        argv.push(self.blocking.as_arg().to_owned());
        argv.push(self.kind.as_arg().to_owned());
        argv.extend(self.args.iter().cloned());
        argv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::PackName;
    use crate::types::{HookAction, PackManifest};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn module(name: &str) -> Result<HookModule, Box<dyn std::error::Error>> {
        let text = format!(
            "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n\n\
             [[hooks]]\nkind = \"lua\"\nevent = \"session.start\"\nrun = \"hooks/{name}.lua\"\n"
        );
        let manifest = PackManifest::parse(std::path::Path::new("packs/core/pack.toml"), &text)
            .map_err(|found| format!("{found:?}"))?;
        let hook = manifest.hooks().first().ok_or("one hook")?;
        let HookAction::Lua(lua) = hook.action() else {
            return Err("a lua hook".into());
        };
        Ok(HookModule::new(&PackName::new("core")?, lua.run()))
    }

    fn command(root: &str) -> Result<HookCommand, Box<dyn std::error::Error>> {
        Ok(HookCommand::new(
            SubPath::new(root)?,
            module("enforce")?,
            Blocking::MayNot,
            HookKind::Neutral,
        ))
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn argv_without_grants() -> TestResult {
        assert_eq!(
            command(".claude/agentstacks")?.argv(),
            strings(&[
                "airsl",
                "run",
                "--fail-open",
                "--policy",
                "confined",
                ".claude/agentstacks/runtime.lua",
                "core.hooks.enforce",
                "0",
                "neutral",
            ])
        );
        Ok(())
    }

    #[test]
    fn argv_grants_in_order() -> TestResult {
        let argv = command(".tool")?
            .with_read(strings(&["a", "b"]))
            .with_write(strings(&["w"]))
            .with_env(vec![EnvName::new("HOME")?, EnvName::new("PATH")?])
            .with_exec(vec![BinaryName::new("git")?])
            .argv();
        assert_eq!(
            argv,
            strings(&[
                "airsl",
                "run",
                "--fail-open",
                "--policy",
                "confined",
                "--allow-read",
                "a",
                "--allow-read",
                "b",
                "--allow-write",
                "w",
                "--allow-env",
                "HOME",
                "--allow-env",
                "PATH",
                "--allow-exec",
                "git",
                ".tool/runtime.lua",
                "core.hooks.enforce",
                "0",
                "neutral",
            ])
        );
        Ok(())
    }

    #[test]
    fn argv_appends_args_after_kind() -> TestResult {
        let argv = command(".tool")?
            .with_args(strings(&[
                "--manifest",
                ".claude/agentstacks/core/enforcement.json",
            ]))
            .argv();
        assert_eq!(
            argv[argv.len() - 3..],
            strings(&[
                "neutral",
                "--manifest",
                ".claude/agentstacks/core/enforcement.json",
            ])[..]
        );
        Ok(())
    }

    #[test]
    fn argv_may_block_and_native() -> TestResult {
        let argv = HookCommand::new(
            SubPath::new(".tool")?,
            module("enforce")?,
            Blocking::May,
            HookKind::Native,
        )
        .argv();
        assert_eq!(argv[argv.len() - 2..], strings(&["1", "native"])[..]);
        Ok(())
    }

    #[test]
    fn argv_normalises_the_root() -> TestResult {
        let argv = command("./.tool//agentstacks")?.argv();
        assert_eq!(argv[5], ".tool/agentstacks/runtime.lua");
        Ok(())
    }

    #[test]
    fn argv_dot_root_names_the_runtime_alone() -> TestResult {
        let argv = command(".")?.argv();
        assert_eq!(argv[5], "runtime.lua");
        Ok(())
    }

    #[test]
    fn argv_keeps_var_values_as_text() -> TestResult {
        let argv = command(".tool")?.with_write(strings(&["$TMPDIR"])).argv();
        assert_eq!(argv[5..7], strings(&["--allow-write", "$TMPDIR"])[..]);
        Ok(())
    }
}
