//! One run of a hook module and the comparison with its recorded output.

use std::error::Error;
use std::fs;
use std::process::Output;

use agentstacks::types::SubPath;
use agentstacks::types::protocol::{Blocking, HookCommand, HookKind, RUNTIME_LUA};
use tempfile::TempDir;

use crate::common::hook::{module, run_piped};
use crate::common::tree::copy_tree;
use crate::common::{fixture, skip_without_airsl};

/// One run of a hook module under the shipped runtime and the expected translator.
pub(crate) struct Run<'a> {
    /// The hook module under `payloads/hooks`.
    hook: &'a str,
    /// The recorded payload under `payloads/input`.
    payload: &'a str,
    blocking: Blocking,
    /// Neutral unless a test binary sets it: only the binaries that run native hooks do.
    pub(crate) kind: HookKind,
}

impl<'a> Run<'a> {
    /// A neutral hook that may not block.
    pub(crate) const fn new(hook: &'a str, payload: &'a str) -> Self {
        Self {
            hook,
            payload,
            blocking: Blocking::MayNot,
            kind: HookKind::Neutral,
        }
    }

    /// The same run for a hook declared `may_block`.
    pub(crate) const fn may_block(mut self) -> Self {
        self.blocking = Blocking::May;
        self
    }
}

/// A project directory holding the translator the `full` fixture of `tool` expects under `root`,
/// the shipped runtime and the hook modules.
fn translator_project(tool: &str, root: &str) -> Result<TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let at = dir.path().join(root);
    fs::create_dir_all(at.join("core/hooks"))?;
    fs::write(at.join("runtime.lua"), RUNTIME_LUA)?;
    fs::copy(
        fixture(tool, "full")
            .join("expected")
            .join(root)
            .join("translate.lua"),
        at.join("translate.lua"),
    )?;
    copy_tree(
        &fixture(tool, "payloads").join("hooks"),
        &at.join("core/hooks"),
        None,
    )?;
    Ok(dir)
}

/// Runs `case` with the argument vector `HookCommand` builds and the payload on stdin, from a
/// project directory whose extension root is `root`, or returns `None` when `airsl` cannot run and
/// is not required.
fn run_hook(tool: &str, root: &str, case: &Run<'_>) -> Result<Option<Output>, Box<dyn Error>> {
    if skip_without_airsl()? {
        return Ok(None);
    }
    let dir = translator_project(tool, root)?;
    let argv = HookCommand::new(
        SubPath::new(root)?,
        module(case.hook)?,
        case.blocking,
        case.kind,
    )
    .argv();
    let payload = fs::read(fixture(tool, "payloads/input").join(format!("{}.json", case.payload)))?;
    Ok(Some(run_piped(&argv, dir.path(), &payload)?))
}

/// The expected stdout under `payloads/expected/<name>.out` or stderr under `<name>.err` of `tool`.
fn recorded(tool: &str, name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(
        fixture(tool, "payloads/expected").join(name),
    )?)
}

/// Runs `case` for `tool` and compares stdout with `<out>.out` and stderr with `<err>.err`, or
/// with nothing when `err` is `None`.
pub(crate) fn check(
    tool: &str,
    root: &str,
    case: &Run<'_>,
    out: &str,
    err: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let Some(output) = run_hook(tool, root, case)? else {
        return Ok(());
    };
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        recorded(tool, &format!("{out}.out"))?
    );
    let want_err = match err {
        Some(name) => recorded(tool, &format!("{name}.err"))?,
        None => String::new(),
    };
    assert_eq!(String::from_utf8_lossy(&output.stderr), want_err);
    assert!(output.status.success());
    Ok(())
}
