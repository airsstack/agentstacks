---
status: done
created: 2026-11-25
depends-on: [02, 05, 09]
built-on: c187d693fbea
---

# ext test Implementation Plan

**Goal:** `agentstacks ext test <EXTENSION>` checks an extension against a built-in conformance pack.

**Architecture:** The rules live in `types/check/`: the staged file set, the embedded conformance
pack, the Lua-test decision and the verdict. A new port, `port/scratch/`, opens an outside
directory as a `FileTree` and creates a temporary project as a `Workspace`. `fs/scratch.rs`
implements it with `tempfile` and the existing disk adapters, and makes no decisions. `Binary`
gains `test`, and `process/binary.rs` runs `airsl test`. `app/ext/check.rs` runs §6.2 steps 1–8
in order over `Binary`, `Scratch`, `Git` and `Adapter`, reusing the plan and apply flows. The CLI
adds `Ext::Test` and `show/check.rs`.

**Tech Stack:** Rust (agentstacks, agentstacks-cli), tempfile 3, airsl 0.1.5 binary, Lua (conformance assets), cargo-make.

**Content authority:** spec §6.1–§6.5 (with two recorded signature deviations from §10: `Scratch::open` takes a label, and `Binary::test` returns `io::Result`), D6, D12, D13, D20, F5, F6, F11, F16, E16–E23, E28, §10 (Scratch port), §12 rows `types/check/*`, `app/ext/check.rs`, `fs/scratch.rs`, `process/binary.rs`, CLI `ext test`.

---

## Files that change

```
crates/agentstacks/assets/conformance/**           — create: the conformance pack (probes/conformance with api = 1 and requires.agentstacks = ">=1.0"; no native/)
crates/agentstacks/assets/conformance/hooks/event_test.lua — create: a real test of event.lua
crates/agentstacks/src/types/check/mod.rs          — create: exports only
crates/agentstacks/src/types/check/conformance.rs  — create: the embedded files (include_str!/include_bytes!)
crates/agentstacks/src/types/check/stage.rs        — create: StagedFile; the scratch file set, its agentstacks.toml text, the --pack rules
crates/agentstacks/src/types/check/lua.rs          — create: has tests (by name); the summary line
crates/agentstacks/src/types/check/verdict.rs      — create: second-plan check; passed/failed
crates/agentstacks/src/types/mod.rs                — modify: pub mod check
crates/agentstacks/src/port/scratch/{mod,seam,fake}.rs — create: the Scratch port and FakeScratch
crates/agentstacks/src/port/mod.rs                 — modify: exports Scratch, FakeScratch
crates/agentstacks/src/port/binary/seam.rs         — modify: fn test(&self, dir) -> io::Result<LuaTestRun>
crates/agentstacks/src/port/binary/fake.rs         — modify: scripted test runs
crates/agentstacks/src/process/binary.rs           — modify: runs airsl test <dir>
crates/agentstacks/src/fs/scratch.rs               — create: Scratch on disk (StdFileTree, TempDir + StdWorkspace)
crates/agentstacks/src/fs/mod.rs                   — modify: exports
crates/agentstacks/Cargo.toml                      — modify: tempfile to [dependencies]
crates/agentstacks/src/app/ext/check.rs            — create: the ext test flow
crates/agentstacks/src/app/ext/mod.rs              — modify: exports check
crates/agentstacks-cli/src/cli.rs                  — modify: Ext::Test { extension, pack }
crates/agentstacks-cli/src/command/ext/check.rs    — create: wiring, streams, exit code
crates/agentstacks-cli/src/command/ext/mod.rs      — modify: exports check
crates/agentstacks-cli/src/show/check.rs           — create: the §6.2 lines
crates/agentstacks-cli/src/show/mod.rs             — modify: exports
crates/agentstacks-cli/src/main.rs                 — modify: dispatch Ext::Test
crates/agentstacks-cli/src/wire.rs                 — modify: builds the real DiskScratch beside the Real ports
crates/agentstacks-cli/tests/commands.rs           — modify: ext test end to end
Makefile.toml                                      — modify: lua task covers crates/agentstacks/assets/conformance
```

