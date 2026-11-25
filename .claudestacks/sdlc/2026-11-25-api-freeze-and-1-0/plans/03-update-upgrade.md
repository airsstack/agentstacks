---
status: done
created: 2026-11-25
depends-on: [01]
built-on: c187d693fbea
---

# update moves a 0.x project Implementation Plan

**Goal:** `agentstacks update` moves a legacy project to `api = 1`.

**Architecture:** A new pure rule, `types/project/upgrade.rs`, rewrites `api = 0` to `1` keeping the
value's decor (F3), with the re-read refusal, as a sibling of `retag.rs`. `app/update.rs` returns
the api move next to the tag entries. The CLI prints `  api: 0 → 1` first and treats a legacy
project as something to update even with no newer tag. The flow's return type and its CLI
consumers change in one task, so no commit leaves the workspace unbuilt.

**Tech Stack:** Rust (agentstacks, agentstacks-cli), toml_edit 0.25.20.

**Content authority:** spec §3.4, D4, D10, F3, E2, E3, E4, §12 rows `types/project/upgrade.rs`, `app/update.rs`, CLI `update` of a 0.x fixture project.

---

## Files that change

```
crates/agentstacks/src/types/project/upgrade.rs  — create: upgrade_project(text) → api 0 → 1 keeping decor; refusal when api is not 0
crates/agentstacks/src/types/project/error.rs    — modify: the "api changed during update" variant
crates/agentstacks/src/types/project/mod.rs      — modify: exports upgrade_project
crates/agentstacks/src/app/update.rs             — modify: reads api 0 or 1; returns the api move with the entries; rewrite applies retag and upgrade
crates/agentstacks-cli/src/show/update.rs        — modify: the api line first; has_newer counts the api move
crates/agentstacks-cli/src/command/update.rs     — modify: consumes the new result; nothing-to-update only with no tag and api 1
crates/agentstacks-cli/tests/fixtures/legacy/    — create: a 0.x project (api = 0, path pack at api = 1, a lock at api = 0)
crates/agentstacks-cli/tests/commands.rs         — modify: update of a 0.x project end to end
```

## Order of work

### Task 1 — The api rewrite rule

**Files:**
- Create `crates/agentstacks/src/types/project/upgrade.rs` — `upgrade_project(text: &str) -> Result<String, ProjectError>`: sets `api` to 1 with the old value's decor; refuses with `ProjectError::ApiChanged` when `api` is not 0 at re-read; four-question module doc
- Modify `crates/agentstacks/src/types/project/error.rs` — `ApiChanged`, displayed `api changed during update (run update again)`, located at `agentstacks.toml`
- Modify `crates/agentstacks/src/types/project/mod.rs` — export

**Change:** The §3.4 step-4 rewrite and its one new re-read refusal. Nothing calls it yet.

**Proof:**
- `api_zero_becomes_one_keeping_comment` — `"# my project\napi = 0 # pinned by hand\n…"` becomes the F3 text byte for byte.
- `api_changed_since_read_is_refused` — text with `api = 1` gives `agentstacks.toml: api changed during update (run update again)`.
- `rest_of_file_unchanged` — every byte outside the `api` value is kept.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): rewrite a 0.x project's api keeping its comments`.

