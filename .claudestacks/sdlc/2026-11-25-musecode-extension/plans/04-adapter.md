---
status: done
created: 2026-11-25
depends-on: [02, 03]
built-on: c1dbedb148db
---

# Muse Code Adapter Implementation Plan

**Goal:** The shipped musecode extension proposes Muse Code project operations on the real host.

**Architecture:** `extensions/musecode/extension.toml` and `adapter.lua`, the only file that calls
`agentstacks.*`, wiring the `lib/` modules of plans 02 and 03 (spec §2). The adapter follows
`extensions/claude/adapter.lua` with Muse Code's paths and the shared file `.muse/hooks.json`
(spec §4.3, §4.5). It is covered by `crates/agentstacks/tests/musecode.rs`, a copy of the
`tests/claude.rs` / `tests/opencode.rs` pattern over `tests/common` (spec §8.2).

**Tech Stack:** Lua under airsl 0.1.5; Rust integration tests (`cargo test -p agentstacks --test
musecode`), part of `cargo make dod`.

**Content authority:** spec §4.1 (manifest), §4.3, §4.5, §4.6, §4.7 (where the text is written), §5,
§6, §8.2.

---

## Files that change

```
extensions/musecode/extension.toml                         — create: the manifest of spec §4.1
extensions/musecode/adapter.lua                            — create: values + plan → operations
crates/agentstacks/tests/musecode.rs                       — create: the extension on the real host
crates/agentstacks/tests/fixtures/musecode/<case>/…        — create: fixture projects and expected trees
crates/agentstacks/tests/fixtures/musecode/payloads/…      — create: input payloads (cwd /p), hook modules, expected stdout
crates/agentstacks/tests/common/mod.rs                     — modify: the module doc names musecode.rs and "musecode"
```

## Order of work

### Task 1 — Manifest, skills, agents and native files

**Files:**
- Create `extensions/musecode/extension.toml` — the spec §4.1 text exactly (`name = "musecode"`,
  `reads = [".muse/hooks.json"]`).
- Create `extensions/musecode/adapter.lua` — the `values` handler and the item part of `plan`.
- Create `crates/agentstacks/tests/musecode.rs` — the test binary, `TOOL = "musecode"`, helpers as
  in `tests/opencode.rs` (`extension_dir`, `fixture`, `project`, `planned`).
- Create `crates/agentstacks/tests/fixtures/musecode/demo/` — the demo pack copied from
  `crates/agentstacks-cli/tests/fixtures/project/packs/demo/`, as `tests/opencode.rs`
  `demo_pack_plan_matches` copies it; `expected/` holds every file the plan writes for it.
- Create `crates/agentstacks/tests/fixtures/musecode/full/` — a project whose pack has a skill with
  `invocation = "any"` and an extra file, a skill with `invocation = "user"` and `argument_hint`, an
  agent with `tools` and `effort`, an agent with `tools = []`, a `native/musecode/` file, and hooks
  (filled in by Task 2); `expected/` holds every written file.
- Modify `crates/agentstacks/tests/common/mod.rs` — its `//!` doc lists `musecode.rs` and the tool
  name `"musecode"` beside `claude` and `opencode`.

**Change:** `values` returns `lib.values` `M.result(payload)`, kept for `plan`. `plan`, per pack: each
skill → `lib.frontmatter` `M.skill`; `write` `<dir>/SKILL.md` covering the `SKILL.md` id and `copy`
each other file next to it, or one `skip` of the skill group with the reason. Each agent →
`M.agent`; `write` `.agents/agents/<p>-<a>.md` covering the agent id, or `skip`. Each native file →
`copy` to `.agents/<rel>` (spec §4.3).

**Proof:** written first, seen failing, then passing —
- `musecode_manifest_loads` — `Extension::load` of `extensions/musecode` gives name `musecode`,
  `api` 0 and the one `reads` entry `.muse/hooks.json`.
- `demo_pack_plan_matches` — the plan of the demo pack, without `runtime.lua`, equals
  `demo/expected`, with no skip line (the shape of `tests/opencode.rs` `demo_pack_plan_matches`).
