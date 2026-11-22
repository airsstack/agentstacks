---
status: approved
created: 2026-11-22
depends-on: [04, 07]
built-on: 7b4a3a8f175f
---

# Claude extension adapter Implementation Plan

**Goal:** Ship the claude extension's manifest and adapter, proven by golden tests on the real host.

**Architecture:**
- `extension.toml` declares `reads = [".claude/settings.json"]`.
- `adapter.lua` is the only file that calls `agentstacks.*`. It loads the lib modules through
  `agentstacks.require("lib.<m>")`, answers `values` and `plan`, and proposes writes, copies and
  skips.
- `crates/agentstacks/tests/claude.rs` runs the real `app::plan::plan` with `StdWorkspace` and
  `AirslAdapter` on fixture projects. It compares every proposed file and skip with checked-in
  expected files. It also runs the generated `translate.lua` with the core `runtime.lua` on recorded
  payloads.

**Tech Stack:** Lua under airsl 0.1.4 (api 0), Rust integration tests in `crates/agentstacks`.

**Content authority:** spec D23, D24, D25, D27; §9.1 (manifest), §9.3, §9.5 (steps 1-3 and 8-9, checked example), §9.6,
§9.7, §11 (adapter rows), §12 (the whole-extension and translate rows, config examples).

---

## Files that change

```
extensions/claude/extension.toml                               — create: the §9.1 manifest
extensions/claude/adapter.lua                                  — create: values + plan handlers wiring lib/*
crates/agentstacks/tests/claude.rs                             — create: golden plan tests and translate runs
crates/agentstacks/tests/fixtures/claude/full/                 — create: project with every item kind, expected/ outputs
crates/agentstacks/tests/fixtures/claude/settings/             — create: existing settings with user keys, a user hook, an old marked hook
crates/agentstacks/tests/fixtures/claude/removed/              — create: a removed tool (zero packs) with marked handlers on disk
crates/agentstacks/tests/fixtures/claude/payloads/             — create: recorded payloads and expected outputs for translate runs
crates/agentstacks/tests/fixtures/claude/{nohooks,badjson,nullevent,nolua}/ — create: the Task 2 variants: no hooks, settings `[]`, a `null` in PreToolUse, no lua hook
```

## Order of work

### Task 1 — Manifest and `values`

**Files:**
- Create `extensions/claude/extension.toml` — exactly the spec §9.1 block.
- Create `extensions/claude/adapter.lua`:
  - `agentstacks.on("values", …)` returns `values.result(payload)`, with `values` loaded through
    `agentstacks.require("lib.values")`.
- Create `crates/agentstacks/tests/claude.rs`:
  - a helper copies a fixture project into a tempdir and copies `extensions/claude` (from the repo
    root, `CARGO_MANIFEST_DIR/../../extensions/claude`) into it as a `path` tool source;
  - another helper runs `app::plan::plan` with `StdWorkspace`, `GitCli::from_env`,
    `SystemBinary::from_env` and `AirslAdapter`.

**Change:** The extension loads and answers `values`.

**Proof:**
- `claude_manifest_loads`: `Extension::load(&StdFileTree::new(<repo>/extensions/claude))` succeeds,
  with `reads == [".claude/settings.json"]` (spec §12, S7).

**Done when:** `cargo test -p agentstacks --test claude` passes and `cargo make dod` is green.
Commit `feat(repo): claude extension manifest and values handler`.

### Task 2 — `plan` handler and golden cases

