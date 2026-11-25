---
status: approved
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
