//! The [`Binary`] port on `PATH`.
//!
//! # Responsibilities
//!
//! - [`SystemBinary`] looks a binary up in the directories of a `PATH` value, and runs the
//!   `airsl` it finds there with `--version`, or with `test` over a directory. The `PATH` is a
//!   constructor argument, so a test never changes the environment of the process.
//! - A binary is found when a `PATH` entry holds a regular file of that name with an execute bit.
//!   A symlink counts when it leads to such a file, and an empty entry is skipped, so the current
//!   directory is never searched.
//!
//! # Non-responsibilities
//!
//! Reading the version out of what `airsl` printed and comparing it with a requirement belong to
//! the rules in `types::op`.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::port::Binary;
use crate::port::binary::LuaTestRun;
use crate::types::name::{AIRSL, BinaryName};
use crate::types::plan::{AirslAnswer, Exit};

/// The executables on a `PATH`.
///
/// # Examples
///
/// ```
/// use agentstacks::port::Binary;
/// use agentstacks::process::SystemBinary;
/// use agentstacks::types::BinaryName;
///
/// // Without a `PATH` nothing is found.
/// let binaries = SystemBinary::new(None);
/// assert!(!binaries.on_path(&BinaryName::new("git")?));
/// # Ok::<(), agentstacks::types::InvalidName>(())
/// ```
#[derive(Debug, Clone)]
pub struct SystemBinary {
    path: Option<OsString>,
}

impl SystemBinary {
    /// Searches the directories of `path`, the value of `PATH`. `None` finds nothing.
    #[must_use]
    pub const fn new(path: Option<OsString>) -> Self {
        Self { path }
    }

    /// Searches the `PATH` of the environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(std::env::var_os("PATH"))
    }

    /// The first executable file called `name` in the directories of `PATH`.
    fn find(&self, name: &str) -> Option<PathBuf> {
        let path = self.path.as_deref()?;
        search_dirs(path)
            .into_iter()
            .map(|dir| dir.join(name))
            .find(|candidate| is_executable(candidate))
    }
}

/// The directories of `path` that are searched: every entry but the empty ones.
fn search_dirs(path: &OsStr) -> Vec<PathBuf> {
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .collect()
}

/// Whether `path` is a regular file with an execute bit. A symlink is followed.
fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

impl Binary for SystemBinary {
    fn on_path(&self, name: &BinaryName) -> bool {
        self.find(name.as_str()).is_some()
    }

    fn airsl_version(&self) -> AirslAnswer {
        let Some(program) = self.find(AIRSL) else {
            return AirslAnswer::NotFound;
        };
        match Command::new(program)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
        {
            Ok(output) => {
                let exit = if output.status.success() {
                    Exit::Success
                } else {
                    Exit::Failure
                };
                AirslAnswer::from_output(&output.stdout, exit)
            }
            // `find` located the file, so even `NotFound` (a missing script interpreter) is a
            // failure to run it, not an absent binary.
            Err(error) => AirslAnswer::SpawnFailed(error),
        }
    }

