//! Binary smoke test: `--version`, asserted through the real binary.

#![expect(
    clippy::expect_used,
    reason = "tests expect() known-valid subprocess output"
)]

use std::process::Command;

#[test]
fn version_flag_prints_name_and_crate_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentstacks"))
        .arg("--version")
        .output()
        .expect("the agentstacks binary launches");

    assert!(output.status.success(), "--version must exit successfully");
    let stdout = String::from_utf8(output.stdout).expect("--version prints UTF-8");
    assert_eq!(
        stdout.trim(),
        format!("agentstacks {}", env!("CARGO_PKG_VERSION"))
    );
}
