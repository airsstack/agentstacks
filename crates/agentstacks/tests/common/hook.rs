//! Building a hook module and running its command.

use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use agentstacks::types::protocol::HookModule;
use agentstacks::types::{HookAction, PackManifest, PackName};

/// The module of `hooks/<name>.lua` in the pack `core`.
pub(crate) fn module(name: &str) -> Result<HookModule, Box<dyn Error>> {
    let text = format!(
        "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n\n\
         [[hooks]]\nkind = \"lua\"\nevent = \"session.start\"\nrun = \"hooks/{name}.lua\"\n"
    );
    let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), &text)
        .map_err(|found| format!("{found:?}"))?;
    let hook = manifest.hooks().first().ok_or("one hook")?;
    let HookAction::Lua(lua) = hook.action() else {
        return Err("a lua hook".into());
    };
    Ok(HookModule::new(&PackName::new("core")?, lua.run()))
}

/// Runs the command `argv` from `dir` with `input` on stdin and returns what it produced.
pub(crate) fn run_piped(
    argv: &[String],
    dir: &Path,
    input: &[u8],
) -> Result<Output, Box<dyn Error>> {
    let (program, rest) = argv.split_first().ok_or("a program")?;
    let mut child = Command::new(program)
        .args(rest)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("a stdin pipe")?.write_all(input)?;
    Ok(child.wait_with_output()?)
}
