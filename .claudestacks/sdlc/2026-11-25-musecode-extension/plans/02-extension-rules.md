---
status: approved
created: 2026-11-25
built-on: c1dbedb148db
---

# Muse Code Extension Rules Implementation Plan

**Goal:** Pure Lua modules decide every rendering rule of the musecode extension.

**Architecture:** Four `lib/` modules under `extensions/musecode/`, each a pure function set (plain
data in, text or tables out), never calling `agentstacks.*` or `require`, each with a colocated
`<module>_test.lua` run by `airsl test` (spec §2). They are copies of the claude extension's
`extensions/claude/lib/values.lua`, `frontmatter.lua`, `hooks.lua` and `settings.lua`, changed as the
spec says; no code is shared across extensions (spec §2, `book/src/ch06-01-extensions.md` §10).

**Tech Stack:** Lua under airsl 0.1.5 (`airsl test extensions`, part of `cargo make lua` in `dod`).

**Content authority:** spec §4.1 (values), §4.2 (frontmatter), §4.4 (hooks), §4.5 (hookfile), §4.6,
§6 (rows owned by `lib/*`), §8.1.

---

## Files that change

```
extensions/musecode/lib/values.lua             — create: root, skill/agent names, var forms
extensions/musecode/lib/values_test.lua        — create: tests for values
extensions/musecode/lib/frontmatter.lua        — create: SKILL.md and agent text, skips F1-F5
extensions/musecode/lib/frontmatter_test.lua   — create: tests for frontmatter
extensions/musecode/lib/hooks.lua              — create: pack hook → Muse Code hook group, or skip H1-H8
extensions/musecode/lib/hooks_test.lua         — create: tests for hooks
extensions/musecode/lib/hookfile.lua           — create: merge rules for the shared .muse/hooks.json
extensions/musecode/lib/hookfile_test.lua      — create: tests for hookfile
```

## Order of work

### Task 1 — `values`

**Files:**
- Create `extensions/musecode/lib/values.lua`, `extensions/musecode/lib/values_test.lua`.

**Change:** `M.result(payload)` returns `{ root = ".muse/agentstacks", skills, agents, vars }` per
spec §4.1: `skills["<p>/<s>"] = { invoke = "/<p>-<s>", dir = ".agents/skills/<p>-<s>" }`,
`agents["<p>/<a>"] = "<p>-<a>"`, and `vars` only for the names the payload lists, with the §4.1
values (`project_dir` `.`, `session_id` `${MUSE_SESSION_ID}`, `arguments` `<arguments>`, `tmpdir`
`${TMPDIR:-/tmp}`, `home` `$HOME`, `airsstack_home` `${AIRSSTACK_HOME:-$HOME/.airsstack}`).
`M.hook_agents(result, pack_name, hook)` as in `extensions/claude/lib/values.lua` (the rendered agent
names of `hook.agents`, nil when absent).

**Proof:** written first, seen failing, then passing —
- `root_is_muse_agentstacks` — `root` is `.muse/agentstacks`.
- `skill_invoke_and_dir` — `core/status` gives `/core-status` and `.agents/skills/core-status`.
- `agent_name_is_pack_dash_agent` — `core/coder` gives `core-coder`.
- `each_var_has_its_form` — each of the six names maps to its §4.1 value exactly.
- `var_not_in_payload_is_absent` — only listed names appear in `vars`.
- `hook_agents_renders_names` — `agents = ["coder"]` in pack `core` gives `{"core-coder"}`; no
  `agents` gives nil.

**Done when:** `airsl test extensions` reports `0 failed` and the Definition of Done is green
(`cargo make dod`). Commit `feat(repo): musecode extension values`.

### Task 2 — `frontmatter`

**Files:**
- Create `extensions/musecode/lib/frontmatter.lua`, `extensions/musecode/lib/frontmatter_test.lua`.

