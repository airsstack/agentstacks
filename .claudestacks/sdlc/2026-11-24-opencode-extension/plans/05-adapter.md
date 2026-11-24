---
status: approved
created: 2026-11-25
depends-on: [01, 03, 04]
built-on: d54f8422f1ff
---

# OpenCode Adapter Implementation Plan

**Goal:** The opencode extension's adapter turns the `values` and `plan` payloads into the operations of spec §4.3.

**Architecture:** `extensions/opencode/adapter.lua` is the only file calling `agentstacks.*`: it
requires the five `lib/` modules, answers `values` with `lib.values`, and in `plan` writes skills,
commands, agents, native files, the one `agentstacks.js` and, when a lua hook became an entry,
`<root>/translate.lua`. `crates/agentstacks/tests/opencode.rs` runs the shipped extension on the real
host like `tests/claude.rs`: plan fixtures compared with checked-in expected files, and the
translator run with the plan 01 runtime on airsl against recorded payloads.

**Tech Stack:** Lua under airsl 0.1.5; Rust integration tests (`agentstacks` crate).

**Content authority:** spec §4.1 (manifest), §4.3, §4.6 (skipped skill has no command), §4.7 (translator behaviour), §5, §6 (rows owned by the adapter and translator), §8.2.

---

## Files that change

```
extensions/opencode/extension.toml                         — create: the spec §4.1 manifest
extensions/opencode/adapter.lua                            — create: values and plan handlers
crates/agentstacks/tests/opencode.rs                       — create: real-host plan and translator tests
crates/agentstacks/tests/fixtures/opencode/full/           — create: project + pack using every item kind
crates/agentstacks/tests/fixtures/opencode/full/expected/  — create: every file the plan writes
crates/agentstacks/tests/fixtures/opencode/demo/           — create: project with a copy of the demo pack + expected
crates/agentstacks/tests/fixtures/opencode/both/           — create: claude + opencode project + expected
crates/agentstacks/tests/fixtures/opencode/nohooks/        — create: pack with no hooks + expected
crates/agentstacks/tests/fixtures/opencode/endonly/        — create: pack whose only lua hook is on session.end + expected
crates/agentstacks/tests/fixtures/opencode/payloads/       — create: recorded shim payloads
crates/agentstacks/tests/fixtures/opencode/payloads/hooks/ — create: hook modules the translator cases run
crates/agentstacks/tests/fixtures/opencode/payloads/expected/ — create: exact stdout of each translator case
```

## Order of work

### Task 1 — Manifest and file operations

**Files:**
- Create `extensions/opencode/extension.toml` — the spec §4.1 text.
- Create `extensions/opencode/adapter.lua` — `values` handler returning `lib.values.result`; `plan` handler for skills, commands, agents and native files.
- Create `crates/agentstacks/tests/fixtures/opencode/full/` — `agentstacks.toml` (`tools.opencode.path = "extensions/opencode"`, one pack `full`), the pack with: an `any` skill with a `references/` file and a binary file, a `user` skill using `{{var arguments}}`, a skill with a bad native key, an agent with tools, an agent without tools, a native file under `native/opencode/`.
- Create `crates/agentstacks/tests/opencode.rs` — helpers mirroring `tests/claude.rs` (copy fixture and `extensions/opencode` into a temp dir, run `app::plan::plan` with `StdWorkspace` and `AirslAdapter`, compare proposals with `expected/`).

**Change:** Per spec §4.3: `SKILL.md` written with `lib.frontmatter.skill` covering its id; the
command written to `.opencode/commands/<p>-<s>.md` covering the same id; other skill files copied;
agents written to `.opencode/agents/<p>-<a>.md`; native files copied to `.opencode/<rel>`; a skill or
agent whose frontmatter returns a reason is skipped with it, and a skipped skill gets no command.

**Proof:**
- `opencode_manifest_loads` — `Extension::load` accepts `extensions/opencode` with name `opencode`, api 0, no reads.
- `full_pack_writes_skills_commands_agents_and_native_files` — the proposed files equal `full/expected/` byte for byte.
- `skill_with_bad_native_key_is_skipped_without_command` — the skip line reads `native.opencode key "<key>" cannot be written as frontmatter` and no `.opencode/commands/` file for it is proposed.

**Done when:** `cargo test -p agentstacks --test opencode` passes and `cargo make dod` is green. Commit `feat(repo): opencode adapter writes skills, commands and agents`.

### Task 2 — Hooks, shim and translator writes