## Order of work

### Task 1 — The conformance pack asset

**Files:**
- Create `crates/agentstacks/assets/conformance/` — the files of `probes/conformance/` (pack.toml, skills/check/SKILL.md, skills/check/references/notes.md, skills/check/logo.png, agents/checker.md, hooks/event.lua, data/settings.json), with `api = 1` and `agentstacks = ">=1.0"` added to `[requires]`, and no `native/` (§6.4)
- Create `crates/agentstacks/assets/conformance/hooks/event_test.lua` — tests that `event.lua`'s function returns `{ context = "conformance <event>" }`
- Modify `Makefile.toml` — the `lua` task adds `airsl check crates/agentstacks/assets/conformance` and `airsl test crates/agentstacks/assets/conformance`

**Change:** The pack exists and its Lua is gated.

**Proof:**
- `event_returns_context` (in `event_test.lua`) — run by `cargo make lua`.

**Done when:** `cargo make dod` passes (its `lua` step runs the new test). Commit `feat(agentstacks): the conformance pack asset`.

### Task 2 — The check rules

**Files:**
- Create `crates/agentstacks/src/types/check/mod.rs` — four-question doc; exports only
- Create `crates/agentstacks/src/types/check/conformance.rs` — `CONFORMANCE`, a list of (path, bytes) for every asset file, by `include_str!`/`include_bytes!` (F16)
- Create `crates/agentstacks/src/types/check/stage.rs` — `StagedFile` (an `OpPath` and its bytes; it lives here, in `types`, and the port of Task 3 takes it from here, because `types` may not name `crate::port` beyond `FileTree`/`EntryKind`, `tests/isolation.rs:12,33`); `pack_name(tree, written) -> Result<PackName, Diagnostics>`: no `pack.toml` in the tree gives `--pack "<dir>": not a pack (no pack.toml)`, otherwise `Pack::load`'s refusals or its name (§6.5 rows 4–5); `stage(extension: &Extension, extension_tree, packs: &[(written, PackName, tree)]) -> Result<Vec<StagedFile>, Diagnostics>`: files from `types::tree::walk` over each tree (dot entries skipped; walk refusals are staging refusals, E21); `native/<name>/note.md` with `Native note for <name>.\n`; the `agentstacks.toml` text (api = 1, tools then packs in name order, rendered like `init`); refusals for a duplicate `--pack` name and for `conformance` (§6.5 exact texts)
- Create `crates/agentstacks/src/types/check/lua.rs` — `has_tests(files)`: a file named `*_test.lua` or `test_*.lua`; `summary(stdout)`: the last non-empty line
- Create `crates/agentstacks/src/types/check/verdict.rs` — `second_plan(plan) -> Result<(), NotRepeatable { write, delete, steps }>`: any create, update, restore, delete or forget step fails; a skip never fails (E23); `Verdict::{Passed, Failed}`
- Modify `crates/agentstacks/src/types/mod.rs` — `pub mod check`

**Change:** Every ext test decision as a pure rule.

