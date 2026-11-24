# Probe: claudevs and claudevs-cli inside the agentstacks workspace

Date: 2026-11-23

Question: do `claudevs` and `claudevs-cli` build and pass the gate unchanged inside the
agentstacks workspace (airsl 0.1.4, serde_json with `preserve_order`, agentstacks lints), and
does cargo-deny pass?

Setup: the agentstacks worktree was copied with `rsync -a --exclude target --exclude .git` to a
scratch workspace; `claudestacks/crates/claudevs`, `claudestacks/crates/claudevs-cli` and
`claudestacks/scripts/corpus-fetch.sh` were copied in unchanged. Only the root `Cargo.toml` was
edited (diff below). `CARGO_TARGET_DIR` was a fresh scratch directory. Each command ran once.

Versions:

~~~~text
cargo 1.94.1 (29ea6fb6a 2026-03-24)
airsl 0.1.4
cargo-deny 0.20.2
2.1.330 (Claude Code)
~~~~

## Verdict

| Step | Command | Exit | Result |
|---|---|---|---|
| a | `cargo fmt --all -- --check` | 0 | pass, no output |
| b | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | pass, no warnings |
| c | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | 0 | pass |
| d | `cargo test --workspace --all-targets --all-features` | 0 | pass, 0 failed |
| e | `cargo test --workspace --all-features --doc` | 0 | pass |
| f | `cargo deny check` | 0 | pass (4 license-not-encountered warnings only) |
| g | claudevs-check fixture corpus script | 0 | pass, 8 fixtures |
| h | Cargo.lock comparison | n/a | airsl 0.1.4; no new duplicate versions; only claudevs + claudevs-cli added |

No source change was needed: the airsl 0.1.3 -> 0.1.4 bump and `serde_json/preserve_order`
compile and test clean, and both crates (which set `[lints] workspace = true`) pass the agentstacks
workspace lints with `-D warnings`.

## Cargo.toml diff (the only edit)

Command: `diff -u $WT/Cargo.toml $SP/ws/Cargo.toml`

~~~~diff
--- /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/plugin-pack-claudevs-move/Cargo.toml	2026-11-23 15:09:49
+++ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/Cargo.toml	2026-11-23 16:16:15
@@ -14,7 +14,9 @@
 agentstacks = { path = "crates/agentstacks", version = "0.0.0" }
 airsl = "0.1.4"
 clap = { version = "4", features = ["derive"] }
+claudevs = { path = "crates/claudevs", version = "0.1.0" }
 globset = "0.4"
+regex = "1"
 semver = "1"
 serde = { version = "1", features = ["derive"] }
 serde-saphyr = { version = "2", default-features = false, features = ["deserialize"] }
@@ -25,6 +27,7 @@
 thiserror = "2"
 toml = "1"
 toml_edit = "0.25"
+walkdir = "2.5"
 
 [workspace.lints.rust]
 unsafe_code        = "forbid"
~~~~

## a. fmt

Command: `cargo fmt --all -- --check` (exit 0). Full output (empty apart from the exit line):

~~~~text
exit=0
~~~~

## b. clippy

Command: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0).
Output trimmed: 119 lines in total, all `Compiling`/`Checking` progress lines plus the
lines below; `grep -E '^(error|warning)'` over the full output matches nothing. Last 5 lines:

~~~~text
    Checking claudevs v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs)
    Checking claudevs-cli v0.1.1 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs-cli)
    Checking agentstacks-cli v0.0.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/agentstacks-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.25s
exit=0
~~~~

## c. rustdoc

