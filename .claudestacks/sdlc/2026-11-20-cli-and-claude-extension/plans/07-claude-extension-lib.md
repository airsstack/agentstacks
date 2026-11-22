---
status: approved
created: 2026-11-22
built-on: 7b4a3a8f175f
---

# Claude extension library Implementation Plan

**Goal:** Build the pure Lua modules of the claude extension, each tested with `airsl test`.

**Architecture:**
- `extensions/claude/lib/` holds five modules: `values`, `frontmatter`, `hooks`, `settings` and
  `translate`.
- They call no `agentstacks.*` and no `require`. Only `airsstack.json` is used. Inside the host,
  `require` is absent and modules load through `agentstacks.require` (`docs/extensions.md:78`,
  `:381-396`), so every dependency is passed in by `adapter.lua` (plan 08).
- Each module has a `<name>_test.lua` beside it. The test requires `"<name>"` and returns a table of
  named test functions (`probes/airsl-test-results.md`).
- `cargo make dod` gains a `lua` task.

**Tech Stack:** Lua under airsl 0.1.4, `airsstack.json` (`decode`, `encode`),
cargo-make.

**Content authority:** spec D23, D24, D25, D27; §2 (extension layout), §9.1 (values), §9.2, §9.4,
§9.5 (steps 4-9 and edge rules), §9.6, §9.7, §11 (Lua rows), §12 (Lua row, `lua` task).

---

## Files that change

```
extensions/claude/lib/values.lua           — create: the values result for a values payload
extensions/claude/lib/values_test.lua      — create
extensions/claude/lib/frontmatter.lua      — create: meta + native → YAML frontmatter text, or a skip reason
extensions/claude/lib/frontmatter_test.lua — create
extensions/claude/lib/hooks.lua            — create: a payload hook → {event, group} or a skip reason
extensions/claude/lib/hooks_test.lua       — create
extensions/claude/lib/settings.lua         — create: decoded old settings + our groups → per-event edits and skipped events
extensions/claude/lib/settings_test.lua    — create
extensions/claude/lib/translate.lua        — create: returns the text of <root>/translate.lua
extensions/claude/lib/translate_test.lua   — create
Makefile.toml                              — modify: `lua` task (airsl test extensions/claude) joins dod
```

## Order of work

### Task 1 — `values` and the `lua` gate task

**Files:**
- Create `extensions/claude/lib/values.lua`, `values_test.lua`.
- Modify `Makefile.toml`:
  - add `[tasks.lua]`, category Gate, `command = "airsl"`, `args = ["test", "extensions/claude"]`;
  - add `lua` to `tasks.dod.dependencies`;
  - update the header comment, which says the gate is five steps, and the `[tasks.deny]` comment,
    which says adding a sixth step would disagree with the guideline. Both now name `lua` as the
    project's extra step.

**Change:** `M.result(payload)` returns:
- `root = ".claude/agentstacks"`;
- `skills["<p>/<s>"] = { invoke = "/<s>", dir = ".claude/skills/<s>" }`;
- `agents["<p>/<a>"] = "<a>"`;
- `vars`, holding only the names listed in `payload.vars`:

  | Var | Value |
  |---|---|
  | `session_id` | `${CLAUDE_SESSION_ID}` |
  | `arguments` | `$ARGUMENTS` |
  | `project_dir` | `${CLAUDE_PROJECT_DIR}` |
  | `tmpdir` | `$TMPDIR` |
  | `home` | `$HOME` |
  | `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` |

**Proof:**
- `root_is_dot_claude_agentstacks`.
- `skill_maps_to_slash_name_and_dir`.
- `agent_maps_to_its_name`.
- `only_requested_vars_are_returned`: `vars = {"tmpdir"}` gives exactly `{ tmpdir = "$TMPDIR" }`.
- `every_var_has_its_value`: all six values, checked one by one.

**Done when:** `airsl test extensions/claude` passes and `cargo make dod` is green, now including
`lua`. Commit `feat(repo): claude extension values module and lua gate task`.

