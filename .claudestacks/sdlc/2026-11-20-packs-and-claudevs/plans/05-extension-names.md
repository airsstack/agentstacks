---
status: approved
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# Extension names Implementation Plan

**Goal:** Make the claude extension name every skill and agent `<pack>-<item>`.

**Architecture:**
- `extensions/claude/lib/values.lua` is the one place that maps `p/s` and `p/a` to Claude names.
  `adapter.lua` reads its result (`run_values`) for agent file paths, frontmatter names and
  agent-stop matchers. The core is unchanged (spec §4: "The core stays tool-agnostic").
- `lib/hooks.lua` stays free of `require` and `agentstacks.*`. It takes the rendered agent names as
  an argument (`docs/extensions.md:78`; `lib/hooks.lua:4`).
- Native files keep their own paths and names (spec §4 "Native files").

**Tech Stack:** Lua under airsl (`airsl test extensions`), Rust integration tests
(`crates/agentstacks/tests/claude.rs`, `crates/agentstacks-cli/tests/commands.rs`), Markdown.

**Content authority:** spec §1 (names rows), §4 (skill names, agent names, frontmatter `name`,
agent-stop matcher, native files, collisions, length, docs), §11 (claude extension row), §12
(prefixed-name docs).

---

## Files that change

```
extensions/claude/lib/values.lua                 — modify: invoke "/<p>-<s>", dir ".claude/skills/<p>-<s>", agent "<p>-<a>"
extensions/claude/lib/values_test.lua            — modify: prefixed skill and agent cases
extensions/claude/lib/hooks.lua                  — modify: render takes the rendered agent names for the matcher
extensions/claude/lib/hooks_test.lua             — modify: matcher cases
extensions/claude/lib/frontmatter_test.lua       — modify: the rendered name is written as given
extensions/claude/adapter.lua                    — modify: rendered names for frontmatter, agent path and hook agents
crates/agentstacks/tests/fixtures/claude/*/expected/**  — modify/rename: expected trees at the prefixed paths and names
crates/agentstacks/tests/claude.rs               — modify: rendered path assertion at :283 (:263 stays: a lock-recorded delete)
crates/agentstacks-cli/tests/commands.rs         — modify: SKILL const (:21)
docs/extensions.md                               — modify: §6 values example (:149-156), payload example dir (:219)
docs/authoring.md                                — modify: worked example (:100-103)
docs/cli.md                                      — modify: plan example paths (:120-122)
docs/plan-and-apply.md                           — modify: example paths (:117-123)
```

## Order of work

### Task 1 — Prefixed names in `values`

**Files:**
- Modify `extensions/claude/lib/values.lua`, `values_test.lua`.

**Change:** `M.result(payload)` returns, for pack `p`:
- `skills["p/s"] = { invoke = "/p-s", dir = ".claude/skills/p-s" }`;
- `agents["p/a"] = "p-a"`.

Vars and `root` are unchanged.

**Proof:**
- `skill_maps_to_slash_name_and_dir` becomes `skill_maps_to_pack_prefixed_name_and_dir`: pack
  `sdlc`, skill `status` gives `{ invoke = "/sdlc-status", dir = ".claude/skills/sdlc-status" }`.
- `agent_maps_to_its_name` becomes `agent_maps_to_pack_prefixed_name`: `core/coder` gives
  `core-coder`.
- New `hyphenated_pack_is_prefixed_whole`: pack `guideline-python`, skill `guidelines` gives
  `/guideline-python-guidelines` (spec §4 "Length": no length rule).

**Done when:** `airsl test extensions` passes. Commit
`feat(repo): claude extension prefixes skill and agent names with the pack`.
(Tasks 1–3 ship together if `cargo make dod` cannot go green between them; then one commit after
Task 3 carries all three subjects in its body.)

### Task 2 — Rendered names in frontmatter, agent paths and matchers

**Files:**
- Modify `extensions/claude/adapter.lua`, `lib/hooks.lua`, `lib/hooks_test.lua`,
  `lib/frontmatter_test.lua`.

**Change:**
- `adapter.lua` `propose_skill` passes the rendered name (the `invoke` of `run_values.skills["p/s"]`
  without its leading `/`) to `frontmatter.skill`, so `SKILL.md` frontmatter `name` is `sdlc-status`
  (spec §4: the frontmatter name sets the command).
- `propose_agent` passes `run_values.agents["p/a"]` to `frontmatter.agent` and writes
  `.claude/agents/<that name>.md`.