    fn airsl_test(&self, dir: &Path) -> io::Result<LuaTestRun> {
        let program = self
            .find(AIRSL)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "airsl not found on PATH"))?;
        let output = Command::new(program)
            .arg("test")
            .arg(dir)
            .stdin(Stdio::null())
            .output()?;
        Ok(LuaTestRun {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};

    use super::*;
    use crate::types::op::parse_airsl_version;

    type TestResult = Result<(), Box<dyn Error>>;

    fn name(text: &str) -> Result<BinaryName, Box<dyn Error>> {
        Ok(BinaryName::new(text)?)
    }

    /// A file `file` of `mode` in `dir`, written and closed.
    fn put(dir: &Path, file: &str, mode: u32) -> Result<PathBuf, Box<dyn Error>> {
        let path = dir.join(file);
        fs::write(&path, b"")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
        Ok(path)
    }

    fn on(dirs: &[&Path]) -> Result<SystemBinary, Box<dyn Error>> {
        Ok(SystemBinary::new(Some(std::env::join_paths(dirs)?)))
    }

    #[test]
    fn a_file_with_an_execute_bit_is_on_path() -> TestResult {
        let (first, second) = (tempfile::tempdir()?, tempfile::tempdir()?);
        put(second.path(), "rg", 0o755)?;
        let binaries = on(&[first.path(), second.path()])?;
        assert!(binaries.on_path(&name("rg")?));
        assert!(!binaries.on_path(&name("fd")?));
        Ok(())
    }

    #[test]
    fn a_file_without_an_execute_bit_is_not_on_path() -> TestResult {
        let dir = tempfile::tempdir()?;
        put(dir.path(), "rg", 0o644)?;
        assert!(!on(&[dir.path()])?.on_path(&name("rg")?));
        Ok(())
    }

    #[test]
    fn a_directory_named_like_the_binary_is_not_on_path() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir(dir.path().join("rg"))?;
        assert!(!on(&[dir.path()])?.on_path(&name("rg")?));
        Ok(())
    }

    #[test]
    fn no_path_finds_nothing() -> TestResult {
        assert!(!SystemBinary::new(None).on_path(&name("rg")?));
        assert!(matches!(
            SystemBinary::new(None).airsl_version(),
            AirslAnswer::NotFound
        ));
        Ok(())
    }

    #[test]
    fn a_symlink_to_an_executable_is_on_path() -> TestResult {
        let (bin, store) = (tempfile::tempdir()?, tempfile::tempdir()?);
        let runnable = put(store.path(), "real", 0o755)?;
        let plain = put(store.path(), "plain", 0o644)?;
        symlink(&runnable, bin.path().join("rg"))?;
        symlink(&plain, bin.path().join("fd"))?;
        let binaries = on(&[bin.path()])?;
        assert!(binaries.on_path(&name("rg")?));
        assert!(!binaries.on_path(&name("fd")?));
        Ok(())
    }

    #[test]
    fn empty_path_entries_are_skipped() -> TestResult {
        let dir = tempfile::tempdir()?;
        put(dir.path(), "rg", 0o755)?;
        let mut path = OsString::from(":");
        path.push(dir.path());
        assert_eq!(search_dirs(&path), [dir.path().to_path_buf()]);
        assert!(SystemBinary::new(Some(path)).on_path(&name("rg")?));
        assert!(search_dirs(OsStr::new("")).is_empty());
        Ok(())
    }

    #[test]
    fn airsl_absent_is_not_found() -> TestResult {
        let dir = tempfile::tempdir()?;
        put(dir.path(), "rg", 0o755)?;
        assert!(matches!(
            on(&[dir.path()])?.airsl_version(),
            AirslAnswer::NotFound
        ));
        Ok(())
    }

    #[test]
    fn a_found_airsl_that_will_not_run_is_a_spawn_failure() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = put(dir.path(), "airsl", 0o755)?;
        // The interpreter on the first line is a directory, which `exec` refuses to run.
        fs::write(&path, format!("#!{}\n", dir.path().display()))?;
        let answer = on(&[dir.path()])?.airsl_version();
        assert!(matches!(answer, AirslAnswer::SpawnFailed(_)), "{answer:?}");
        Ok(())
    }

    #[test]
    fn a_found_airsl_with_a_missing_interpreter_is_a_spawn_failure_not_absent() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = put(dir.path(), "airsl", 0o755)?;
        fs::write(&path, "#!/nonexistent/interp\n")?;
        let answer = on(&[dir.path()])?.airsl_version();
        assert!(matches!(answer, AirslAnswer::SpawnFailed(_)), "{answer:?}");
        Ok(())
    }

    /// Whether the test must stop because `airsl` is not on `PATH`. It fails instead when
    /// `AGENTSTACKS_REQUIRE_AIRSL` is set, so a missing binary cannot pass the suite unseen.
    fn skip_without_airsl(binaries: &SystemBinary) -> Result<bool, Box<dyn Error>> {
        if binaries.on_path(&name(AIRSL)?) {
            return Ok(false);
        }
        if std::env::var_os("AGENTSTACKS_REQUIRE_AIRSL").is_some() {
            return Err("airsl is not on PATH and AGENTSTACKS_REQUIRE_AIRSL is set".into());
        }
        println!("skipped: airsl is not on PATH");
        Ok(true)
    }

    #[test]
    fn real_airsl_prints_a_parsable_version() -> TestResult {
        let binaries = SystemBinary::from_env();
        if skip_without_airsl(&binaries)? {
            return Ok(());
        }
        let answer = binaries.airsl_version();
        assert!(
            matches!(&answer, AirslAnswer::Printed(text) if parse_airsl_version(text).is_some()),
            "{answer:?}"
        );
        Ok(())
    }

    /// A directory holding one Lua test file whose single test has `body`.
    fn lua_dir(body: &str) -> Result<tempfile::TempDir, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        fs::write(
            dir.path().join("sample_test.lua"),
            format!("return {{\n  sample = function()\n    {body}\n  end,\n}}\n"),
        )?;
        Ok(dir)
    }

    #[test]
    fn airsl_test_contract() -> TestResult {
        let binaries = SystemBinary::from_env();
        if skip_without_airsl(&binaries)? {
            return Ok(());
        }
        let passing = lua_dir("assert(1 + 1 == 2)")?;
        let run = binaries.airsl_test(passing.path())?;
        assert!(run.success, "{run:?}");
        assert_eq!(
            run.stdout.lines().last(),
            Some("1 passed, 0 failed (1 files)"),
            "{run:?}"
        );

        let failing = lua_dir("assert(false, \"boom\")")?;
        let run = binaries.airsl_test(failing.path())?;
        assert!(!run.success, "{run:?}");
        Ok(())
    }

    #[test]
    fn test_without_airsl_on_path_is_not_found() -> TestResult {
        let dir = tempfile::tempdir()?;
        let Err(error) = SystemBinary::new(None).airsl_test(dir.path()) else {
            return Err("a missing airsl must be an error".into());
        };
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }
}