**Change:** Two functions, each returning the text or `nil, reason`. Rendering is
`extensions/claude/lib/frontmatter.lua`'s `render` (spec §4.2): `---\n`, one
`key: <airsstack.json.encode(value)>\n` per key, generated keys first in the §4.2 order, a native
key replacing a generated one of the same name, other native keys after them sorted, `---\n`, the
body.
- `M.skill(name, meta, text)` — keys `name`, `description`, `disable-model-invocation: true` only
  for `invocation = "user"`, `argument-hint` when `meta.argument_hint` is set.
- `M.agent(name, meta, text)` — keys `name`, `description`, `tools` when `meta.tools` is set,
  `effort` when set; never `model`. `tools`: each word in `meta.tools`' order mapped `read`→
  `read_file`, `edit`→`edit_file`, `write`→`write_file`, `grep`→`search`, `glob`→`search`, `shell`→
  `bash`, `skill`→`read_skill`, a name already written not repeated; an empty list is written as the
  literal `[]` (not `encode`'s `{}`).
- Skips, checked in order F1-F5 with the exact reasons of spec §4.2, F2-F5 on the values as written
  (after native replacement; a native `name` or `description` that is not a string is not checked):
  F1 `native.musecode key "<key>" cannot be written as frontmatter`; F2 `musecode skill names are at
  most 128 characters (this one has <n>)`, counted with `utf8.len` (spec §4.2); F3 `musecode agent names are lowercase letter words joined
  by - (got "<name>")` for a name not matching `^[a-z]+(-[a-z]+)*$`; F4 `musecode drops an agent
  whose description is over 4096 bytes (this one has <n>)`; F5 `musecode refuses a SKILL.md over
  262144 bytes (this one has <n>)`, measured on the whole rendered `SKILL.md` text.

**Proof:** written first, seen failing, then passing —
- `skill_any_writes_name_and_description` — exact text for an `any` skill.
- `skill_user_writes_disable_model_invocation` — the third key is `disable-model-invocation: true`.
- `skill_writes_argument_hint` — `argument_hint = "[x]"` gives the line `argument-hint: "[x]"`.
- `agent_core_coder_exact` — the spec §4.2 `core-coder` example, byte exact.
- `agent_without_tools_has_no_tools_key`, `agent_with_empty_tools_writes_literal_list` (line
  exactly `tools: []`), `agent_grep_and_glob_give_one_search`.
- `agent_never_writes_model` — `model = "deep"` writes no `model` key.
- `native_key_replaces_generated` — a native `description` replaces the generated one.
- `f1_bad_native_key`, `f2_name_128_written_129_skipped`, `f3_rejects_digit_underscore_capital`
  (`core-coder2`, `core_x`, `Core-x`), `f3_f4_apply_to_native_values`,
  `f4_description_4096_written_4097_skipped`, `f5_skill_262144_written_262145_skipped` — each skip
  with its exact reason.

**Done when:** `airsl test extensions` reports `0 failed` and the Definition of Done is green.
Commit `feat(repo): musecode extension frontmatter`.

### Task 3 — `hooks`

**Files:**
- Create `extensions/musecode/lib/hooks.lua`, `extensions/musecode/lib/hooks_test.lua`.

**Change:** `M.MARKER = " # agentstacks"`; `M.render(hook, vars, agents)` returns
`{ event, group = { matcher, hooks = { { type = "command", command } } } }` or `nil, reason`, per
spec §4.4:
- Events and matchers by the §4.4 table: `session.start`→`SessionStart` (`match` joined with `|`);
  `session.end`→`SessionEnd`, `prompt.submit`→`UserPromptSubmit`, `agent.stop`→`SubagentStop` (no
  matcher); `tool.before`/`tool.after`→`PreToolUse`/`PostToolUse` (`read`→`Read`, `edit`→`Edit`,
  `write`→`Write`, joined with `|`, `Read|Edit|Write` when absent). A native hook keeps its event and
  its `match` joined with `|` (none when absent or empty).
- Lua command: each argv element quoted by `extensions/claude/lib/hooks.lua`'s `quote_argument`,
  joined with spaces, every `'` replaced by `'\''`, wrapped as `/bin/sh -c '<…>' # agentstacks`; no
  `cd`.
- Message command: `extensions/claude/lib/hooks.lua`'s `message_command`, unchanged.
- Skips H1-H8 in the spec §4.4 order with their exact reasons: H1 (`${MUSE_SESSION_ID}` in text or
  argv), H2 (message on `session.end`), H3 (`agent.stop` with `agents`), H4 (message on `agent.stop`),
  H5 (native event outside the 18 names of §4.4 → `musecode does not run hooks on "<event>"`), H6
  (native message outside `SessionStart`, `SubagentStart`, `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse`, `PostToolUseFailure`), H7 (native non-empty `match` on `UserPromptSubmit` or
  `Stop`), H8 (a `$` left in a Lua argv element after every var form is removed, as
  `extensions/claude/lib/hooks.lua`'s `has_literal_dollar`).

**Proof:** written first, seen failing, then passing —
- `maps_each_neutral_event` — one case per row of the §4.4 events table, matcher included.
- `session_start_clear_compact_matcher_renders` — `match = ["clear", "compact"]` renders with matcher
  `clear|compact` (spec §6).
- `native_match_joined_and_empty_match_has_none`.
- `lua_command_matches_f_r7` — the argv `["touch", "${TMPDIR:-/tmp}/it's \"q\" `b` \\z"]` gives the
  spec §4.4 Lua command byte exact.
- `message_command_matches_f_r8` — the text `MSG-R8 line1⏎line2 it's "q" back\slash` on
  `SessionStart` gives the spec §4.4 message command byte exact.
- `skips_h1` … `skips_h8` — each rule with its exact reason, `<event>`/`<arg>` filled in.
- `skips_follow_table_order` — a hook that breaks H1 and H8 gets the H1 reason.
- `native_session_start_message_renders`, `native_precompact_message_h6`, `native_onsave_h5`,
  `native_user_prompt_submit_empty_match_renders`.

**Done when:** `airsl test extensions` reports `0 failed` and the Definition of Done is green.
Commit `feat(repo): musecode extension hook rules`.

### Task 4 — `hookfile`

**Files:**
- Create `extensions/musecode/lib/hookfile.lua`, `extensions/musecode/lib/hookfile_test.lua`.

**Change:** A copy of `extensions/claude/lib/settings.lua` (spec §4.5): `M.decode(old)` (nil, empty
or whitespace-only → `{}`), `M.edits(decoded, groups_by_event, marker)` (touched events, the old array
minus marked handlers with empty groups dropped, then ours appended; a non-rewritable array → in
`skipped`), `M.should_write(old, edits)`, and `M.skip_reason(event)` returning
`.muse/hooks.json: hooks.<event> holds entries the musecode extension cannot rewrite (a null, an
empty list, or a value that is not a list of hook groups)`.

**Proof:** written first, seen failing, then passing — the cases of
`extensions/claude/lib/settings_test.lua`, each rewritten for this module, plus:
- `skip_reason_names_muse_hooks_json` — the exact reason for `PreToolUse`.
- `user_group_kept_before_ours` — a user `SessionStart` group stays first, ours follows.
- `removed_tool_strips_only_marked` — no groups of ours and an old marked handler → the event keeps
  only the user's handlers, or is removed when none remain.

**Done when:** `airsl test extensions` reports `0 failed` and the Definition of Done is green.
Commit `feat(repo): musecode extension hooks-file merge`.

## Risks

- `airsl test extensions` runs every extension's tests: a name clash between test files across
  extensions is impossible (paths differ), but a slow test here slows `dod`.
- The riskiest step is Task 3's byte-exact commands: the quoting must match spec §4.4 exactly, since
  `F-R7`/`F-R8` proved those bytes in Muse Code.

## Proof

- `airsl test extensions` → `0 failed`; `cargo make dod` green.
