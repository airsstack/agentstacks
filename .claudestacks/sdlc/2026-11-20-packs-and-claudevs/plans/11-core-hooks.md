---
status: approved
created: 2026-11-23
depends-on: [06, 10]
built-on: 0f89fdb3ebb3
---

# Core hooks Implementation Plan

**Goal:** Run core's style, handoff-check, comm-check and reminder hooks as pack hooks.

**Architecture:**
- Each hook is a `[[hooks]]` table in `packs/core/pack.toml`, or in
  `packs/core/native/claude/hooks.toml` for `SubagentHandback`. The text is from spec §6.2.
- Each lua hook file is a module: `return function(event) … end`, returning `nil`,
  `{ context = … }` or `{ block = … }`. It never calls `airsstack.hook.*` or `airsstack.stdio`
  (spec §6.3 "Module shape"; `docs/hooks.md` §4–§5).
- The runtime loads `core.scripts.<name>`. Under `airsl test` the same file is `<name>`. Every file a
  hook loads therefore starts its requires from its own name (spec §3 "The idiom"):
  `local base = (...):match("^(.-)[^.]*$")`, then `require(base .. "<path of the target relative to
  this file's directory, dotted>")`. An entry in `scripts/` requires `base .. "lib.handoff_report"`;
  a file in `scripts/lib/` requires its sibling as `base .. "handoff"`.
- Hook order in `pack.toml` is the §6.2 order. Plan 12 inserts the `rearm` (index 1) and `enforce`
  (index 5) tables, so this plan leaves those two places empty and writes the other eight in order.

**Tech Stack:** Lua under airsl 0.1.5, pack format (TOML), the `agentstacks` binary.

**Content authority:** spec §1 (hook rows: SessionEnd, PreToolUse deny, SubagentHandback, style,
cross-pack `{{root core}}`), §3 (idiom), §6.2 (core and native hook tables), §6.3 rows H3, H5,
H6, H7, H9–H14 and "Module shape", §6.5 (`style_test` row, new unit tests for `style`,
`handoff_check_hook`, `comm_check_hook`), §8 "Hooks", §9 (handback, style, SessionEnd rows).

---

## Files that change

```
packs/core/pack.toml                              — modify: hook tables 0, 2, 3, 4, 6, 7, 8, 9 of §6.2
packs/core/native/claude/hooks.toml               — create: §6.2 native text
packs/core/scripts/style.lua                      — create: module from P/claudestacks/scripts/style.lua
packs/core/scripts/lib/style.lua                  — create: carried; its comment at :34 no longer names the env var
packs/core/scripts/style_test.lua                 — create: moved; path and args cases
packs/core/scripts/handoff_check_hook.lua         — create: module from the plugin script
packs/core/scripts/handoff_check_hook_test.lua    — create: new
packs/core/scripts/comm_check_hook.lua            — create: module from the plugin script
packs/core/scripts/comm_check_hook_test.lua       — create: new
packs/core/scripts/lib/handoff_report.lua         — modify: idiom; file_from_payload keyed by the neutral event
packs/core/scripts/handoff_report_test.lua        — modify: file_from_payload cases take neutral events
packs/core/scripts/lib/comm_report.lua            — modify: idiom; hook_output keyed by the neutral event
packs/core/scripts/comm_report_test.lua           — modify: hook_output cases by neutral event
```

## Order of work

### Task 1 — Reminder message hooks

**Files:**
- Modify `packs/core/pack.toml`.

**Change:** Append the §6.2 `message` tables:
- `session.start` `match = ["startup", "resume", "clear"]` with the `{{skill snapshot-load}}`
  reminder text;
- `session.end` with the `{{skill snapshot-save}}` reminder text.

Each text is byte for byte §6.2. The airsl-missing STATUS/FIX text of the plugin preflight is not
carried (spec §6.3 H3).

**Proof:**
- In a temp git project with copies of `packs/core` and `extensions/claude`, `agentstacks init
  --tool ./extensions/claude --pack ./packs/core --yes` exits 0. `.claude/settings.json` holds a
  `SessionStart` group with matcher `startup|resume|clear` whose command echoes the reminder with
  `/core-snapshot-load`.
- `agentstacks plan` lists the skip `core/hook/1: claude shows no hook output to the model at
  session end`. It becomes `core/hook/2` after Task 2 inserts the style `session.start` table, and
  `core/hook/3` after plan 12 inserts `rearm` (spec §1, `probes/manifests-results.md:271,275`).

**Done when:** both checks pass and `cargo make dod` is green. Commit
`feat(repo): core reminder hooks`.

### Task 2 — Style hook as a module

**Files:**
- Create `packs/core/scripts/style.lua`, `scripts/lib/style.lua`, `scripts/style_test.lua`.
- Modify `packs/core/pack.toml`.

**Change:**
- `style.lua` returns `function(event)`. The protocol path is `event.args[1]`.
  - On `session.start` it always returns `{ context = rules }`.
  - On `prompt.submit` it returns `{ context = rules }` unless `event.args[2]` is `"false"` or
    `"0"`, in which case it returns `nil` (as `lib/style.lua:37-43` reads the value today).
  - It reads no environment variable. `--turn` is gone, because `event.event` names the event
    (spec §6.3 H5, H7).
- The `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT` read (`P/claudestacks/scripts/style.lua:20`) goes. The
  comment at `lib/style.lua:34` that names that variable says "the `style_reinject` option
  (args[2])". The usage block at `style.lua:5-9`, an `airsl run` command line, is dropped: a module
  has no command line (spec §6.4 comment row).
