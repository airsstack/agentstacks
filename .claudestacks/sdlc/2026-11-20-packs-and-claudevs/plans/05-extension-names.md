---
status: done
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

## Review findings

- 🟡 fixed — `docs/authoring.md:120` worked example wrote the agent at `paths.agent_file(a.name)` while its `values` names it `<pack>-<agent>`; now `paths.agent_file(p.name .. "-" .. a.name)`. Re-run of the example (adapter re-extracted from the doc, demo pack, `agentstacks plan`): `create   .mytool/agents/demo-helper.md  demo`.
- 🟡 fixed (follow-up) — `extensions/claude/adapter.lua:63`: reverting the `hook_agents` mapping (passing raw `hook.agents`) passes every test. The only neutral agent.stop hook with `agents` in the Rust fixtures (`full` hook 6) is skipped by the literal-`$` rule; `hooks_test` checks the join, not the prefixing. Needs a fixture with a rendered agent.stop hook, or the lookup moved into `lib/` with a unit test.
- 🔵 fixed (follow-up) — `docs/authoring.md:70` says `airsl test` gave `2 passed, 0 failed`; the example's test file gives `1 passed, 0 failed (1 files)` (line not changed here).
- 🔵 fixed (follow-up) — `extensions/claude/adapter.lua:12` comment says `plan` needs `run_values` for var forms; it now also supplies skill and agent names.
- 🔵 fixed (follow-up) — `extensions/claude/lib/values.lua:21` one 120-column comment line.
- 🔵 fixed (follow-up) — `full/packs/core/native/claude/hooks.toml:5` native `SubagentStop` matcher `"coder"` now matches no installed agent (`core-coder`); correct (native hooks are not renamed), but no doc tells pack authors native matchers must use `<pack>-<agent>`.
- Verification (reviewer re-run): `cargo make dod` exit 0, `airsl test` 67 passed / 0 failed; `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks --test claude` 23 passed; `cargo test -p agentstacks-cli --test commands` 10 passed.

## Deviations

- 2026-11-23 — The task-briefer spawns were skipped: the coder read this plan directly. Plans 05 and 06 ran in one coder, in order (both change `values.lua`, `values_test.lua`, `docs/extensions.md` and `full/expected/.claude/settings.json`).
- 2026-11-23 — Task 3 claim "`settings.json` agent-stop `matcher` becomes the prefixed list (`"coder"` → `"core-coder"` in `full`)": that matcher comes from the native `SubagentStop` hook (`full/packs/core/native/claude/hooks.toml` `match = ["coder"]`), and native hooks are not renamed (Task 2); the neutral agent.stop hook listing `coder` is skipped in `full` (literal `$`). Left `"coder"`.
- 2026-11-23 — `full/expected` rendered bodies and `references/guide.md` also changed (`/core-ask`, `core-coder`, `/core-status`, `.claude/skills/core-status/…`): they hold rendered `{{skill}}`, `{{agent}}`, `{{skill_dir}}` tokens.
- 2026-11-23 — Docs: the `extensions.md`/`authoring.md` examples are multi-line; the `authoring.md` test example uses its pack name (`paths.skill_dir("demo-greet")`, `.mytool/skills/demo-greet`); the `cli.md` plan block is re-aligned.
- 2026-11-23 — Test-first partial in Task 3: the Lua unit tests were seen red; `full_pack_renders_every_item_kind` was first run after both the Lua change and the expected tree, so not seen red. `agent_stop_without_agents_has_no_matcher` and `rendered_name_is_the_frontmatter_name` were green on first run (pin existing behaviour).
- 2026-11-23 — Not committed (execute does not commit unless told).

## Follow-up fixes

- 2026-11-23 — Every open finding above was fixed in one follow-up pass (three parallel coders, disjoint files). Workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0: 23 `test result: ok` blocks, 2474 Rust tests passed, lua `71 passed, 0 failed (5 files)`, `Build Done in 55.65 seconds`.
- `adapter.lua:63`: the lookup moved to `values.hook_agents(result, pack_name, hook)` with 4 unit tests; `full/packs/core/pack.toml` gains hook 7 (neutral agent.stop, `agents = ["coder"]`), expected `settings.json` gains a `SubagentStop` matcher `"core-coder"`. Revert proofs: raw `hook.agents` in the adapter → `full_pack_renders_every_item_kind … FAILED`; raw names in `hook_agents` → 2 Lua FAILs.
- Native matcher: `full/packs/core/native/claude/hooks.toml` `match = ["core-coder"]`; `docs/extensions.md` says native matchers use the installed name `<pack>-<agent>`.
- `docs/authoring.md:70` now quotes `1 passed, 0 failed (1 files)` (`airsl test mytool`, airsl 0.1.4).
