//! Whether the `airsl` binary can run, and what a test does when it cannot.

use std::error::Error;
use std::process::Command;

/// The variable that turns a missing `airsl` from a skip into a failure.
const REQUIRE_AIRSL: &str = "AGENTSTACKS_REQUIRE_AIRSL";

/// The version `airsl` reports, or `None` when it cannot run.
fn airsl_version() -> Option<String> {
    let output = Command::new("airsl").arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Whether the test must stop: a pack with lua hooks needs the `airsl` binary, because the plan
/// checks it, and a translator run needs it to run the hook. When it cannot run, the test says so,
/// unless `AGENTSTACKS_REQUIRE_AIRSL` is set: then the test fails, so a missing binary cannot pass
/// the suite unseen.
pub(crate) fn skip_without_airsl() -> Result<bool, Box<dyn Error>> {
    if let Some(version) = airsl_version() {
        eprintln!("ran on {version}");
        return Ok(false);
    }
    if std::env::var_os(REQUIRE_AIRSL).is_some() {
        return Err(format!("airsl cannot run on this machine and {REQUIRE_AIRSL} is set").into());
    }
    eprintln!("skipped: airsl cannot run on this machine");
    Ok(true)
}