- `pack.toml` gains the two style tables of §6.2:
  - on `session.start`: `args` is the protocol path only;
  - on `prompt.submit`: `args` are the protocol path, then `{{option style_reinject}}`.

  Both grant `read = ["{{skill_dir discuss}}"]`.
- `style_test.lua:69`: the path becomes `packs/core/skills/discuss/references/protocol.md`, and the
  env case becomes the `args[2]` case.

**Proof:**
- `session_start_always_returns_the_rules`: `{event="session.start", args={<protocol>}}` gives
  `{ context = <rules> }`.
- `session_start_ignores_a_false_option`: with `args={<protocol>, "false"}` it still gives the rules.
- `prompt_submit_with_false_returns_nil`: `args={<protocol>, "false"}` gives `nil`; `"0"` gives
  `nil`.
- `prompt_submit_with_true_returns_the_rules`: `args={<protocol>, "true"}` gives the rules.
- The moved `style_test` cases pass with the new path.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): core style hook as a pack module`.

### Task 3 — Handoff and comm checks as modules

**Files:**
- Create `packs/core/scripts/{handoff_check_hook,comm_check_hook}.lua` and their `_test.lua`.
- Modify `scripts/lib/handoff_report.lua`, `scripts/lib/comm_report.lua`, `scripts/comm_report_test.lua`,
  `packs/core/pack.toml`.
- Create `packs/core/native/claude/hooks.toml`.

**Change:**
- Both hook files return `function(event)`:
  - `tool.after` reads the report at `event.path`, relative to `event.cwd`;
  - `agent.stop` reads `event.last_message`;
  - native (`event.payload ~= nil`) reads `event.payload.tool_input.message` and
    `event.payload.cwd`.

  A violation returns `{ block = reason }`, with the reason text the plugin printed; otherwise the
  hook returns `nil` (spec §6.3 H9–H14).
- `lib/comm_report.lua` `hook_output` switches on `event.event` (`tool.after`, `agent.stop`) or on
  the native case, in place of `payload.hook_event_name`. It returns the block reason, not a Claude
  output shape.
- `lib/handoff_report.lua` `file_from_payload` (`:362-378`) takes the event: `tool.after` →
  `event.path`; `agent.stop` → `event.last_message`; native → `event.payload.tool_input.message`.
  The 10 `file_from_payload_*` cases in `handoff_report_test.lua` (`:365-441`) pass neutral events
  in place of Claude payloads, each asserting the same file it found before.
- `lib/handoff_report.lua:11` `local env = airsstack.env` moves inside the functions that read
  the environment, which read it when called (spec §6.3 "Environment"). A test that needs an env
  value swaps `airsstack.env` for a fake table and restores it.
- `lib/handoff_report.lua` and `lib/comm_report.lua` take the §3 idiom for their requires
  (`base .. "handoff"`, `base .. "handoff_report"`). The entries require `base ..
  "lib.handoff_report"` and `base .. "lib.comm_report"`.
- `pack.toml` gains the §6.2 tables:
  - two `tool.after` `match = ["write"]` tables;
  - `agent.stop` `agents = ["coder", "explorer", "reviewer"]` running `handoff_check_hook.lua`;
  - `agent.stop` with no `agents` running `comm_check_hook.lua`.

  All four are `may_block = true` with `env = ["HOME", "TMPDIR"]` and `read = ["/"]`.
- `native/claude/hooks.toml` is the §6.2 native text: two `PreToolUse` `^SubagentHandback$` tables.

**Proof:**
- `handoff_check_hook_test.lua`:
  - `tool_after_bad_report_blocks`;
  - `agent_stop_bad_report_blocks`;
  - `native_handback_bad_report_blocks`;
  - `good_report_returns_nil`.

  Each "bad" case uses a report that `lib/handoff_report` rejects in `handoff_report_test.lua`, and
  asserts `{ block = <that reason> }`.
- `comm_check_hook_test.lua` has the same four cases for the comm check.
- `comm_report_test.lua`'s `hook_output` cases assert by neutral event.
- After `init` in a temp project, the rendered `SubagentStop` command gets a `SubagentStop` payload
  on stdin whose `last_assistant_message` is a bad report, with `agent_type: "core-coder"`. Run
  with `CLAUDE_PROJECT_DIR` set to the project, it prints `{"decision":"block","reason":…}`. This
  shows the idiom loads `core.scripts.lib.handoff_report` under the runtime.
- `settings.json` holds the `SubagentStop` matcher `core-coder|core-explorer|core-reviewer` and a
  `PreToolUse` group with matcher `^SubagentHandback$`.

**Done when:** `cargo make lua` passes, the piped run prints the block, and `cargo make dod` is
green. Commit `feat(repo): core handoff and comm checks as pack hooks`.

---

## Risks

- The riskiest step is Task 3's library switch from Claude event names to neutral ones. Every
  caller of `hook_output` is in `comm_check_hook.lua`. `grep -rn hook_output packs/core` lists them
  before the change.
- A hook that errors exits 0 under `--fail-open` (spec §8). A broken module would pass silently in
  Claude, so the Task 3 piped run is the runtime check.
- `pack.toml` is valid after each task, because a table refers only to files that exist by then.

## Proof

- `cargo make dod` green. A core-only `init` writes the `SessionStart`, `UserPromptSubmit`,
  `PostToolUse`, `SubagentStop` and `PreToolUse` (`^SubagentHandback$`) groups, and the plan has one
  skip.