- `full_pack_renders_every_item_kind` — the plan's written paths and bytes equal `full/expected`
  (skills under `.agents/skills/<p>-<s>/`, agents under `.agents/agents/`, the native file under
  `.agents/`), and its skip lines equal the expected list.

**Done when:** `cargo test -p agentstacks --test musecode` passes and the Definition of Done is green.
Commit `feat(repo): musecode extension adapter for skills, agents and native files`.

### Task 2 — Hooks into the shared `.muse/hooks.json` and `translate.lua`

**Files:**
- Modify `extensions/musecode/adapter.lua` — the hook part of `plan`.
- Modify `crates/agentstacks/tests/musecode.rs`.
- Modify `crates/agentstacks/tests/fixtures/musecode/full/` — hooks on every neutral event (one lua,
  one message, a `tool.before` with `match`, an `agent.stop` with `agents`, a `session.end` message)
  and one native hook; `expected/.muse/hooks.json` and `expected/.muse/agentstacks/translate.lua`.
- Create fixtures `hooksfile/` (an existing `.muse/hooks.json` with a user `SessionStart` group and an
  earlier marked handler of ours), `removed/` (the tool removed from `agentstacks.toml` with a
  locked project), `nohooks/` (no hook, no hooks file), `badjson/` (`.muse/hooks.json` with a
  trailing comma), `nullevent/` (`hooks.PreToolUse` is `null`), `nolua/` (message hooks only), `allskipped/` (one Lua hook, an `agent.stop` with `agents`),
  `both/` (`claude` and `musecode` listed), each with `expected/` where the test compares trees.

**Change:** As `extensions/claude/adapter.lua`'s `pack_hooks`, `render_hooks` and
`propose_settings`, with `lib.hooks` and `lib.hookfile` and the path `.muse/hooks.json` (spec §4.5
steps 1-4): neutral hooks then native hooks per pack in payload order; each rendered or skipped;
the old file read, checked with `agentstacks.json.edit(old, {"hooks", "agentstacks-check"}, nil)`,
merged, its non-rewritable events' hooks skipped with `lib.hookfile` `M.skip_reason`, written when it
existed or there is an edit, `covers` sorted. When at least one Lua hook is covered by that write,
`write` `<root>/translate.lua` with `lib.translate` `M.text()`.

**Proof:** written first, seen failing, then passing —
- `full_pack_renders_every_item_kind` (extended) — `.muse/hooks.json` and `translate.lua` equal the
  expected files; skip lines for H2 and H3 with their spec §4.4 reasons.
- `existing_hooks_file_keeps_user_group_and_replaces_marked` — the user group stays first, the old
  marked handler is gone, ours follow.
- `removed_tool_strips_only_marked_handlers` — the user's handlers stay, ours are removed, the
  locked files are deleted.
- `hooks_file_absent_and_no_hooks_proposes_no_write` — no `.muse/hooks.json` and no `translate.lua`
  step.
- `invalid_hooks_json_refuses_the_plan` — the refusal starts `musecode/adapter.lua:` and holds
  `json.edit: trailing comma` (the core's serde_json message, `book/src/ch06-01-extensions.md:421`).
- `unrewritable_event_skips_our_hook` — our `PreToolUse` hook is skipped with the §4.5 reason; the
  user's `null` stays.
- `no_lua_hook_writes_no_translate` — message hooks only: hooks file written, no `translate.lua`.
- `every_lua_hook_skipped_writes_runtime_but_no_translate` — a pack whose only Lua hook is an
  `agent.stop` with `agents` (skipped, H3): the core still writes `.muse/agentstacks/runtime.lua`, no
  `translate.lua` (spec §6).
- `both_tools_plan_without_clash` — the plan proposes both tools' paths (`.claude/…` and
  `.agents/…`, `.muse/…`) and is not refused.

**Done when:** `cargo test -p agentstacks --test musecode` passes and the Definition of Done is green.
Commit `feat(repo): musecode extension hooks`.