### Task 2 — `frontmatter`

**Files:**
- Create `extensions/claude/lib/frontmatter.lua`, `frontmatter_test.lua`.

**Change:**
- `M.skill(name, meta, text)` and `M.agent(name, meta, text)` return `content`, or `nil, reason`.
- The shape is `---\n`, then one `key: <airsstack.json.encode(value)>\n` per key, then `---\n`, then
  `text`.
- **Skill keys:**
  - `name`;
  - `description`;
  - `disable-model-invocation: true` when `invocation = "user"`; `any` writes nothing;
  - `argument-hint` from `argument_hint`.
- **Agent keys:**
  - `name`;
  - `description`;
  - `tools`, mapped read→Read, write→Write, edit→Edit, grep→Grep, glob→Glob, shell→Bash,
    skill→Skill, in the written order; absent writes nothing;
  - `model`, mapped fast→`haiku`, balanced→`sonnet`, deep→`opus`; absent writes nothing;
  - `effort`, the same word.
- **Native keys:**
  - `meta.native` keys replace a generated key of the same name;
  - native keys that were not generated follow the generated ones, sorted by name;
  - a native key that does not match `^[A-Za-z0-9_-]+$` returns
    `nil, 'native.claude key "<key>" cannot be written as frontmatter'`;
  - decoded `{}` and `[]` values encode back as `{}` and `[]` (spec §9.2, S6).

**Proof:**
- `skill_frontmatter_has_name_and_description_in_order`.
- `user_invocation_writes_disable_model_invocation`.
- `any_invocation_writes_nothing`.
- `argument_hint_is_renamed`.
- `agent_tools_map_in_written_order`: `{shell, read}` gives `tools: ["Bash","Read"]`.
- `every_model_word_maps`.
- `effort_is_kept`.
- `absent_tools_and_model_write_nothing`.
- `native_key_overrides_a_generated_key`.
- `extra_native_keys_follow_sorted`.
- `native_key_with_a_space_is_a_skip_reason`: the exact text.
- `native_empty_object_and_list_encode_back`: native values `{}` and `[]` (decoded) are written as `{}` and `[]`.
- `string_with_quote_is_json_escaped`: `a "q"` encodes as `"a \"q\""`.
- `body_text_follows_the_closing_fence_unchanged`.

**Done when:** `airsl test extensions/claude` passes and `cargo make dod` is green. Commit
`feat(repo): claude extension frontmatter module`.

### Task 3 — `hooks`

**Files:**
- Create `extensions/claude/lib/hooks.lua`, `hooks_test.lua`.

**Change:** `M.render(hook, vars) -> { event = <Claude event>, group = {…} }` or `nil, reason`.
`vars` is the `values.vars` map from `values` (the rendered var forms).
- **Events and matchers:**

  | Neutral event | Claude event | Matcher |
  |---|---|---|
  | `session.start` | `SessionStart` | `match` joined with `\|`; absent → none |
  | `session.end` | `SessionEnd` | none |
  | `prompt.submit` | `UserPromptSubmit` | none |
  | `tool.before` | `PreToolUse` | read/edit/write → Read/Edit/Write joined with `\|`; absent → `Read\|Edit\|Write` |
  | `tool.after` | `PostToolUse` | same as `tool.before` |
  | `agent.stop` | `SubagentStop` | `agents` joined with `\|`; absent → none |
  | native hook | its `event` as written | its `match` list joined with `\|`; absent → none |

- **Group:** `{ matcher = <m> or absent, hooks = { { type = "command", command = <c> } } }`.
- **Message hook:**
  - the command is `printf '%s\n' '<json>' # agentstacks`;
  - `<json>` is `airsstack.json.encode({ hookSpecificOutput = { hookEventName = E,
    additionalContext = text } })`, with each `'` replaced by `'"'"'`.
- **Lua hook:**
  - the command is `cd "$CLAUDE_PROJECT_DIR" && `, then each argv element in double quotes joined by
    spaces, then ` # agentstacks`;
  - inside the quotes, `\`, `"` and `` ` `` are escaped with `\`; `$` is left alone.
