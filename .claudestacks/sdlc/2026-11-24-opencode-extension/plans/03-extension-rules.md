---
status: done
created: 2026-11-25
built-on: d54f8422f1ff
---

# OpenCode Extension Rules Implementation Plan

**Goal:** Pure Lua modules decide every rendering rule of the opencode extension.

**Architecture:** Three `lib/` modules under `extensions/opencode/`, each a pure function set (plain
data in, text or tables out), never calling `agentstacks.*` or `require`, each with a colocated
`<module>_test.lua` run by `airsl test`. They follow the claude extension's shape
(`extensions/claude/lib/values.lua`, `frontmatter.lua`, `hooks.lua`); no code is shared across
extensions.

**Tech Stack:** Lua under airsl 0.1.5 (`airsl test`, part of `cargo make lua`).

**Content authority:** spec §4.1 (values), §4.2 (frontmatter), §4.4 and §4.6 (hooks and skips), §8.1.

---

## Files that change

```
extensions/opencode/lib/values.lua            — create: root, skill/command/agent names, var forms
extensions/opencode/lib/values_test.lua       — create: tests for values
extensions/opencode/lib/frontmatter.lua       — create: SKILL.md, agent and command text
extensions/opencode/lib/frontmatter_test.lua  — create: tests for frontmatter
extensions/opencode/lib/hooks.lua             — create: hook → plugin entry, or skip reason
extensions/opencode/lib/hooks_test.lua        — create: tests for hooks
```

## Order of work

### Task 1 — `values`

**Files:**
- Create `extensions/opencode/lib/values.lua`, `extensions/opencode/lib/values_test.lua`.

**Change:** `M.result(payload)` returns `{ root = ".opencode/agentstacks", skills, agents, vars }`
per spec §4.1: `skills["<p>/<s>"] = { invoke = "/<p>-<s>", dir = ".opencode/skills/<p>-<s>" }`,
`agents["<p>/<a>"] = "<p>-<a>"`, and `vars` only for the names the payload lists, from the §4.1
table (`project_dir`, `session_id`, `arguments`, `tmpdir`, `home`, `airsstack_home`).

**Proof:**
- `root_is_opencode_agentstacks` — `root` is `.opencode/agentstacks`.
- `skill_invoke_and_dir` — `core/status` gives `/core-status` and `.opencode/skills/core-status`.
- `agent_name_is_pack_dash_agent` — `core/coder` gives `core-coder`.
- `each_var_has_its_form` — each of the six names maps to its §4.1 value exactly.
- `var_not_in_payload_is_absent` — only listed names appear in `vars`.

**Done when:** `airsl test extensions` reports `0 failed` and `cargo make dod` is green. Commit `feat(repo): opencode extension values`.

### Task 2 — `frontmatter`

**Files:**
- Create `extensions/opencode/lib/frontmatter.lua`, `extensions/opencode/lib/frontmatter_test.lua`.

**Change:** Three functions, each returning the text, or `nil` and the reason
`native.opencode key "<key>" cannot be written as frontmatter` for a native key that is not
`^[A-Za-z0-9_-]+$`. Rendering follows spec §4.2: `---\n`, one `key: <airsstack.json.encode(value)>\n`
per key, `---\n`, the body; generated keys in the listed order, a native key replaces a generated one
of the same name, other native keys follow sorted. A top-level value that is an empty table,
generated or native, is written `[]` (spec §4.2); nested values are written as `encode` gives them.
- `M.skill(name, meta, text)` — keys `name`, `description`, then `disable-model-invocation: true` only for `invocation = "user"`; `argument_hint` is never written.
- `M.agent(meta, text)` — keys `description`, `mode: "subagent"`, `permissions`: absent `tools` → the literal text `[]` (`airsstack.json.encode` writes an empty table as `{}`, spec §4.2); otherwise the encoded list of a deny-all rule then one allow per mapped tool in written order without duplicates (read→`read`, write→`edit`, edit→`edit`, grep→`grep`, glob→`glob`, shell→`shell`, skill→`skill`), each rule with keys `action`, `effect`, `resource` as `encode` sorts them; `tools = []` → only the deny-all rule. No `name`, `model` or `effort` key.
- `M.command(id, meta)` — frontmatter with `description` only (no native keys), then the two body lines of spec §4.2 with `id` filled in, ending in `\n`.