**Files:**
- Modify `extensions/claude/adapter.lua` — `agentstacks.on("plan", …)`:
  1. **Skills.** For each pack in payload order and each skill:
     - `frontmatter.skill`; a reason gives `agentstacks.skip{ item = <skill group id>, reason }`;
     - otherwise `agentstacks.write{ path = <dir>.."/SKILL.md", content, covers = {id} }`;
     - every other skill file is `agentstacks.copy{ path = <dir>.."/"..rel, file = id }`.
  2. **Agents:**
     - `frontmatter.agent`, then `write{ path = ".claude/agents/"..name..".md", content, covers = {id} }`,
       or a skip.
  3. **Native files:** `copy{ path = ".claude/"..rel, file = id }`.
  4. **Hooks.** In payload order (packs in order; within a pack, `hooks` by index, then native hooks
     by index):
     - `hooks.render(hook, vals.vars)`, where `vals` is the `values.result` of this run; a reason
       gives a skip;
     - otherwise the group is collected per Claude event.
  5. **Settings:**
     - `old = agentstacks.read(".claude/settings.json")`;
     - the start text is `"{}\n"` when `old` is nil, otherwise `old`;
     - when `old` is not nil, validate with `agentstacks.json.edit(old, {"hooks","agentstacks-check"}, nil)`,
       uncaught;
     - then `settings.decode(old)` (`{}` for `nil`, spec D23), then `settings.edits(…, hooks.MARKER)`;
     - every hook of a skipped event gets `skip{ item = id, reason = settings.skip_reason(E) }`;
     - apply each edit with `agentstacks.json.edit(text, {"hooks", E}, value)` in the given order;
     - if `settings.should_write`, call `write{ path = ".claude/settings.json", content = text,
       covers = <ids of every hook written> }`.
  6. **Translator:** when at least one lua hook (neutral or native) is written,
     `write{ path = ".claude/agentstacks/translate.lua", content = translate.text() }`.
- Create `crates/agentstacks/tests/fixtures/claude/full/`:
  - a project whose `agentstacks.toml` names the extension and one path pack;
  - the pack has:
    - a skill with an extra file and a binary file;
    - a skill with `invocation = "user"`;
    - an agent with tools, model and effort;
    - a `native/claude/` file;
    - a message hook on `session.start`;
    - a message hook on `session.end`;
    - a lua hook on `tool.before`;
    - a hook using `{{var session_id}}`;
    - a native hook;
    - a skill with a native key containing a space;
    - a message hook on `agent.stop`;
    - a lua hook with an argument `^a$`;
    - a native message hook on `Notification`;
  - `expected/` holds one file per proposed path, plus `skips.txt`.
- Create `crates/agentstacks/tests/fixtures/claude/settings/`:
  - `.claude/settings.json` is the spec §9.5 old text;
  - the expected `settings.json` is the spec §9.5 result text, with that fixture's actual hooks.
- Create `crates/agentstacks/tests/fixtures/claude/removed/`:
  - `agentstacks.toml` lists the tool and zero packs;
  - the on-disk settings hold a user handler and a marked handler;
  - the expected settings keep only the user handler.
- Modify `crates/agentstacks/tests/claude.rs` — the golden tests below.

**Change:** The extension renders packs into `.claude/` and merges its hooks into
`.claude/settings.json`.

**Proof:** the real host, with plan steps compared with `expected/`.
- `full_pack_renders_every_item_kind`:
  - every Create step's content equals its expected file byte for byte;
  - `skips.txt` equals the plan's skip lines: session_id, `session.end` message, native key,
    `agent.stop` message (D25), literal `$` (D27), native `Notification` message (D24);
  - `translate.lua` is proposed.
- `existing_settings_keep_user_keys_and_replace_the_marked_hook`: the proposed settings text equals
  the expected file, with `permissions` kept, the user handler kept with sorted keys, the old marked
  handler gone, and the new groups appended.
- `removed_tool_strips_only_marked_handlers`.
- `hooks_left_empty_stays_an_empty_object`: when only a marked handler existed, the result holds `"hooks": {}`.
- `settings_absent_and_no_hooks_proposes_no_settings_write`: a fixture variant with no hooks.
- `invalid_settings_json_refuses_the_plan`: settings text `[]` gives the refusal
  `claude/adapter.lua:<line>: json.edit: top level must be an object`, matched with the line number
  as a wildcard.