**Proof:**
- `stage_lists_every_file_and_project_text` — a fake extension and one `--pack` give `ext/<name>/…`, `packs/conformance/…`, `packs/<p>/…` and the exact `agentstacks.toml` text.
- `native_note_for_the_tool` — `packs/conformance/native/mytool/note.md` holds `Native note for mytool.\n`.
- `duplicate_pack_names` — `--pack "<b>": pack "x" is already given by --pack "<a>"`.
- `conformance_name_reserved` — `--pack "<dir>": pack "conformance" is already given by the built-in conformance pack`.
- `walk_refusal_is_staging_refusal` — a symlink in a `--pack` `FakeFileTree` gives `<path>: symlinks are not allowed in a pack`. This is the wording the flow reaches: an extension's symlink is already refused by `Extension::load` at step 2.
- `pack_name_without_pack_toml` — `--pack "<dir>": not a pack (no pack.toml)`.
- `pack_name_with_invalid_name` — `Pack::load`'s own `pack.name` refusal.
- `has_tests_by_name` — `lib/a_test.lua` and `test_b.lua` count; `lib/a.lua` and `.hidden/x_test.lua` do not.
- `summary_is_last_nonempty_line` — F5's pass stdout gives `1 passed, 0 failed (1 files)`.
- `second_plan_write_fails`, `second_plan_delete_fails` — the §6.2 step-7 text with counts.
- `skips_do_not_fail` — a plan with only unchanged steps and skips passes.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): the ext test rules`.

### Task 3 — The Scratch port and Binary::test

**Files:**
- Create `crates/agentstacks/src/port/scratch/mod.rs`, `seam.rs`, `fake.rs` — `trait Scratch { type Tree: FileTree; type Project: Workspace; fn open(&self, dir: &Path, label: &Path) -> io::Result<Self::Tree>; fn create(&self, files: &[StagedFile]) -> io::Result<Self::Project>; }`, with `StagedFile` taken from `types::check`; `label` is the path as the user wrote it, which every refusal shows (§6.5 "as written"), and `dir` is the path resolved against `-C`. Spec §10 writes `open(dir)`; the added `label` is a recorded deviation, needed for §6.5's wording. `FakeScratch` has a `FakeFileTree` per registered dir, a `FakeWorkspace` preloaded with the files, and recorded calls
- Modify `crates/agentstacks/src/port/mod.rs` — exports
- Modify `crates/agentstacks/src/port/binary/seam.rs` — `fn test(&self, dir: &Path) -> io::Result<LuaTestRun>`; `LuaTestRun { success: bool from the exit status, stdout, stderr }` as a port value. Spec §10 writes `-> LuaTestRun`; the `io::Result` carries D20's spawn failure and is a recorded deviation
- Modify `crates/agentstacks/src/port/binary/fake.rs` — a scripted test result and a spawn failure

**Change:** The abstractions `app/ext/check.rs` needs, with fakes and no logic.

**Proof:**
- `fake_scratch_serves_registered_dirs` — `open` of a registered dir returns its tree, and an unregistered dir gives `NotFound`.
- `fake_scratch_create_preloads_workspace` — the project returned by `create` reads back the files.
- `fake_binary_test_returns_script` — the scripted run and the spawn failure come back as set.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): the Scratch port and Binary::test`.

### Task 4 — The adapters

**Files:**
- Create `crates/agentstacks/src/fs/scratch.rs` — `DiskScratch`: `open(dir, label)` returns `StdFileTree::labelled(dir, label)`; `create` makes a `tempfile::TempDir`, writes each file (parent directories created), and returns a project type that wraps `StdWorkspace` and holds the `TempDir`, so dropping it removes the directory
- Modify `crates/agentstacks/src/fs/mod.rs` — exports
- Modify `crates/agentstacks/Cargo.toml` — `tempfile = { workspace = true }` under `[dependencies]` (stays in dev-dependencies too if used there)
- Modify `crates/agentstacks/src/process/binary.rs` — `test` runs `airsl test <dir>` and returns exit success, stdout and stderr as bytes decoded lossily

**Change:** Translation only (§10).

