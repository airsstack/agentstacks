---
status: done
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# claudevs import Implementation Plan

**Goal:** Make `claudevs` and `claudevs-cli` workspace members here, laid out under the one-word file rule.

**Architecture:**
- The two crates are copied whole from `../claudestacks/crates/` in one commit with no claudestacks
  history (spec §2 "Import").
- The nine compound file names are then renamed with `git mv`; `types/mod.rs` and `harness/mod.rs`
  keep their flat re-exports, so the public API does not change (spec §2 "Renames", §10).
- `claudevs-cli → claudevs → airsl`; no edge to `agentstacks` (spec §10).

**Tech Stack:** Rust 1.94.1 (`rust-toolchain.toml`), cargo workspace, airsl 0.1.4 from crates.io,
`regex` 1, `walkdir` 2.5, cargo-make.

**Content authority:** spec §1 (claudevs rows), §2, §10, §11 (claudevs row).

---

## Files that change

```
crates/claudevs/**                         — create: copied from ../claudestacks/crates/claudevs/
crates/claudevs-cli/**                     — create: copied from ../claudestacks/crates/claudevs-cli/
scripts/corpus-fetch.sh                    — create: copied from ../claudestacks/scripts/corpus-fetch.sh
Cargo.toml                                 — modify: three [workspace.dependencies] entries
Cargo.lock                                 — modify: gains claudevs 0.1.0 and claudevs-cli 0.1.1
crates/claudevs/src/types/name/mod.rs      — create: declares and re-exports case, marketplace, plugin
crates/claudevs/src/types/name/case.rs     — rename from types/case_name.rs
crates/claudevs/src/types/name/marketplace.rs — rename from types/marketplace_name.rs
crates/claudevs/src/types/name/plugin.rs   — rename from types/plugin_name.rs
crates/claudevs/src/types/version.rs       — rename from types/plugin_version.rs
crates/claudevs/src/types/event.rs         — rename from types/hook_event.rs
crates/claudevs/src/types/mod.rs           — modify: module declarations, re-export paths, prose
crates/claudevs/src/harness/module.rs      — rename from harness/t_module.rs
crates/claudevs/src/harness/hooks.rs       — rename from harness/hooks_file.rs
crates/claudevs/src/harness/mod.rs         — modify: module declarations and re-export paths
crates/claudevs/tests/fetch.rs             — rename from tests/corpus_fetch.rs
crates/claudevs/tests/engine.rs            — rename from tests/verify_core.rs
```

## Order of work

### Task 1 — Import the two crates and the corpus script

**Files:**
- Create `crates/claudevs/**` and `crates/claudevs-cli/**`. Copy the whole trees, including
  README, CHANGELOG, LICENSE, `docs/`, `examples/`, `tests/fixtures/` and `tests/corpus/`, but no
  `target/`.
- Create `scripts/corpus-fetch.sh`, copied unchanged.
- Modify `Cargo.toml`: under `[workspace.dependencies]` add
  `claudevs = { path = "crates/claudevs", version = "0.1.0" }`, `regex = "1"` and
  `walkdir = "2.5"`. The members glob `crates/*` already covers both crates (`Cargo.toml:3`).
- Modify `Cargo.lock` by building.

**Change:** The crates build here with no source edit. Crate versions stay `0.1.0` (claudevs) and
`0.1.1` (claudevs-cli). `repository` is inherited from this workspace. `deny.toml` is not touched
(spec §1: `cargo deny check` passes; `regex` 1.13.1 and `walkdir` 2.5.1 are already locked here).
The commit message and files name no claudestacks commit.

**Proof:**
- The crates' own unit, integration and doc tests, unchanged, pass in this workspace (spec §1,
  `probes/claudevs-results.md` steps d–e).
- `cargo tree -d` prints the same duplicate list as before the import (the 11 crates of
  `probes/claudevs-results.md` step h); no crate gains a second version (spec §1).

**Done when:** `cargo make dod` and `cargo make deny` are green. Commit
`build(repo): import claudevs and claudevs-cli`.

### Task 2 — Group the name newtypes and rename the other two type files

**Files:**
- `git mv` `src/types/case_name.rs` → `src/types/name/case.rs`, `marketplace_name.rs` →
  `name/marketplace.rs`, `plugin_name.rs` → `name/plugin.rs`, `plugin_version.rs` → `version.rs`,
  `hook_event.rs` → `event.rs` (all under `crates/claudevs/`).
- Create `src/types/name/mod.rs`.
- Modify `src/types/mod.rs`.

**Change:**
- `types/name/mod.rs` holds only a `//!` block (what the module holds, why it exists, what it
  depends on, how it is tested — the four-question block the other `mod.rs` files use), the three
  `mod` lines and their `pub use` lines (`mod-rs-export-only.md`).
- `types/mod.rs` declares `mod name; mod version; mod event;` in place of the five old modules, and
  its `pub use` block re-exports the same item names as before from the new paths
  (`pub use name::{CaseName, InvalidCaseName, …}`), so every `crate::types::X` path still
  resolves.
- Its prose that says "this file" or "Four of the five types this module exports" is reworded to
  the new layout.
- Each moved file keeps its content. Only doc comments that name an old path change.

**Proof:**
- Every existing test that uses `CaseName`, `MarketplaceName`, `PluginName`, `PluginVersion` or
  `HookEvent` passes unchanged.
- `find crates/claudevs crates/claudevs-cli -name '*.rs' -not -path '*/fixtures/*' | grep '_'`
  lists only the four files Task 3 renames.

**Done when:** `cargo make dod` is green. Commit
`refactor(claudevs): group name types and rename type files to one word`.

### Task 3 — Rename the harness files and the two test files