### Task 3 — Translator on recorded payloads

**Files:**
- Modify `crates/agentstacks/tests/musecode.rs`.
- Create `crates/agentstacks/tests/fixtures/musecode/payloads/`, in the layout of
  `crates/agentstacks/tests/fixtures/claude/payloads/`:
  - `input/` — payloads made from the stdin part of `probes/raw/` with `cwd` replaced by `/p` and
    every other path that holds the scratch directory shortened the same way, as the claude
    fixtures do: `start.json` (SessionStart `startup`), `prompt.json`, `read.json` (PreToolUse
    `read_file`), `edit.json` (PreToolUse `edit_file`), `write.json` (PreToolUse `write_file`, path
    `b.txt`), `postread.json` and `postedit.json` (PostToolUse), `stop.json` (SubagentStop); and
    hand-made `fork.json` (SessionStart `source` `fork`), `emptypath.json` (PreToolUse `write_file`
    with `path` `""`), `emptycwd.json` (PreToolUse `write_file` with `cwd` `""`), `bash.json`
    (PreToolUse `bash`), `end.json` (SessionEnd).
  - `hooks/` — the claude fixtures' hook modules, same names and roles: `echo.lua` (returns the
    neutral event as context), `ctx.lua` (a context), `blk.lua` (a block), `both.lua` (a context and
    a block), `none.lua` (nil).
  - `expected/` — the stdout of each test's run, written from spec §4.7.

**Change:** Tests only, in the pattern of `tests/claude.rs`'s translator tests: each runs the
`translate.lua` of `full/expected` with the core runtime on the real `airsl`, one hook module and one
input, and compares stdout with its expected file.

**Proof:** written first, seen failing, then passing —
- `echo_maps_each_event` — `echo.lua` over each `input/` file gives the neutral event of spec §4.7
  (`write.json` gives `path` `/p/b.txt`; `bash.json` and `end.json` give no tool event and no
  output respectively, as the translator defines).
- `pre_tool_use_block_prints_deny_with_absolute_path` (`blk.lua`, `write.json`),
  `pre_tool_use_context_prints_additional_context` (`ctx.lua`, `read.json`),
  `pre_tool_use_context_and_deny_prints_both` (`both.lua`, `write.json`),
  `post_tool_use_block_prints_decision_block` (`blk.lua`, `postread.json`),
  `post_tool_use_context_prints_additional_context` (`ctx.lua`, `postedit.json`),
  `user_prompt_submit_block_prints_decision_block` (`blk.lua`, `prompt.json`),
  `session_start_context_prints_additional_context` (`ctx.lua`, `start.json`),
  `session_start_with_context_and_block_joins_them` (`both.lua`, `start.json`),
  `subagent_stop_block_prints_decision_block` (`blk.lua`, `stop.json`),
  `subagent_stop_context_only_prints_nothing` (`ctx.lua`, `stop.json`),
  `session_end_prints_nothing` (`ctx.lua`, `end.json`), `bash_tool_is_not_run` (`blk.lua`,
  `bash.json`), `fork_source_maps_to_nil` (`echo.lua`, `fork.json`), `empty_path_passes_through`
  (`echo.lua`, `emptypath.json`), `empty_cwd_passes_path_through` (`echo.lua`, `emptycwd.json`),
  `nil_result_prints_nothing` (`none.lua`, `read.json`) — each against its expected stdout (spec
  §4.7 table and §6 rows).

**Done when:** `cargo test -p agentstacks --test musecode` passes (with `airsl` on `PATH`) and the
Definition of Done is green. Commit `test(agentstacks): musecode translator on recorded payloads`.

## Risks

- The plan tests of packs with Lua hooks need `airsl`; they skip without it like `tests/opencode.rs`
  (`common::skip_without_airsl`), so a machine without airsl runs less.
- Riskiest step: Task 2's merge wiring; the `hooksfile`, `removed` and `nullevent` fixtures guard it.
- Expected trees are written from the rules of plans 02-03, not generated by the code under test.