- **Skips**, checked in this order:
  - text or argv containing `${CLAUDE_SESSION_ID}`:
    `claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}`;
  - a message hook on `session.end`: `claude shows no hook output to the model at session end`;
  - a message hook on `agent.stop`, or a native message hook on `Stop` or `SubagentStop` (spec D25):
    `claude treats hook output at agent stop as feedback that keeps the agent running`;
  - a native message hook on an event outside `SessionStart`, `SubagentStart`, `UserPromptSubmit`,
    `UserPromptExpansion`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch` and
    `PostModelSwitch` (spec D24): `claude passes no hook message to the model on <Event>`, with the
    event as written;
  - a lua hook with a literal `$` (spec D27). For each argv element, remove every value of `vars`,
    longest first. If a `$` remains, the reason is
    `claude runs hook commands through sh, which would expand the literal $ in argument "<arg>"`,
    where `<arg>` is the element as rendered.
- Also `M.MARKER = " # agentstacks"`, which `settings` uses.

**Proof:**
- `each_neutral_event_maps_to_its_claude_event` (six cases).
- `tool_match_maps_and_joins`.
- `tool_match_absent_defaults_to_read_edit_write`.
- `agent_stop_matcher_joins_agents`.
- `prompt_submit_has_no_matcher`.
- `native_hook_keeps_event_and_match`.
- `message_command_is_printf_of_additional_context_json`.
- `single_quote_in_message_is_shell_escaped`.
- `lua_command_matches_the_spec_example`: the spec §9.4 argv gives exactly the spec's example
  command.
- `dollar_is_left_unescaped`.
- `session_id_var_in_message_is_skipped` and `session_id_var_in_argv_is_skipped`: exact reasons.
- `message_on_session_end_is_skipped`: exact reason.
- `message_on_agent_stop_is_skipped` and `native_message_on_stop_and_subagent_stop_is_skipped`: the
  exact D25 reason.
- `native_message_on_notification_is_skipped`: exactly
  `claude passes no hook message to the model on Notification`.
- `native_message_on_post_tool_use_failure_renders`.
- `literal_dollar_in_argv_is_skipped`: `^a$` gives exactly
  `claude runs hook commands through sh, which would expand the literal $ in argument "^a$"`.
- `var_forms_in_argv_render`: `$TMPDIR/x` and `${AIRSSTACK_HOME:-$HOME/.airsstack}/y` are rendered,
  with `vars` holding both forms.
- `every_command_ends_with_the_marker`.

**Done when:** `airsl test extensions/claude` passes and `cargo make dod` is green. Commit
`feat(repo): claude extension hook rendering`.

### Task 4 — `settings`

**Files:**
- Create `extensions/claude/lib/settings.lua`, `settings_test.lua`.

**Change:**
- `M.decode(old)`:
  - returns `{}` when `old` is `nil` (spec D23), empty or whitespace-only, without calling decode;
  - otherwise returns `airsstack.json.decode(old)`.
- `M.edits(decoded, groups_by_event, marker)` returns `{ edits = { {event, value_or_nil}, … },
  skipped = { [event] = true } }`.
  - **Touched events:** events whose old array holds a handler whose `command` ends with `marker`,
    plus events in `groups_by_event`.
  - **Rewritable** means all of these hold:
    - the event's array is a list of tables;
    - each group's `hooks` is a list of tables;
    - each `command` is a string;
    - no `null` userdata and no empty list appears anywhere inside it.
    A list is a table with keys exactly `1..n`, or an empty table carrying decode's array metatable.
  - **A rewritable touched event:**
    - marked handlers are removed;
    - groups left with no handlers are dropped;
    - our groups are appended in the given order;
    - an empty result is `nil`.
  - **A touched event that is not rewritable** goes to `skipped`, and no edit is made for it.
  - Edits are sorted by event name.
- `M.should_write(old, edits)` is true when `old ~= nil` or `#edits > 0`.
- `M.skip_reason(event)` returns `.claude/settings.json: hooks.<E> holds entries the claude
  extension cannot rewrite (a null, an empty list, or a value that is not a list of hook groups)`.