### Task 2 — update moves the api, flow and command together

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the project is read with api 0 or 1 (plan 01's reader); the returned value carries the entries and whether api moves (legacy); the rewrite applies `retag_project`, then `upgrade_project` for a legacy project, on one re-read text, written once
- Modify `crates/agentstacks-cli/src/show/update.rs` — `  api: 0 → 1` as the first stdout line when api moves; `has_newer` is true when api moves
- Modify `crates/agentstacks-cli/src/command/update.rs` — consumes the new result (`command/update.rs:21,47-58` today); `nothing to update` only with no new tag and api 1 (§3.4 step 3)
- Create `crates/agentstacks-cli/tests/fixtures/legacy/` — `agentstacks.toml` (`api = 0`, the claude extension copied at test time under `ext/claude`, path pack `packs/demo` at api 1) and an `agentstacks.lock` at `api = 0` that does not record the demo pack's files, so the plan writes at least one file
- Modify `crates/agentstacks-cli/tests/commands.rs` — the end-to-end cases

**Change:** §3.4 steps 1–5 in the flow and the CLI. A legacy project with no newer tag still rewrites.

**Proof:**
- `legacy_project_upgrades_with_no_newer_tag` (in `app/update.rs`) — `FakeWorkspace` with `api = 0` and path sources only: the result says api moves, and the rewrite writes `api = 1`.
- `legacy_and_new_tags_rewrite_both` — `api = 0` plus a newer `packs-v` tag: both change in one write.
- `current_project_unchanged_behaviour` — `api = 1` with no newer tag: no api move and no write.
- `api_edited_between_read_and_rewrite` — the re-read sees `api = 1`: the `ApiChanged` refusal; nothing written.
- `update_text_puts_api_first` (in `show/update.rs`) — entries plus an api move render `  api: 0 → 1` before `  packs.core: …`.
- `update_legacy_with_no_tag_does_not_say_nothing` (in `command/update.rs`) — a legacy project with path sources prints `  api: 0 → 1` and `agentstacks.toml updated`, not `nothing to update`.
- `update_moves_a_legacy_project` (in `tests/commands.rs`) — `agentstacks update --yes` on the legacy fixture exits 0, `agentstacks.toml` then says `api = 1`, and the lock starts with `api = 1`.
- `plan_refuses_the_legacy_fixture_before_update` (in `tests/commands.rs`) — `agentstacks plan` on the fixture prints `agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)` and exits 1.
- every existing `command/update.rs` test passes.

**Done when:** `cargo make dod` passes. Commit `feat(repo): update moves a 0.x project to api 1`.

---

## Risks

- `update`'s rewrite today is one `retag_project` call. Composing it with `upgrade_project` must keep a single read-then-write, or a concurrent edit between them could be lost. `api_edited_between_read_and_rewrite` guards it.
- An apply with nothing to change writes no lock (D10), so the fixture's lock leaves the demo pack's files unrecorded. That way the plan writes, and the lock assertion holds.

## Proof

- `cargo make dod` is green.
- The api-0 gate of plan 01 ("Kept lines") prints only kept lines.

## Review findings

- Task 1 (reviewer, handoff 20261125-174220-4854/19-reviewer-batchC.md): 0🔴, blocking none; batch gate `cargo make dod` exit 0. The open 🔵 below are now fixed (fix pass, below).
- Task 2 (reviewer, handoff 20261125-174220-4854/22-reviewer-batchD.md): 0🔴, blocking none; batch gate `cargo make dod` exit 0 (1717 lib tests). One re-read, one write, refusal writes nothing, api line first: each holds and has a test. The open 🟡 and 🔵 below are now fixed (fix pass, below).
- Fix pass. Final gates, each run once after the last edit: `cargo make dod` exit 0 (Rust: 2595 passed, 0 failed across 25 test binaries and doctests; lua 651 passed, fingerprint gate 8 cases, pack text gates 4 passed), `cargo make claudevs-check` exit 0, `cargo make opencode-check` exit 0 (S3 to S8b ok), `cargo deny check` exit 0 (advisories ok, bans ok, licenses ok, sources ok), `cargo make book-build` exit 0 with no `[ERROR]` line.
  - fixed: the `ProjectError::ApiChanged` doc said "other than the legacy `0`"; it now says the `api` is missing, not an integer, or another number.
  - fixed: doc lines over 100 columns in upgrade.rs, project/mod.rs and every other `.rs` file this change touched are wrapped. Proof: a scan of `//!` and `///` lines over 100 characters in the files `git diff 41799a9 --name-only` lists finds only the three doctest code lines of hooks/grants.rs:100 and hooks/native/file.rs:74 and :82.
  - fixed: `PROJECT_FILE` was copied in about ten modules; `types::project::PROJECT_FILE` is now the one constant, used by app/{init,update,plan/*}.rs, fs/workspace/project.rs, project/{retag,upgrade}.rs, resolve/key.rs, plan/airsl.rs, and the own-files lists of op/path.rs and lock/entry/written.rs. Proof: `git grep -nE 'const PROJECT_FILE' -- crates` prints only project/file.rs; the test helper in set/testing.rs re-exports it. The test-local `const PROJECT` in the op/airsl.rs tests keeps its own literal.
  - fixed: the TOML-parse closure copied between retag.rs and upgrade.rs is one `parse_document` in the new types/project/document.rs, with its own tests.
  - fixed: `Update::api_moves: bool` is `Update::api: ApiCheck` (the project's `api`, never `Unsupported` because `update` refuses it); `rewrite`, show/update.rs and command/update.rs match `ApiCheck::Legacy`. The text and exit codes are unchanged (the `update_text_puts_api_first` and `update_legacy_with_no_tag_does_not_say_nothing` tests assert the same output).
  - fixed: the test-only `retag` shim is dropped; the tests call `rewrite` and are named `rewrite_*`.
  - fixed: the legacy `0` in show/update.rs is `LEGACY_API` (plan 01).
  - fixed: book/src/ch04-00-cli.md section 7 said "With no new tag, print `nothing to update`"; it now says that applies only when the `api` is already current, and that a 0.x project is moved to `api = 1` (and prints `  api: 0 → 1` first). Proof: `cargo make book-build` exit 0, no `[ERROR]` line.

## Deviations

- 2026-11-25: `upgrade_project` returns `Result<String, Diagnostic>` (as `retag_project` does), not `Result<String, ProjectError>`: the refusal text needs the `agentstacks.toml:` location, and a TOML parse failure needs a Diagnostic. `ApiChanged` is the ProjectError inside it. An extra test, `text_that_is_not_toml_is_refused_at_the_file`, covers the parse refusal.
- 2026-11-25: The flow's rewrite function `app::update::retag` is renamed `rewrite`; it takes the `Update` result and applies retag then upgrade on one re-read text. Its only caller is the CLI's command/update.rs.