**Proof:**
- `skill_any_writes_name_and_description` — exact text for an `any` skill.
- `skill_user_writes_disable_model_invocation` — the third key is `disable-model-invocation: true`.
- `skill_drops_argument_hint` — no `argument-hint` key.
- `agent_without_tools_has_empty_permissions` — the line is exactly `permissions: []`.
- `agent_with_empty_tools_denies_all` — the line is exactly `permissions: [{"action":"*","effect":"deny","resource":"*"}]`.
- `agent_tools_map_to_permissions` — `["read","write","edit","grep"]` gives the line `permissions: [{"action":"*","effect":"deny","resource":"*"},{"action":"read","effect":"allow","resource":"*"},{"action":"edit","effect":"allow","resource":"*"},{"action":"grep","effect":"allow","resource":"*"}]`.
- `agent_writes_no_model_or_effort` — `model` and `effort` in `meta` produce no key.
- `native_key_replaces_generated` — `native.description` wins.
- `empty_native_table_is_written_as_a_list` — an agent with native `permissions = {}` gets the line `permissions: []`; a skill with an empty native list gets `<key>: []`.
- `bad_native_key_gives_reason` — key `a b` returns `nil` and the exact reason.
- `command_text_for_core_discuss` — byte-exact text for `core-discuss` with a description holding a `"`.

**Done when:** `airsl test extensions` reports `0 failed` and `cargo make dod` is green. Commit `feat(repo): opencode extension frontmatter`.

### Task 3 — `hooks`

**Files:**
- Create `extensions/opencode/lib/hooks.lua`, `extensions/opencode/lib/hooks_test.lua`.