**Proof:**
- `disk_scratch_contract` (contract test, under `$TMPDIR`) — `open` reads a directory like `FakeFileTree` does; `create` writes the files; after drop the directory is gone (E22).
- `airsl_test_contract` (contract test, needs airsl) — on a temp dir with a passing test, `success` is true and the stdout's last line is `1 passed, 0 failed (1 files)`; on a failing one, `success` is false (F5).

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): scratch projects on disk and airsl test`.

### Task 5 — The ext test flow

**Files:**
- Create `crates/agentstacks/src/app/ext/check.rs` — `check<B, S, G, A>(binary, scratch, git, adapter, extension: (resolved, written), packs: &[(resolved, written)]) -> CheckReport`: §6.2 steps 1–8 in order. Step 1 `on_path(airsl)`. Step 2 `scratch.open` + `Extension::load`. Step 3 `has_tests` then `binary.test`, where a spawn failure is D20. Step 4 opens each `--pack` (an I/O error is `<written>: <io::Error Display>`), then `pack_name`, `stage` and `scratch.create`; every decision is in `types/check/stage.rs`. Steps 5–7 run the existing plan and apply flows on the project, then plan again. Step 8 is the verdict. The report holds each step's outcome for the CLI to print; steps 1–2 failures carry no verdict
- Modify `crates/agentstacks/src/app/ext/mod.rs` — exports `check`

**Change:** The §6.2 sequence. It holds no rule of its own.

**Proof:** (with `FakeBinary`, `FakeScratch`, `FakeAdapter`, `FakeGit`)
- `airsl_missing_stops_without_verdict` — step-1 refusal text; no verdict.
- `load_refusal_stops_without_verdict` — an extension load refusal; no verdict.
- `no_tests_is_none` — no test file: `lua tests: none` outcome; passes (E16).
- `tests_fail_run_continues_and_fails` — a scripted failing run: the plan still runs; verdict failed (E17).
- `airsl_spawn_failure_fails` — D20 outcome; verdict failed (E28).
- `pack_dir_missing`, `pack_dir_not_a_pack` — the §6.5 texts.
- `plan_refusal_stops_with_failed` — a scripted extension that leaves an item uncovered: refusals; verdict failed.
- `not_repeatable_fails` — a scripted extension that writes different content on the second dispatch: step-7 failure (E18).
- `skips_pass` — a scripted extension that skips items: passed (E23).

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): the ext test flow`.

### Task 6 — The ext test command

**Files:**
- Modify `crates/agentstacks-cli/src/cli.rs` — `Ext::Test { extension: PathBuf, #[arg(long = "pack", value_name = "DIR")] pack: Vec<PathBuf> }`; the positional is named `extension`, never `dir` (F6)
- Create `crates/agentstacks-cli/src/command/ext/check.rs` — resolves each path against `-C` or the current directory, passes both the resolved path and the path as written (the label) to the flow, runs it with the real adapters, prints, and returns the exit code
- Modify `crates/agentstacks-cli/src/command/ext/mod.rs`, `crates/agentstacks-cli/src/main.rs` — dispatch
- Modify `crates/agentstacks-cli/src/wire.rs` — a `DiskScratch` built beside the `Real` ports (`wire.rs:28,36-50`); `Ports` keeps its four parameters, and `ext test` takes the scratch adapter as its own argument
- Create `crates/agentstacks-cli/src/show/check.rs` — the stdout lines of §6.2: `ext test <name> <version>`, `lua tests: …`, the plan text through `show/plan.rs`, `applied: …`, `second plan: …`, `passed`/`failed`; refusals and airsl's stderr go to stderr
- Modify `crates/agentstacks-cli/src/show/mod.rs` — exports
- Modify `crates/agentstacks-cli/tests/commands.rs` — end to end

**Change:** §6.1 and the output of §6.2.