**Files:**
- `git mv` `src/harness/t_module.rs` → `src/harness/module.rs`, `src/harness/hooks_file.rs` →
  `src/harness/hooks.rs`, `tests/corpus_fetch.rs` → `tests/fetch.rs`, `tests/verify_core.rs` →
  `tests/engine.rs` (all under `crates/claudevs/`).
- Modify `src/harness/mod.rs`.

**Change:**
- `harness/mod.rs` declares `mod module; mod hooks;` and re-exports the same items as before
  (`pub use hooks::…`, `pub use module::TModule`).
- Doc comments and comments that name `t_module.rs`, `hooks_file.rs`, `corpus_fetch.rs` or
  `verify_core.rs` are updated to the new names.

**Proof:**
- `grep -rnE '(hooks_file|t_module|corpus_fetch|verify_core|case_name|plugin_name|marketplace_name|plugin_version|hook_event)(\.rs|::)' crates/claudevs crates/claudevs-cli`
  prints nothing (file and module paths only; identifiers such as `hook_event_name` stay). This also covers module paths in comments (`suite.rs:190,275`, `case/lua.rs:311`).
- The renamed integration test targets `fetch` and `engine` run and pass:
  `cargo test -p claudevs --test fetch` and `cargo test -p claudevs --test engine`.
- `find crates/claudevs crates/claudevs-cli -name '*.rs' -not -path '*/fixtures/*' | grep '_'`
  prints nothing; `… | wc -l` prints `57` (56 files, plus `types/name/mod.rs`).

**Done when:** `cargo make dod` is green. Commit
`refactor(claudevs): rename harness and test files to one word`.

---

## Risks

- A doc comment or intra-doc link that names an old module path breaks `cargo doc -D warnings`.
  The rustdoc step catches it; fix the link, not the lint.
- A `#[path]` attribute or `include_str!` naming a moved file would break the build. Task 2 and
  Task 3 grep each crate for the old file names before committing.
- The import must not carry `target/` or `.git` from claudestacks.

## Proof

- `cargo make dod` green after every task; `cargo make deny` green after Task 1.
- `cargo test -p claudevs` runs the same test count before Task 2 and after Task 3.

## Review findings

- 🟡 fixed — git index half-staged (`types/mod.rs`, `harness/mod.rs` staged with old content beside staged renames; `types/name/mod.rs`, `scripts/` untracked): `git add -A`; `git status --short` then shows only `A`/`M`/`D`/`R` entries (178 A, 17 M, 10 D, 14 R), none partial.
- 🔵 fixed — `types/mod.rs:4` "(in `name`)" read as PluginName only; now "(all in `name`)".
- 🔵 fixed — `types/mod.rs:12` "the exception in this file" (Task 2 "this file" rewording missed); now "[`HookEvent`] (in `event`) is the exception here".
- 🔵 fixed — `types/mod.rs:28` 82-char line; block reflowed to ≤ 80.
- 🔵 fixed — `types/name/mod.rs:8` named "the parent `types` module"; now names `types::ident` and says only `marketplace` and `plugin` use it.
- Verification after fixes: `cargo make dod` (workspace) exit 0 — 21 `test result: ok` blocks, 2472 Rust tests passed, lua `63 passed, 0 failed (5 files)`, `Build Done in 77.30 seconds`.
- 🔵 fixed (follow-up) — `crates/claudevs/tests/corpus/corpus.toml:13` cites a claudestacks planning path (carried over unchanged; Task 1 is a no-edit import).
- ❓ fixed (follow-up) — `crates/claudevs/README.md:64` links claudestacks `examples/README.md` (carried over unchanged).
- 🔵 fixed (follow-up) — nothing in the gate guards the one-word file-name rule; only the plan's grep does.

## Deviations

- 2026-11-23 — The task-briefer ledger spawn was skipped; tasks were listed with `grep -n '^### Task'` (same rows).
- 2026-11-23 — Tasks 2 and 3 ran in one coder, in order: both rebuild `claudevs`, so parallel coders would break each other's build.
- 2026-11-23 — Per-task commits were not made (execute does not commit unless told). The three tasks sit staged in one index, renames recorded by `git mv` after `git add crates/claudevs crates/claudevs-cli` (git mv refuses untracked files). Cutting the three commits needs restaging.
- 2026-11-23 — Plan claim "the 11 crates of `probes/claudevs-results.md` step h": `cargo tree -d | grep -E '^[a-z]'` gives 20 lines = 10 crates with two versions each (block-buffer, cpufeatures, crypto-common, digest, serde_spanned, sha2, toml, toml_datetime, toml_edit, winnow), identical before and after the import. The proof (no crate gains a version) holds; the count differs.
- 2026-11-23 — Test counts: 321 claudevs tests before Task 2 and after Task 3 (lib 301, corpus 4, engine/verify_core 2, examples 2, fetch/corpus_fetch 4, installed 2, wiring 5, doc 1); `find … | grep '_'` prints nothing; `wc -l` 57.

## Follow-up fixes

- 2026-11-23 — Every open finding above was fixed in one follow-up pass (three parallel coders, disjoint files). Workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0: 23 `test result: ok` blocks, 2474 Rust tests passed, lua `71 passed, 0 failed (5 files)`, `Build Done in 55.65 seconds`.
- File-name rule: `source_and_test_names_are_one_word` in `crates/claudevs/tests/layout.rs` and `crates/agentstacks/tests/layout.rs` (red with a stray `src/x_y/`; green after removal). CLI crates checked by hand: `find crates/*-cli/src -name '*_*'` prints nothing.
- `corpus.toml` comment drops the planning path; `crates/claudevs/README.md` links the local `examples/README.md`.
