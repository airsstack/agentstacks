# Design probes — devex-first-time-setup

Run 2026-11-27, aarch64-apple-darwin, rustc/cargo 1.94.1 (workspace pin).
Probe source: probes/probe-main.rs (throwaway crate in /tmp, target deleted after run).
Locked versions proved: clap 4.6.9, serde_json 1.0.154, semver 1.0.28, agentstacks 1.0.0 (path).

## Outcomes (full output: probes/probe-output.txt)
- R1 clap: --summary/--verbose with conflicts_with gives ArgumentConflict exit 2; --dry-run parses.
- R2 cache_home: None/"" -> HomeUnset; any non-empty passes through; url_dir appends .airsstack/cache/git/<sha256>.
- R3 serde_json: "#", "//", "<!--" prefixes all fail parse (expected value at line 1 column 1) — JSON cannot carry a comment stamp.
- R4 semver: ">=0.1.5" matches 0.1.5, 0.1.6, 0.2.0, 1.0.0; not 0.1.4.
- Shell: ${AIRSSTACK_HOME:-$HOME/.airsstack} treats empty as unset; ${AIRSSTACK_HOME-...} does not.
- Existing tests: cargo test -p agentstacks-cli green; cargo test -p agentstacks --lib 1858 passed, 1 failed (fs::disk::tests::a_unix_socket_is_other, PermissionDenied creating a unix socket — sandbox environment, code untouched).