**Change:** `M.entry(hook, vars, native)` returns a plugin entry table or `nil` and a skip reason,
per spec §4.4 and §4.6. `native` is true when the adapter took the hook from the pack's
`native.hooks` list; the event name never decides it (spec §4.4). Entry keys: `on`, `tools` (tool
events only), `match` (neutral tool events only), `command` (lua: each argv element in double quotes
with `\`, `"` and `` ` `` escaped, joined by spaces) or `text` (message). Neutral
`tool.before`/`tool.after` map `match` to OpenCode tools (read→`read`; edit→`edit`,`patch`;
write→`write`,`patch`; none → all four); native `tool.execute.before`/`tool.execute.after` keep a
non-empty `match` as `tools`, and no `tools` key for none or `[]`; `prompt.submit`, `session.start`
and native `session.prompt` carry neither. `tools` and `match` are never written as empty lists.
Skip rules in the §4.6 table order, each only for the hooks its "Applies to" column names, with its
exact reason: (neutral) session.end; (neutral) agent.stop; (neutral) session.start without `startup`
in `match`; `may_block` lua on neutral `prompt.submit` or native `session.prompt`; (native)
`session.prompt` with a non-empty `match`; (native) any event but the three; (both)
`${OPENCODE_SESSION_ID}` in text or argv; (both) a literal `$` left after removing the var forms
present in `vars`, longest first.

**Proof:**
- `tool_before_write_maps_to_write_and_patch` — `match = ["write"]` gives `tools = {"write","patch"}` and `match = {"write"}`.
- `tool_after_without_match_runs_for_all_file_tools` — `tools = {"read","edit","write","patch"}`, no `match`.
- `edit_and_write_share_patch_once` — `match = ["edit","write"]` gives `edit, patch, write` without a second `patch`.
- `prompt_and_session_start_have_no_tools` — no `tools`, no `match`.
- `native_tool_hook_keeps_its_match_as_tools` — `tool.execute.before` with `["shell"]` gives `tools = {"shell"}`.
- `native_tool_hook_without_match_has_no_tools` — native `tool.execute.after` with no `match`, and native `tool.execute.before` with `match = {}`: no `tools` key, no `match` key.
- `native_prompt_message_renders` — native `session.prompt` message hook: `on = "session.prompt"`, `text` set, no `tools`.
- `native_prompt_lua_renders` — native `session.prompt` lua hook, `may_block = false`: `command` set, no `tools`; with `match = {}` it still renders.
- `native_flag_decides_not_the_event_name` — a hook on `prompt.submit` with `native = true` is skipped with the "no native hook event" reason naming `prompt.submit`; the same hook with `native = false` renders.
- `native_hooks_on_neutral_names_get_the_native_reason` — with `native = true`: a message on `session.end`, a lua hook on `agent.stop`, a lua hook on `session.start` with no `match`, and a `may_block` lua hook on `prompt.submit` each get spec §4.6 rule 6 naming its event (never rules 1-4).
- `session_start_without_match_renders` — neutral `session.start` lua hook with no `match`: `on = "session.start"`, no `tools`.
- `message_hook_gives_text` — `text` set, no `command`.
- `lua_hook_command_is_quoted` — an argv element holding `"`, `\` and a backtick is escaped as spec §4.4 says.
- One test per skip row, asserting the exact reason: `skips_session_end`, `skips_agent_stop`, `skips_session_start_without_startup`, `skips_may_block_prompt`, `skips_native_prompt_with_match`, `skips_other_native_event` (reason names the event), `skips_session_id_in_hook`, `skips_literal_dollar` (`^a$`, reason names the argument).
- `renders_var_form_with_dollar` — `${TMPDIR:-/tmp}/x` with `tmpdir` in `vars` renders.
- `session_start_with_startup_in_match_renders` — `["startup","resume","clear"]` renders.

**Done when:** `airsl test extensions` reports `0 failed` and `cargo make dod` is green. Commit `feat(repo): opencode extension hook rules`.

---

## Risks

- A test file put in the wrong place could go unrun. Each task checks that its new test names
  appear in the `airsl test` output.
- Riskiest step: Task 3's skip-rule order; the tests pin each reason with a hook that could match two
  rules where the spec order decides.

## Proof

- `airsl test extensions` reports `0 failed`; `cargo make dod` green.

## Review findings

Batch review, 2026-11-25 (`claudestacks:reviewer`, 0🔴). `airsl test extensions` → 122 passed, 0 failed (10 files); workspace `cargo make dod` → exit 0.

- fixed 🔵 `extensions/opencode/lib/frontmatter.lua:40` — `ACTIONS[word] or word` passes an unmapped tool word through; unreachable today (the core's tool set is closed and all seven are mapped).
- fixed 🔵 `extensions/opencode/lib/hooks.lua:99,106` — the `TOOLS[word] or { word }` fallback is unreachable (the core validates match values).
- fixed 🔵 `extensions/opencode/lib/hooks.lua:125`, `extensions/opencode/lib/hooks_test.lua:187,206` — comments cite skip-table rule numbers; naming the reason would survive a reorder.
- kept: extra tests `agent_every_tool_word_maps` and `skips_follow_table_order`.

### Fix round, 2026-11-25

All open findings above were fixed on the author's instruction ("fix all of findings"), by four
parallel coders plus inline edits; one `claudestacks:reviewer` pass over the fixes (1🔴 F1: the
first `--tool ../agentstacks/extensions/opencode` workaround is refused, `must stay inside the
project`; fixed to copy the extension in and pass `--tool ext/opencode`, the form
`init --tool ext/opencode --pack packs/check --yes` already proved, `applied: 14 written`). The
reviewer's 🟡/🔵 on the fixes (CLAUDE.md claim and wraps, RUNTIME_LUA native-event wording, Lua
lookup comments, `covers` doc, check.mjs header, per-directory PATH links, explicit mock env, stale
`target/opencode-check/2.4.1/opencode`) were fixed inline. Gate after the round:
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` → exit 0, 25 `test result: ok`, lua 122 + 651 passed
0 failed; `cargo make opencode-check` → `opencode v2.4.1`, `ok S1` … `ok S7`; `cargo make
book-build` → 0 `[ERROR]`; `crates/agentstacks/assets/runtime.lua` unchanged.

## Deviations

- 2026-11-25 — Task 3, rule 3: the Change text reads "(neutral) session.start without `startup` in `match`", but the task's own Proof `session_start_without_match_renders` requires a no-`match` session.start to render. Built: rule 3 skips only a non-empty `match` that lacks `startup`. Tests guard both directions (`session_start_without_match_renders`, `skips_session_start_without_startup`); the reviewer judged it sound.
- 2026-11-25 — Not committed: the execute instruction did not ask for commits.