**Proof:**
- `ext_test_parses` (in `cli.rs`) — F6's `ext test` argv lines parse with `-C` in every position.
- `check_text_passed` / `check_text_failed` (in `show/check.rs`) — the exact lines for a passing and a failing report.
- `ext_test_claude_passes` (in `tests/commands.rs`, needs airsl) — `agentstacks ext test <repo>/extensions/claude` exits 0 and its last stdout line is `passed`.
- `ext_test_opencode_passes` — the same for `extensions/opencode`; the plan text lists the two skips of F11.
- `ext_test_writes_nothing_outside_scratch` — the extension directory's file hashes are the same before and after.
- `ext_test_labels_paths_as_written` — `agentstacks -C proj ext test ext/missing` prints `ext/missing: <io::Error Display>`, not a resolved path.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks-cli): agentstacks ext test`.

---

## Risks

- The riskiest step is Task 5's reuse of the plan and apply flows on a `Scratch::Project`. They are generic over `Workspace`, so the wrapper in Task 4 must implement `Workspace` fully, delegating to `StdWorkspace`.
- `tests/isolation.rs` must stay green. `fs/scratch.rs` is an adapter, so it may use `std::fs` and `tempfile`, while `types/check/` and `app/ext/` must not.
- The CLI integration tests need airsl on `PATH`, as today's CLI tests do. CI already installs it.

## Proof

- `cargo make dod` is green, including `cargo make lua` over the conformance assets.
- `agentstacks ext test extensions/claude` and `extensions/opencode` both print `passed`.

## Review findings

One `claudestacks:reviewer` over tasks 1–6 (`cargo make dod` exit 0; `ext test extensions/claude` and
`ext test extensions/opencode` each exit 0 with last line `passed`, opencode with the two F11 skips):
0🔴 1🟡 15🔵. Every code finding went to one fix coder; `cargo make dod` after the fix: exit 0.

- fixed 🟡 `agentstacks-cli/src/command/ext/check.rs`: test `a_failed_verdict_is_refused_and_ends_stdout_with_failed`.
- fixed 🔵 `agentstacks-cli/src/command/mod.rs:5` rewrapped; `crates/agentstacks/Cargo.toml` `testing` comment names `FakeScratch`.
- fixed 🔵 `port/adapter/fake.rs`: queued-script doc matches behaviour.
- fixed 🔵 `port/scratch/fake.rs`: no hard-coded `ext/`/`packs/`; `FakeWorkspace::source_tree` serves files under a prefix (test `source_tree_serves_the_files_held_under_a_directory`).
- fixed 🔵 `process/binary.rs`: `skip_without_airsl` honours `AGENTSTACKS_REQUIRE_AIRSL` in both airsl contracts.
- fixed 🔵 `types/check/lua.rs`: doctest reads an in-crate fixture; summary test uses airsl's real stdout shape.
- fixed 🔵 `types/check/mod.rs`: doc says apply runs once; `GivenPack` and `NAME` re-exported.
- fixed 🔵 `types/check/stage.rs`: `GivenPack` is a named struct; `FirstPlan { Planned, Applied }` replaces `plan` + `applied: bool`.
- fixed 🔵 `Makefile.toml` comment rewrapped.
- fixed 🔵 `book/src/ch07-00-architecture.md`: app, types and testing rows name ext, check and `FakeScratch`.
- fixed 🔵 `book/src/ch04-00-cli.md` stderr row and `--tool/--pack` line: in plan 07 Task 3 (owner of the ch04 rewrite).
- fixed 🔵 `types/check/verdict.rs:72`: a forget-only second plan now reads `second plan: 0 to write, 0 to delete, <f> forgotten (apply is not repeatable)` (fixed in the chain-wide cross-check pass; `cargo make dod` exit 0). See Deviations.
- recorded 🔵 plan Files additions: see Deviations.

## Deviations

- 2026-11-25: Added beyond the Files list: `types/check/error.rs` `CheckError` (typed step-1 and airsl-spawn texts), `FormatError::{Stage, Check}`, `Verdict::of`, `app::ext::{Dir, FirstPlan, LuaOutcome, CheckReport, Checked}`, `FakeAdapter` queued scripts (one per run, so a second plan can be scripted), `FakeFileTree::relabel`, prefix source trees in `FakeWorkspace`. Each keeps the rule in `types/` or makes a flow testable with fakes.
- 2026-11-25: `pack.toml` of the conformance asset carries `[pack] api = 1` (the brief read "api = 1 under [requires]"; `requires` has no api key).
- 2026-11-25: A passing airsl run that prints nothing renders `lua tests: passed`; the spec gives only `lua tests: <last non-empty stdout line>`.
- 2026-11-25: `wire::scratch()` is a separate function so `Ports` keeps four parameters.
- 2026-11-25: The D20 second-plan refusal gains `, <f> forgotten` only when forget steps exist; with none it is the spec's text byte for byte. `book/src/ch04-00-cli.md` documents it.
- 2026-11-25: Chain-wide cross-check renamed `Binary::test` to `Binary::airsl_test` (and its fake), `Dir.written`/`GivenPack.written` to `label`, `FakeFileTree::relabel` to `labelled`; added `types::name::AIRSL`, `app/local.rs` `local_tree`, `ProjectFile::refuse_legacy`, and one shared `AlreadyGiven` phrase (`StageError::Reserved` folded into it, texts unchanged).
- 2026-11-25: Several coders wrote tests together with the code instead of watching them fail first (tasks 2, 3, 6); the flow (task 5) was seen red against a stub.