Command: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` (exit 0).
Output trimmed: `Compiling`/`Checking` lines removed.

~~~~text
 Documenting claudevs v0.1.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs)
 Documenting agentstacks v0.0.0 (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/agentstacks)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.92s
   Generated /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/doc/agentstacks/index.html and 1 other file
exit=0
~~~~

## d. tests

Command: `cargo test --workspace --all-targets --all-features` (exit 0).
Output trimmed to the `Running` and `test result` lines per test binary. No failures.

~~~~text
     Running unittests src/lib.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/agentstacks-26cf07ac567da477)
test result: ok. 1693 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.55s
     Running tests/claude.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/claude-7d82eb99b2005b87)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
     Running tests/extension.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/extension-a813743789bbf3c4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/isolation.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/isolation-ed74273ecda666f4)
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
     Running tests/pack.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/pack-dd8881d123cf5b0f)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running tests/runtime.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/runtime-13dada7d5d7fddb6)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/set.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/set-dfe0f0a25909cbac)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running unittests src/main.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/agentstacks-04fd47cbd56b0c23)
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/commands.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/commands-56d207c2e52af589)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
     Running tests/version.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/version-019bad5a688c684c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/claudevs-bd39bcf73ad35376)
test result: ok. 300 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.81s
     Running tests/corpus.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/corpus-05a44e717de3bd5e)
test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/corpus_fetch.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/corpus_fetch-1deabc0c2148d12c)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s
     Running tests/examples.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/examples-ec6e771a532e5e55)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.38s
     Running tests/installed.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/installed-64d2d05ee14cffd1)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
     Running tests/verify_core.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/verify_core-e10e4ffcd1ee078f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
     Running tests/wiring.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/wiring-e1a4bdf9c2811183)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/main.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/claudevs-5a8371b2007b3b5f)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli.rs (/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws-target/debug/deps/cli-e9652f4fd70ebb66)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
exit=0
~~~~

Ignored tests (from the full output):

~~~~text
test harness::project::tests::a_project_is_built_under_a_hostile_global_git_config ... ignored, exercised only as a child process of `project_empty_is_hermetic_against_a_hostile_global_git_config`, under a deliberately hostile GIT_CONFIG_GLOBAL
test the_sweep_covers_every_manifest_root_and_matches_the_committed_snapshot ... ignored, needs the corpus fetched by `cargo make corpus-fetch`
~~~~

## e. doctests

Command: `cargo test --workspace --all-features --doc` (exit 0). Output trimmed to summary lines.

~~~~text
   Doc-tests agentstacks
test result: ok. 292 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
   Doc-tests claudevs
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
exit=0
~~~~

## f. cargo-deny

Command: `cargo deny check` (exit 0). Full output:

~~~~text
warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/deny.toml:45:6
   │
45 │     "CC0-1.0",
   │      ━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/deny.toml:46:6
   │
46 │     "CDLA-Permissive-2.0",
   │      ━━━━━━━━━━━━━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/deny.toml:47:6
   │
47 │     "ISC",
   │      ━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/deny.toml:49:6
   │
49 │     "MIT-0",
   │      ━━━━━ unmatched license allowance

advisories ok, bans ok, licenses ok, sources ok
exit=0
~~~~

## g. claudevs-check fixture corpus

The `[tasks.claudevs-check]` script body from `claudestacks/Makefile.toml` (line 232 onward) was
extracted verbatim to `$SP/claudevs-check.sh` and run as `bash $SP/claudevs-check.sh` from the
workspace root (exit 0). Full output:

~~~~text
ok  minimal-plugin (exit 0, no stage failed)
ok  dead-script-plugin (exit 0, no stage failed)
ok  escape-plugin (FAIL wiring)
ok  bad-matcher-plugin (FAIL wiring)
ok  hooks-array-plugin (FAIL wiring)
ok  project-branch-plugin (exit 0, no stage failed)
ok  exec-args-plugin (exit 0, no stage failed)
ok  matcher-routing-plugin (FAIL test)
exit=0
~~~~

What the validate stage does: `claude` is installed here, so `claudevs check` delegates the
`validate` stage to `claude plugin validate`. Observation against the Makefile comment: with
Claude Code 2.1.330, `bad-matcher-plugin` now PASSES validate (`PreToolUseX` is reported as a
warning, "unknown hook event; entry ignored at runtime", not "Invalid key in record"), while
`hooks-array-plugin` still FAILs validate. The lane still passes because each expectation greps
only for its own stage line (`FAIL  wiring`). Three fixtures run directly with
`cargo run -q -p claudevs-cli -- check crates/claudevs/tests/fixtures/<p>` for the record:

~~~~text
== bad-matcher-plugin
  ok    validate
        Validating plugin manifest: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/bad-matcher-plugin/.claude-plugin/plugin.json
        Validating hooks: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/bad-matcher-plugin/hooks/hooks.json
        ⚠ Found 1 warning:
          ❯ hooks: hooks.PreToolUseX: unknown hook event; entry ignored at runtime
        Plugin directory checks for /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/bad-matcher-plugin. The plugin directory is the catalog that plugins are submitted to; this is a subset of what it runs.
        ✘ Found 2 problems the plugin directory refuses a submission for:
          ❯ README.md: README missing (README_MISSING). This plugin has no README.md. People who install it and the directory's automated review read the README to learn what it does; the first line of a README in the plugin folder also stands in as the listing summary when plugin.json has no description.
            Fix: Add a README.md to the plugin folder describing what the plugin does, how to use it, and any setup it needs, in at least 40 words. A README at the repository root also counts for a plugin in a sub-folder.
          ❯ LICENSE: License missing (LICENSE_MISSING). This plugin has no LICENSE file and plugin.json has no `license`. The directory requires one of them.
            Fix: Add a LICENSE file to the plugin folder, or set `license` (an SPDX id) in plugin.json. A LICENSE at the repository root also counts for a plugin in a sub-folder.
        ⚠ Found 1 plugin directory warning:
          ❯ .claude-plugin/plugin.json icon: No icon (ICON_MISSING). No icon found in the plugin. The validator looked for `icon` in plugin.json and for a file at .claude-plugin/icon.png or .svg, assets/icon.png or .svg, or a similar path.
            Fix: Add a square PNG to the plugin folder at .claude-plugin/icon.png, or set `icon` in plugin.json to the path of a square PNG or JPEG in the plugin: 512 to 2048 px on each side, under 2 MB. SVG and WebP are not accepted. That file may become the listing icon only once: the first time the plugin is saved or submitted in the developer portal. Adding or changing it later does not change the icon.
        These plugin directory checks do not change the result below. To fail on problems the directory refuses a submission for, run this command with --profile directory.
        The directory also runs a safety scan and a policy review that this command does not.
        ✔ Validation passed with warnings
  FAIL  wiring
          error  refs  hooks/hooks.json:7  `${CLAUDE_PLUGIN_ROOT}/scripts/absent.sh` does not exist
          warn   matchers  hooks/hooks.json  `PreToolUseX` is not an event this version of claudevs knows about; it may be newer than the catalogue
          warn   matchers  hooks/hooks.json  claudevs cannot evaluate matcher `Edit(` (regex parse error:
            Edit(
                ^
        error: unclosed group), so it cannot tell whether the runtime accepts it
        1 error, 2 warnings
  skip  test
        no case files found under `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/bad-matcher-plugin/tests` (cases are `*.yaml`, `*_test.lua` or `test_*.lua` in tests/)
  skip  test --installed
        no case files found under `/private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/.tmpTaNx6l/cache/claudevs-fixtures/bad-matcher-plugin/0.1.0/tests` (cases are `*.yaml`, `*_test.lua` or `test_*.lua` in tests/)

2 stages run, 1 failed, 2 skipped
exit=1
== hooks-array-plugin
  FAIL  validate
        Validating plugin manifest: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/hooks-array-plugin/.claude-plugin/plugin.json
        Validating hooks: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/hooks-array-plugin/hooks/hooks.json
        ✘ Found 1 error:
          ❯ hooks: PreToolUse/PermissionRequest is declared at the top level, outside the "hooks" object — a PreToolUse/PermissionRequest hook that cannot be loaded may be what guards the permissions declared beside it, so nothing it sits in is applied until the entry is fixed or removed
        Plugin directory checks for /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/hooks-array-plugin. The plugin directory is the catalog that plugins are submitted to; this is a subset of what it runs.
        ✘ Found 2 problems the plugin directory refuses a submission for:
          ❯ README.md: README missing (README_MISSING). This plugin has no README.md. People who install it and the directory's automated review read the README to learn what it does; the first line of a README in the plugin folder also stands in as the listing summary when plugin.json has no description.
            Fix: Add a README.md to the plugin folder describing what the plugin does, how to use it, and any setup it needs, in at least 40 words. A README at the repository root also counts for a plugin in a sub-folder.
          ❯ LICENSE: License missing (LICENSE_MISSING). This plugin has no LICENSE file and plugin.json has no `license`. The directory requires one of them.
            Fix: Add a LICENSE file to the plugin folder, or set `license` (an SPDX id) in plugin.json. A LICENSE at the repository root also counts for a plugin in a sub-folder.
        ⚠ Found 1 plugin directory warning:
          ❯ .claude-plugin/plugin.json icon: No icon (ICON_MISSING). No icon found in the plugin. The validator looked for `icon` in plugin.json and for a file at .claude-plugin/icon.png or .svg, assets/icon.png or .svg, or a similar path.
            Fix: Add a square PNG to the plugin folder at .claude-plugin/icon.png, or set `icon` in plugin.json to the path of a square PNG or JPEG in the plugin: 512 to 2048 px on each side, under 2 MB. SVG and WebP are not accepted. That file may become the listing icon only once: the first time the plugin is saved or submitted in the developer portal. Adding or changing it later does not change the icon.
        About these checks:
          ❯ hooks/hooks.json has a "hooks" that is not an object. The directory refuses a plugin with such a file (HOOKS_SCHEMA_INVALID). Make "hooks" an object that maps each hook event to a list.
          ❯ No remote service was seen in what these checks could read, and plugin.json sets no privacy policy URL that they accept, so the directory's privacy warning (PRIVACY_URL_MISSING) cannot be ruled out until the file named above can be read in full. Fix that first, or set `privacyPolicyUrl` in plugin.json if the plugin connects to a remote service.
        These plugin directory checks do not change the result below. To fail on problems the directory refuses a submission for, run this command with --profile directory.
        The directory also runs a safety scan and a policy review that this command does not.
        ✘ Validation failed
  FAIL  wiring
          error  matchers  hooks/hooks.json  has no `hooks` object at the top level
        1 error, 0 warnings
  skip  test
        no case files found under `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/hooks-array-plugin/tests` (cases are `*.yaml`, `*_test.lua` or `test_*.lua` in tests/)
  skip  test --installed
        no case files found under `/private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/.tmp6Ej3OF/cache/claudevs-fixtures/hooks-array-plugin/0.1.0/tests` (cases are `*.yaml`, `*_test.lua` or `test_*.lua` in tests/)

2 stages run, 2 failed, 2 skipped
exit=1
== minimal-plugin
  ok    validate
        Validating plugin manifest: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/minimal-plugin/.claude-plugin/plugin.json
        Provides:
          ❯ hooks: PreToolUse Edit|Write command "sh \"${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh\""
          ❯ hooks: SessionStart command "echo 'reminder: load the snapshot'"
          ❯ skills: minimal-plugin:demo
        Hosts not read for: hooks (2), skills (1)
        Hosts not declared: list what these parts reach under "network.hosts" in plugin.json
        Plugin directory checks for /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/ws/crates/claudevs/tests/fixtures/minimal-plugin. The plugin directory is the catalog that plugins are submitted to; this is a subset of what it runs.
        ✘ Found 2 problems the plugin directory refuses a submission for:
          ❯ README.md: README missing (README_MISSING). This plugin has no README.md. People who install it and the directory's automated review read the README to learn what it does; the first line of a README in the plugin folder also stands in as the listing summary when plugin.json has no description.
            Fix: Add a README.md to the plugin folder describing what the plugin does, how to use it, and any setup it needs, in at least 40 words. A README at the repository root also counts for a plugin in a sub-folder.
          ❯ LICENSE: License missing (LICENSE_MISSING). This plugin has no LICENSE file and plugin.json has no `license`. The directory requires one of them.
            Fix: Add a LICENSE file to the plugin folder, or set `license` (an SPDX id) in plugin.json. A LICENSE at the repository root also counts for a plugin in a sub-folder.
        ⚠ Found 1 plugin directory warning:
          ❯ .claude-plugin/plugin.json icon: No icon (ICON_MISSING). No icon found in the plugin. The validator looked for `icon` in plugin.json and for a file at .claude-plugin/icon.png or .svg, assets/icon.png or .svg, or a similar path.
            Fix: Add a square PNG to the plugin folder at .claude-plugin/icon.png, or set `icon` in plugin.json to the path of a square PNG or JPEG in the plugin: 512 to 2048 px on each side, under 2 MB. SVG and WebP are not accepted. That file may become the listing icon only once: the first time the plugin is saved or submitted in the developer portal. Adding or changing it later does not change the icon.
        These plugin directory checks do not change the result below. To fail on problems the directory refuses a submission for, run this command with --profile directory.
        The directory also runs a safety scan and a policy review that this command does not.
        ✔ Validation passed
  ok    wiring
        0 errors, 0 warnings
  ok    test
          ok    blocks-lockfile
          ok    crash-on-garbage
          ok    blocks_Cargo_lock
          ok    blocks_poetry_lock
          ok    rust_files_get_the_guideline_pointer
          ok    session-banner
          ok    skill_command_runs_what_the_skill_documents
          ok    native: sh -c 'echo native-suite-ok' (exit 0)
        7 passed, 0 failed (7 cases, 1 native suites)
  ok    test --installed
          ok    blocks-lockfile
          ok    crash-on-garbage
          ok    blocks_Cargo_lock
          ok    blocks_poetry_lock
          ok    rust_files_get_the_guideline_pointer
          ok    session-banner
          ok    skill_command_runs_what_the_skill_documents
          ok    native: sh -c 'echo native-suite-ok' (exit 0)
        7 passed, 0 failed (7 cases, 1 native suites)

4 stages run, 0 failed, 0 skipped
exit=0
~~~~

## h. Cargo.lock

Command: `$SP/lockcmp.sh $WT/Cargo.lock $SP/ws/Cargo.lock` (greps the airsl entry, lists crates with
two or more versions in each lock, and diffs the name/version pairs). Full output:

~~~~text
== airsl entry in probe Cargo.lock
name = "airsl"
version = "0.1.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a47f225b4b5a151c26115ce4c9da03157f2220a164fa46a9f696246354af9c2b"
== crates with two+ versions: worktree Cargo.lock
block-buffer: 0.10.4 0.12.1
cpufeatures: 0.2.17 0.3.1
crypto-common: 0.1.7 0.2.2
digest: 0.10.7 0.11.3
serde_spanned: 0.6.9 1.1.2
sha2: 0.10.9 0.11.1
syn: 2.0.119 3.0.10
toml_datetime: 0.6.11 1.1.3+spec-1.1.0
toml_edit: 0.22.27 0.25.20+spec-1.1.0
toml: 0.8.23 1.2.0+spec-1.1.0
winnow: 0.7.15 1.0.4
== crates with two+ versions: probe Cargo.lock
block-buffer: 0.10.4 0.12.1
cpufeatures: 0.2.17 0.3.1
crypto-common: 0.1.7 0.2.2
digest: 0.10.7 0.11.3
serde_spanned: 0.6.9 1.1.2
sha2: 0.10.9 0.11.1
syn: 2.0.119 3.0.10
toml_datetime: 0.6.11 1.1.3+spec-1.1.0
toml_edit: 0.22.27 0.25.20+spec-1.1.0
toml: 0.8.23 1.2.0+spec-1.1.0
winnow: 0.7.15 1.0.4
== packages added (in probe, not in worktree)
claudevs 0.1.0
claudevs-cli 0.1.1
== packages removed (in worktree, not in probe)
~~~~

The duplicate-version list is identical before and after: no crate gained a second version.
`regex` and `walkdir` were already in the agentstacks lock transitively, so the only added
packages are the two moved crates.