## Proof

- `cargo test -p agentstacks --test musecode` passes; `cargo make dod` green.

## Review findings

One reviewer pass per batch (Task 1, Task 2, Task 3); verdicts: Task 1 compliant-with-amendments,
`blocking:` demo fixture restore (🟡); Task 2 and Task 3 compliant-with-amendments, `blocking: none`.
0🔴 in every pass.

- 🟡 fixed (T1) — demo fixture had its two `[[hooks]]` removed; restored as a byte copy of
  `crates/agentstacks-cli/tests/fixtures/project/packs/demo` (`diff -r` clean) with the expected
  `.muse/hooks.json`, `translate.lua` and `guard.lua`, compared "without runtime.lua".
- 🟡 fixed (T1) — lint-attribute comment/reasons on `pub mod common;` described future work; the
  attributes are now gone (no `allow`/`expect` left in `musecode.rs`).
- 🔵 fixed (T1) — over-100-column `//!` lines in `common/mod.rs` and `musecode.rs`; skipped skill `odd`
  gained `references/x.md` (guards "skipped whole"); adapter header matches the file.
- 🔵 fixed (T2) — module doc lists the fixtures without expected trees; one doc per `RUNTIME` /
  `TRANSLATE`; within-event order guarded (native `PreToolUse` beside the neutral one in `full`);
  sorted `covers` asserted; adapter.lua lines 15, 62, 83, 104 wrapped to ≤100 columns.
- 🔵 fixed (T3) — `bash.json` added to `echo_maps_each_event`; the translator harness duplicated
  from `claude.rs` moved to the shared `crates/agentstacks/tests/translator/run.rs`, used by
  `claude.rs` and `musecode.rs` (no assertion changed).
- Verified: `cargo test -p agentstacks --test musecode --test claude` → musecode `28 passed`, claude
  `23 passed`, 0 failed; `cargo make dod` → rc 0, Lua `651 passed, 0 failed (25 files)`,
  fingerprint gate 8 cases passed, pack text gates 4 passed. Every named test was first seen failing
  (`not an extension` / `hook got no operation` / missing payload fixtures).

## Deviations

- 2026-11-26 — No commit made: the execute instruction did not ask for one. Board row at start was
  clean (`execute 04`).
- 2026-11-26 — Task 1 temporarily dropped the demo hooks (the core refuses an item with no operation:
  `musecode/adapter.lua: demo/hook/0 got no operation (copy it, write it, or skip it with a reason)`);
  Task 2 restored them, so the final fixture is the plan's byte copy.
- 2026-11-26 — Review fixes were folded into the next task's coder (same files, one owner per file);
  Task 3's nits went to a fresh fix coder.
- 2026-11-26 — `allskipped/expected` includes `.muse/agentstacks/core/hooks/enforce.lua`: the core
  copies a pack's hook module even when every hook is skipped (seen when the hand-written tree omitted
  it).
- 2026-11-26 — Expected `translate.lua` (demo, full) is the `TEXT` of `lib/translate.lua`, as opencode
  does; the translator text is itself the rule. Every other expected file is hand-written from rules.
- 2026-11-26 — Payload fixtures: `blk.lua` appends the event path to its reason so the deny test can
  show the absolute path; `stop.json` keeps the recording but cuts `last_assistant_message` to its
  first sentence; `end.json` is the recorded SessionEnd, not hand-made; `echo.lua` is claude's echo
  (stderr `key=value`, returns nil), so the plan's "returns the neutral event as context" reads as
  claude's role.
- 2026-11-26 — `covers` is read through a `Recording` adapter wrapper in `musecode.rs` (a plan `Step`
  keeps no `covers`).
- 2026-11-26 — The shared translator harness lives in `crates/agentstacks/tests/translator/`, not
  `common/hook.rs`: `common` is compiled into `opencode.rs` too, whose harness differs, and the unused
  items would fail clippy `-D warnings` there. `opencode.rs` got an import-only edit.