**Files:**
- Modify `extensions/opencode/adapter.lua` — hooks to entries with `lib.hooks` (`native` = false for the pack's `hooks`, true for its `native.hooks`), `agentstacks.js` with `lib.plugin`, `translate.lua` with `lib.translate`.
- Modify `crates/agentstacks/tests/fixtures/opencode/full/` — add hooks: session.start message with `match = ["startup"]`, session.start lua with `match = ["compact"]`, session.start lua with no `match`, prompt.submit lua, tool.before lua `may_block` with `match = ["write"]`, tool.after lua, session.end message, agent.stop lua, a lua hook whose argv holds `^a$`; a `native/opencode/hooks.toml` with a `tool.execute.before` lua hook with `match = ["shell"]`, a `tool.execute.after` message with `match = []`, a `session.prompt` message hook and a `Notification` hook; expected `agentstacks.js` and `translate.lua`.
- Create `crates/agentstacks/tests/fixtures/opencode/nohooks/` — one skill, no hooks, expected files.
- Create `crates/agentstacks/tests/fixtures/opencode/endonly/` — one pack whose only hook is a lua hook on session.end, expected files.
- Create `crates/agentstacks/tests/fixtures/opencode/demo/` — a copy of `crates/agentstacks-cli/tests/fixtures/project/packs/demo/` with its project and expected files.
- Create `crates/agentstacks/tests/fixtures/opencode/both/` — `claude` and `opencode` tools over one pack, with `extensions/claude` copied in by the test.
- Modify `crates/agentstacks/tests/opencode.rs` — the cases below.

**Change:** Hooks are visited in payload order (packs in order; `hooks` by index, then native hooks by
index); each becomes an entry or a skip with `lib.hooks`' reason. `agentstacks.js` is always written,
covering every hook id that became an entry, sorted. `translate.lua` is written under the payload's
root only when a lua hook became an entry.

**Proof:**
- `full_pack_renders_hooks_into_the_shim` — `agentstacks.js` equals `full/expected/.opencode/plugins/agentstacks.js` (the native `session.prompt` message and the `match = []` native hook render with no `tools`), and the skip lines carry the exact §4.6 reasons for the compact-only, session.end, agent.stop, `^a$` and `Notification` hooks.
- `no_hooks_writes_an_empty_shim_and_no_translator` — `nohooks`: `const HOOKS = [];`, no `translate.lua`.
- `every_lua_hook_skipped_writes_no_translator` — `endonly`: no `translate.lua`, the core still writes `runtime.lua`.
- `demo_pack_plan_matches` — `demo` proposals equal its `expected/`.
- `both_tools_plan_without_clash` — `both`: plan succeeds, each tool has its own `runtime.lua`, `.claude/` and `.opencode/` files both proposed.
- `plan_is_clean_after_apply` — after applying `full`, a second plan proposes nothing to write.

**Done when:** `cargo test -p agentstacks --test opencode` passes and `cargo make dod` is green. Commit `feat(repo): opencode adapter renders hooks into the shim`.

### Task 3 — Translator on the real host

**Files:**
- Create `crates/agentstacks/tests/fixtures/opencode/payloads/` — shim payloads (spec §4.5 "Payload on stdin"): `read.json`, `write.json`, `patch-add-update-write.json` (`match = ["write"]`), `patch-add-update.json` (`match` null), `patch-move.json`, `patch-delete.json`, `subagent-read.json` (with `parent_id`), `session-start.json`, `prompt.json`, `bash.json` (tool `shell`).
- Create `crates/agentstacks/tests/fixtures/opencode/payloads/hooks/` — `ctx.lua` (context naming the event's `tool`, `path`, `agent`, `agent_id` and `prompt`), `blk.lua` (block), `both.lua` (context and block), `none.lua` (returns nil), the shape `tests/claude.rs` uses in `tests/fixtures/claude/payloads/hooks/`.
- Create `crates/agentstacks/tests/fixtures/opencode/payloads/expected/` — one `<case>.out` per case below holding its exact stdout (empty for the no-output cases), written from spec §4.7.
- Modify `crates/agentstacks/tests/opencode.rs` — run `full/expected/.opencode/agentstacks/translate.lua` with the shipped runtime and the hook modules above on `airsl`, as `tests/claude.rs` runs Claude's, comparing stdout with `expected/<case>.out` byte for byte, skipping when airsl is absent unless `AGENTSTACKS_REQUIRE_AIRSL` is set.

**Change:** None in the extension; this proves the §4.7 translator on the real binary.

**Proof:** (each compares stdout with its `expected/<case>.out` byte for byte)
- `translator_read_gives_context` — `ctx` on `read.json`: the context names `tool=read`, `path` as sent, `agent` nil.
- `translator_write_block_passes` — `blk` with `may_block` on `write.json`: the block.
- `translator_patch_splits_per_file` — `ctx` on `patch-add-update-write.json` runs once (the Add); on `patch-add-update.json`, twice, contexts joined by `\n`.
- `translator_patch_move_uses_new_path` — `patch-move.json` gives one edit event for the `Move to` path.
- `translator_patch_delete_only_runs_nothing` — `patch-delete.json`: no output.
- `translator_subagent_names_agent` — `subagent-read.json`: `agent` = the payload's agent, `agent_id` = its `session_id`.
- `translator_session_start_joins_block` — a hook returning context and block on `session-start.json` prints only `context` = `ctx\nblk`.
- `translator_prompt_gives_context` — `prompt.json`: `prompt` is passed through.
- `translator_other_tool_is_silent` — `bash.json`: no output.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test opencode` passes and `cargo make dod` is green. Commit `test(agentstacks): run the opencode translator on airsl`.

---

## Risks

- Expected fixture files are written from the spec's rules, not generated by the code under test;
  a wrong expectation fails loudly rather than passing silently.
- `both` copies `extensions/claude`; a later claude change can move its expected files. They are
  limited to presence checks, not byte comparisons, for the claude side.
- Riskiest step: Task 2's payload order and `covers` sorting, which decide `agentstacks.js` bytes.

## Proof

- `cargo make dod` green; `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test opencode` green.
