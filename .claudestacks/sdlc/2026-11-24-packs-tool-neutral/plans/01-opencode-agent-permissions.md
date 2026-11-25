---
status: approved
created: 2026-11-25
built-on: 63f62977694c
---

# OpenCode Agent Permissions Implementation Plan

**Goal:** Every OpenCode pack agent with `tools` may read and write outside the project.

**Architecture:** One rule is added to `permissions(tools)` in the opencode extension's
`lib/frontmatter.lua`, right after the deny-all rule. The Lua unit test, the `opencode.rs`
expected fixtures, a both-tools install test in `packs.rs` and one book sentence follow it. No
Rust `src/` file changes; no hook, shim or runtime file changes, so `runtime.lua`'s hash stays.

**Tech Stack:** Lua under airsl 0.1.5, Rust integration tests, mdBook.

**Content authority:** spec §6a, §10 (rows `five_packs_install_on_both_tools`,
`opencode_agents_reach_outside_the_project`, the `opencode` expected fixtures), E14, E15, F1, F21,
F22.

---

## Files that change

```
extensions/opencode/lib/frontmatter.lua            — modify: permissions() adds the external_directory allow
extensions/opencode/lib/frontmatter_test.lua       — modify: the rule lists of three tests
crates/agentstacks/tests/fixtures/opencode/demo/expected/.opencode/agents/demo-helper.md — modify: permissions line
crates/agentstacks/tests/fixtures/opencode/full/expected/.opencode/agents/full-coder.md  — modify: permissions line
crates/agentstacks-cli/tests/packs.rs              — modify: project copies the opencode extension; two tests
book/src/ch06-03-opencode.md                       — modify: line 33, the agent row
```

`full-plain.md` (an agent without `tools`, `permissions: []`) does not change.

## Order of work

### Task 1 — Allow paths outside the project for agents with tools

**Files:**
- Modify `extensions/opencode/lib/frontmatter.lua` — `permissions(tools)`.
- Test `extensions/opencode/lib/frontmatter_test.lua` — `agent_with_empty_tools_denies_all`
  (line 46), `agent_tools_map_to_permissions` (line 51), `agent_every_tool_word_maps` (line 59,
  its rule list at line 65).
- Modify `crates/agentstacks/tests/fixtures/opencode/demo/expected/.opencode/agents/demo-helper.md`
  and `crates/agentstacks/tests/fixtures/opencode/full/expected/.opencode/agents/full-coder.md` —
  line 4, `permissions:`.

**Change:** When `tools` is present (empty or not), `permissions(tools)` returns the deny-all rule
`{ action = "*", effect = "deny", resource = "*" }`, then
`{ action = "external_directory", effect = "allow", resource = "*" }`, then one allow rule per
mapped tool in today's order. An absent `tools` still returns `[]`. The doc comment above
`permissions` (`frontmatter.lua:32-33`, "the deny-all rule, then one allow rule per mapped action")
names the external allow between the two. In each of the two expected
agent files, insert `{"action":"external_directory","effect":"allow","resource":"*"},` right after
the deny-all object on the `permissions:` line; nothing else in those files changes. For
`tools: [read, write, edit, shell, glob, grep]` the rendered line is exactly the spec §6a line.

**Proof:** written first, seen failing, then passing —
- `agent_with_empty_tools_denies_all` — `tools: []` gives exactly `[deny-all, external_directory
  allow]` (E15).
- `agent_tools_map_to_permissions` — the external allow is the second rule, before the first tool
  allow.
- `agent_every_tool_word_maps` — its expected list carries the external allow second.
- `agent_without_tools_has_empty_permissions` — unchanged, still `[]`.
- The `opencode.rs` fixture tests (`full_pack_writes_skills_commands_agents_and_native_files` and
  `demo_pack_plan_matches`, `crates/agentstacks/tests/opencode.rs:213`) — the rendered agents
  equal the updated expected files.
- `cargo make opencode-check` S1-S7 still pass: S3 asserts the check pack's `helper` agent is
  offered exactly `grep,read,skill` (`check.mjs:372-375`), and that list comes from the
  `permissions` line this task changes.

**Done when:** `cargo make lua`, `cargo test -p agentstacks --test opencode` and
`cargo make opencode-check` pass and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`fix(repo): let opencode agents reach paths outside the project`.

### Task 2 — Install the five packs on both tools in the pack tests

**Files:**
- Modify/Test `crates/agentstacks-cli/tests/packs.rs` — `project()` (line 114), the module doc
  (lines 1-8), a new both-tools init helper, two new tests.

**Change:** `project()` also copies `extensions/opencode` to `ext/opencode` (beside today's
`ext/claude`), and its doc comment and the module doc say both extensions are copied. A new helper
`init_both(dir)` runs `init --tool ./ext/claude --tool ./ext/opencode` with the five packs and
`--yes`, and asserts exit 0, as `init_all` does for one tool. `init_all` and every existing test
are unchanged. Every new test starts with `skip_without_airsl()`. Doc comments name behaviour only.

**Proof:** written first, seen failing, then passing —
- `five_packs_install_on_both_tools` — after `init_both`, `plan` exits 0 and its `skip ` lines are
  exactly these 7, in any order:
  `core/hook/3: claude shows no hook output to the model at session end`;
  `core/hook/1: opencode runs session.start hooks only for a new session (source "startup"), which this hook does not match`;
  `core/hook/3: opencode has no hook when a session ends`;
  `core/hook/8: opencode has no hook when a subagent stops`;
  `core/hook/9: opencode has no hook when a subagent stops`;
  `journal/hook/1: opencode has no hook when a subagent stops`;
  `sdlc/hook/1: opencode has no hook when a subagent stops` (F1). It fails first because
  `ext/opencode` is not copied.
- `opencode_agents_reach_outside_the_project` — after `init_both`, every `.opencode/agents/*.md`
  has a `permissions:` line whose first rule is `{"action":"*","effect":"deny","resource":"*"}`
  and whose second is `{"action":"external_directory","effect":"allow","resource":"*"}` (parsed
  with `serde_json`). The directory holds at least one agent.

**Done when:** `cargo test -p agentstacks-cli --test packs` passes and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`test(agentstacks-cli): install the five packs on both tools`.

### Task 3 — Document the agent permissions

**Files:** Modify `book/src/ch06-03-opencode.md` — line 33, the `agent` table row.

**Change:** The row's second cell, which ends "with frontmatter and the text" with no period, gains
a period and then the sentence:
``Its `tools` become `permissions`: deny everything, allow paths outside the project, then allow
each listed tool.`` The row stays one table line.

**Proof:** `grep -n 'allow paths outside the project' book/src/ch06-03-opencode.md` prints line 33
(before the edit it exits 1); `cargo make book-build` prints no `[ERROR]` line.

**Done when:** `cargo make book-build` passes with no `[ERROR]` and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`docs(repo): describe opencode agent permissions`.

## Risks

- The extra rule is applied to every agent with `tools`; an agent author who wanted a project-only
  agent now has none. The spec accepts this (D3): OpenCode's own default allows it.
- Task 2 makes every new `packs.rs` test copy two extensions; the existing claude-only tests copy
  one more directory and are otherwise unchanged.
- Riskiest step: Task 1 changes three Lua tests and two fixtures in one commit; a missed fixture
  shows as an `opencode.rs` failure, which the done-check runs.

## Proof

- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green.
- `cargo make book-build` with no `[ERROR]`.