**Proof:**
- `nil_empty_and_whitespace_text_decode_to_an_empty_table`.
- `marked_handler_is_replaced_and_user_handler_kept`: the spec §9.5 example input gives a
  `SessionEnd` edit holding the user group, then the new group.
- `new_event_is_added`: `PreToolUse` edit present.
- `group_with_only_marked_handlers_is_dropped`.
- `event_left_empty_is_removed`.
- `null_inside_an_event_makes_it_unrewritable`.
- `empty_list_inside_an_event_makes_it_unrewritable`.
- `non_string_command_makes_it_unrewritable`.
- `unrewritable_event_with_a_marked_handler_gets_no_edit`.
- `edits_are_sorted_by_event_name`.
- `removed_tool_only_removes_marked_handlers`.
- `should_write_false_only_without_old_and_edits`.
- `user_handler_ending_with_marker_is_treated_as_ours`.

**Done when:** `airsl test extensions/claude` passes and `cargo make dod` is green. Commit
`feat(repo): claude extension settings merge rules`.

### Task 5 — `translate`

**Files:**
- Create `extensions/claude/lib/translate.lua`, `translate_test.lua`.

**Change:** `M.text()` returns the source text of `<root>/translate.lua` as one Lua string. That
generated module defines `M.event(p)` and `M.result(r, p)` exactly as spec §9.7.
- **`event`:**

  | `p.hook_event_name` | Neutral event | Extra fields |
  |---|---|---|
  | `SessionStart` | `session.start` | `source` when it is startup, resume, clear or compact; otherwise `nil` |
  | `SessionEnd` | `session.end` | — |
  | `UserPromptSubmit` | `prompt.submit` | `prompt` |
  | `PreToolUse` / `PostToolUse` | `tool.before` / `tool.after` | `tool` for Read/Edit/Write, and `nil` for any other tool; `path = tool_input.file_path` |
  | `SubagentStop` | `agent.stop` | `last_message = last_assistant_message` |
  | any other | `nil` | — |

  Every mapped event also carries `session_id`, `cwd`, and `agent = agent_type` when it is a
  non-empty string.
- **`result`:**

  | Event | Output |
  |---|---|
  | `PreToolUse` | `permissionDecision = "deny"` + `permissionDecisionReason`, and `additionalContext` |
  | `UserPromptSubmit`, `PostToolUse` | `decision = "block"` + `reason`, and `hookSpecificOutput.additionalContext` |
  | `SubagentStop` | `decision = "block"` + `reason`; `ctx` dropped (spec D25), so context-only gives `nil` |
  | `SessionStart` | `additionalContext` = ctx and blk joined by `\n` |
  | `SessionEnd` and any other | `nil` |

  The output is `airsstack.json.encode(out) .. "\n"`. When both ctx and blk are absent, the result
  is `nil`.

**Proof:** `translate_test.lua` checks only the text's structure. Spec §12 tests the translator's
behaviour with `airsl run` on recorded payloads, which is plan 08 Task 3, where every row above has
a case.
- `text_is_a_non_empty_string`.
- `text_defines_event_and_result`: it contains `function M.event(p)` and `function M.result(r, p)`.
- `text_ends_by_returning_the_module`: the last non-blank line is `return M`.

**Done when:** `airsl test extensions/claude` passes and `cargo make dod` is green. Commit
`feat(repo): claude extension translator text`.

---

## Risks

- **The translator is checked only by structure here.** A behaviour slip in the generated text is
  caught only by plan 08 Task 3, which runs it on the real runtime.
- **The `lua` task needs the binary.** It makes `cargo make dod` need the airsl binary locally.
  Without it, the task fails with the shell's command-not-found error (spec §12), and CI already
  installs it (`.github/workflows/ci.yml:59-72`).

## Proof

- `airsl test extensions/claude` reports 0 failed.
- `cargo make dod` green, with `lua` in its step list.