- `unrewritable_event_skips_our_hook`: a `PreToolUse` array holding a `null`. Our `tool.before` hook
  is skipped with the exact settings reason, and that event's text is unchanged.
- `no_lua_hook_writes_no_translate`.

**Done when:** `cargo test -p agentstacks --test claude` passes and `cargo make dod` is green.
Commit `feat(repo): claude extension plan handler`.

### Task 3 — Translator runs on recorded payloads

**Files:**
- Create `crates/agentstacks/tests/fixtures/claude/payloads/`:
  - one JSON payload per spec §9.7 row, anonymised from `probes/claude-results.md`: `SessionStart`,
    `SessionEnd`, `UserPromptSubmit`, `PreToolUse` (Read), `PreToolUse` (Bash), `PostToolUse`,
    `SubagentStop`, and an unmapped `Stop`;
  - small hook modules returning context, block or both;
  - an `echo` hook module that writes the neutral event's fields (`event`, `source`, `prompt`,
    `tool`, `path`, `last_message`, `agent`, `session_id`, `cwd`) to stderr with
    `airsstack.stdio.error`, which exists under `confined` (`docs/hooks.md:121`). Each field is
    written as `key=value`, with `nil` printed as `nil`. The module returns `nil`, so the mapping
    shows for every event, including `SessionEnd` and `SubagentStop`, which print nothing on stdout;
  - extra payloads: `SessionStart` with `source = "fork"`, and `PreToolUse` with `agent_type = ""`;
  - one expected-output file per case.
- Modify `crates/agentstacks/tests/claude.rs`:
  - for each case, write the expected `translate.lua` (taken from the full fixture's `expected/`)
    and the core `RUNTIME_LUA` into a tempdir root;
  - lay the hook modules out under the root and build the argv with `HookCommand`, as the existing
    harness does (`tests/runtime.rs:109-145`), so the argv matches production; run it with the
    payload on stdin;
  - compare stdout byte for byte;
  - skip with a printed reason when the binary is absent, unless `AGENTSTACKS_REQUIRE_AIRSL` is set
    (as `tests/runtime.rs` does).

**Change:** The generated translator is proven on the real runtime.

**Proof:**
- `pre_tool_use_block_prints_deny`.
- `post_tool_use_block_prints_decision_block`.
- `session_start_context_prints_additional_context`.
- `subagent_stop_context_only_prints_nothing` (spec D25).
- `subagent_stop_block_prints_decision_block`.
- `user_prompt_submit_block_prints_decision_block`.
- `session_end_prints_nothing`.
- `bash_tool_is_not_run`: no output.
- `native_hook_on_stop_prints_nothing`.
- `echo_maps_each_event`: for each of the six mapped events, the echo stderr shows the neutral name
  and the extra fields of spec §9.7 (`prompt`, `tool`/`path`, `last_message`, `source`), and stdout
  is empty.
- `fork_source_maps_to_nil`: the echo stderr shows `source=nil`.
- `empty_agent_type_maps_to_nil_agent`: the echo stderr shows `agent=nil`.
- `session_start_with_context_and_block_joins_them`: the output's `additionalContext` is
  `<ctx>\n<blk>`.
- `nil_result_prints_nothing`: a hook returning `nil` prints nothing.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test claude` passes and
`cargo make dod` is green. Commit `test(agentstacks): run the claude translator on recorded payloads`.

---

## Risks

- **Golden files that copy the code's output.** Expected files written from the extension's own
  output would prove nothing. Each expected file is built from the spec rules: the frontmatter
  tables, the §9.4 command forms and the §9.5 result shape. Review them against the spec before
  committing.
- **Riskiest step.** Task 2's settings fixture. Its expected text must follow the proven `json.edit`
  pretty-print behaviour (spec §9.5 example, S4).
- **Fixture placement.** The fixture copies `extensions/claude` into a tempdir, because path sources
  must stay inside the project.

## Proof

- `cargo make dod` green, including `lua`.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test claude` green.
