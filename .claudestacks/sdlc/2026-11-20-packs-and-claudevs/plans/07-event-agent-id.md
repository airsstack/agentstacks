---
status: done
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# Event agent id Implementation Plan

**Goal:** Carry the tool's agent instance id on the neutral hook event as `agent_id`.

**Architecture:**
- The neutral event is built by each tool's translator. For claude that is `M.event` in
  `extensions/claude/lib/translate.lua:28-56`. It sets `agent` from `agent_type` and gains
  `agent_id` from the payload's `agent_id` the same way. The core and `runtime.lua` are unchanged.
- Claude's hook input carries `agent_id`, unique per subagent (spec §1;
  `…/cli-and-claude-extension/probes/docs/hooks.md:255,764-768`).
- The translator is shipped as text, and its mapping is tested in Rust: `tests/claude.rs` runs
  fixture hooks through the real runtime, and the echo hook prints a fixed field list (spec §1, §4).
  The `agent_id` proof extends that list. `translate_test.lua` checks only the text and does not
  change.
- `docs/hooks.md` §5 documents the field. It is optional: a tool with no instance id leaves it
  `nil`.

**Tech Stack:** Lua under airsl, Rust integration tests (`crates/agentstacks/tests/claude.rs`),
Markdown.

**Content authority:** spec §1 (`agent_id` rows, translator test row), §4 (translator row, the
`agent_id` test bullet, docs bullet), §11 (claude extension row), §12 (`docs/hooks.md` §5).

---

## Files that change

```
extensions/claude/lib/translate.lua                                   — modify: M.event sets e.agent_id
crates/agentstacks/tests/fixtures/claude/payloads/hooks/echo.lua      — modify: FIELDS ends with agent_id
crates/agentstacks/tests/fixtures/claude/payloads/expected/echo/*.err — modify: each line gains " agent_id=<value>" (8 files)
crates/agentstacks/tests/fixtures/claude/payloads/input/noagent.json  — modify: gains "agent_id":""
crates/agentstacks/tests/claude.rs                                    — modify: rename empty_agent_type_maps_to_nil_agent
crates/agentstacks/tests/fixtures/claude/full/expected/.claude/agentstacks/translate.lua — modify: byte-identical copy of the new translator text
docs/hooks.md                                                         — modify: §5 event table gains the agent_id row
```

## Order of work

### Task 1 — Translate `agent_id`

**Files:**
- Modify the files listed above.

**Change:**
- In `M.event`, beside the `agent` line: when `p.agent_id` is a string and not empty,
  `e.agent_id = p.agent_id`. This holds for every mapped event, not only `agent.stop`.
- `echo.lua`: `"agent_id"` is appended to `FIELDS`.
- Each `expected/echo/*.err` line (`end`, `fork`, `noagent`, `prompt`, `read`, `start`, `subagent`,
  `write`) gains ` agent_id=<value>` at its end: `a1` for `subagent` (the only input carrying an
  id, spec §1), `nil` for the rest.
- `input/noagent.json` gains `"agent_id":""`; its line still shows `agent_id=nil` (an empty id is
  absent).
- `claude.rs`: `empty_agent_type_maps_to_nil_agent` becomes `empty_agent_type_and_id_map_to_nil`.
- The `full` expected `translate.lua` is the new translator text, byte for byte.
- `docs/hooks.md` §5 table, after the `agent` row:
  "`agent_id` | optional; the tool's id for the agent instance, `nil` when the tool has none".

**Proof:**
- `echo_maps_each_event` passes with `subagent` printing `agent_id=a1` and the other five
  `agent_id=nil` (written first: the expected files change before `translate.lua`, and the test is
  seen failing on `subagent`).
- `empty_agent_type_and_id_map_to_nil` passes: `agent=nil` and `agent_id=nil`.
- `fork_source_maps_to_nil` passes with `agent_id=nil`.
- `full_pack_renders_every_item_kind` passes against the new expected `translate.lua`.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test claude` passes and
`cargo make dod` is green. Commit `feat(repo): neutral hook event carries agent_id`.

---

## Risks

- `docs/authoring.md` lists no event fields (`grep -n 'agent_type\|session_id' docs/authoring.md`
  prints nothing), so it needs no edit; it links `hooks.md` §5 for the table.
- A hook that keys state by `agent_id` must still work with `nil` (main agent, or a tool without
  ids). Plan 12's enforce hook keeps `"main"` for `nil`.

## Proof

- `cargo make dod` green.

## Review findings

- No findings on this plan. Verification: red `echo_maps_each_event` (subagent: left `agent_id=nil`, right `agent_id=a1`), 22 others passed → green 23 passed after `translate.lua`; reviewer re-run `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test claude` 23 passed, `cargo make dod` exit 0.

## Deviations

- 2026-11-23 — Claim "the `full` expected `translate.lua` is the new translator text, byte for byte" of `extensions/claude/lib/translate.lua`: that file wraps the shipped text (`local TEXT = [==[ … ]==]`, `M.text()`); copying the whole file failed 10 `--test claude` tests. The expected file is the text between the brackets; its diff against the old expected file is the 3 `agent_id` lines (reviewer: `cmp` matches).
- 2026-11-23 — Ran after plans 05/06 (shared `crates/agentstacks/tests/claude.rs`). Not committed.