- `hooks.render(hook, vars, agents)` gains a third argument: the rendered names of `hook.agents`, in
  the hook's order, or `nil` when the hook has none. `matcher_for` joins that list with `|` for
  `agent.stop`. The adapter builds it from `run_values.agents[pack.name .. "/" .. a]`.
- Native files and native hooks are not renamed (`native/claude/<rel>` → `.claude/<rel>`).
- Two proposals of one path stay refused by the existing rule (spec §4 "Collisions"); no new check.

**Proof:**
- `agent_stop_matcher_joins_agents` passes `{"sdlc-task-briefer", "sdlc-chain-reader",
  "sdlc-artifact-reviewer"}` and asserts the matcher
  `sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer`.
- New `agent_stop_without_agents_has_no_matcher`: `agents = nil` gives no `matcher` key.
- New `rendered_name_is_the_frontmatter_name` (`frontmatter_test.lua`): `frontmatter.skill("sdlc-status", …)`
  and `frontmatter.agent("core-coder", …)` write `name: "sdlc-status"` and `name: "core-coder"`.
- `full_pack_renders_every_item_kind` (`tests/claude.rs:207`) passes against the updated expected
  tree (Task 3).

**Done when:** `airsl test extensions` passes. Commit with Task 3.

### Task 3 — Expected trees and docs follow the prefixed names

**Files:**
- Modify/rename under `crates/agentstacks/tests/fixtures/claude/*/expected/`: every
  `.claude/skills/<s>/` becomes `.claude/skills/<pack>-<s>/`, every `.claude/agents/<a>.md` becomes
  `.claude/agents/<pack>-<a>.md`, each with frontmatter `name` set to the rendered name; every
  `settings.json` agent-stop `matcher` becomes the prefixed list (`"coder"` → `"core-coder"` in
  `full`). Input fixtures (packs, locks) are not changed.
- Modify `crates/agentstacks/tests/claude.rs:283` (the rendered `nohooks` write) and
  `crates/agentstacks-cli/tests/commands.rs:21` to the prefixed paths. `claude.rs:263` stays: it
  asserts the delete of `.claude/skills/gone/SKILL.md`, the path the input lock records (`.claude/skills/demo-greet/SKILL.md` for the CLI demo pack).
- Modify `docs/extensions.md` §6 (`:149-156`: `invoke = "/" .. p.name .. "-" .. s`,
  `dir = ".claude/skills/" .. p.name .. "-" .. s`, `agents[…] = p.name .. "-" .. a`) and the payload
  example at `:219` (`"dir": ".claude/skills/core-status"`); `docs/authoring.md:100-103` (the `mytool` example):
  `invoke = "/" .. p.name .. "-" .. s`, `dir = paths.skill_dir(p.name .. "-" .. s)`,
  `agents[…] = p.name .. "-" .. a`, and the path it asserts at `:156` becomes
  `.mytool/skills/<pack>-greet` with that example's pack name; `docs/cli.md:120-122` and `docs/plan-and-apply.md:117-123`: `.claude/skills/core-status/…`,
  `.claude/agents/core-coder.md`, and `team-lint`'s `.claude/agents/team-lint-old.md`; also
  `docs/cli.md:218` (`changed: .claude/agents/core-coder.md`).

**Change:** Tests and docs state the new names. No code change.

**Proof:**
- `cargo test -p agentstacks --test claude` passes: `full_pack_renders_every_item_kind`,
  `removed_tool_strips_only_marked_handlers` and
  `settings_absent_and_no_hooks_proposes_no_settings_write` at the prefixed paths.
- `cargo test -p agentstacks-cli --test commands` passes.
- `grep -n 'invoke = "/" .. s' docs/*.md` prints nothing.

**Done when:** `cargo make dod` is green. Commit
`feat(repo): claude extension prefixes skill and agent names with the pack`.

---

## Risks

- A lock fixture used as input (`removed/agentstacks.lock`) records old paths. A test that deletes
  a recorded path keeps the recorded path; only rendered output moves. If a test then fails for a
  reason other than the rename, stop and report it (do not edit the lock to make it pass).
- The commit across Tasks 1–3 is large; the expected trees are mechanical renames.

## Proof

- `airsl test extensions` and `cargo make dod` green.
- `grep -rn 'skills/status/\|agents/coder.md' crates/agentstacks/tests/fixtures/claude/*/expected docs`
  prints nothing.
