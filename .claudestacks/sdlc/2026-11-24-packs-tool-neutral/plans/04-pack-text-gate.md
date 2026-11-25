---
status: done
created: 2026-11-25
depends-on: [01, 02, 03]
built-on: 63f62977694c
---

# Pack Text Gate 4 Implementation Plan

**Goal:** `cargo make lua` refuses one tool's tool names, events or store in neutral pack text.

**Architecture:** Gate 4 is a shell block in `[tasks.lua]` of `Makefile.toml`, after gate 3, in
the same shape as gate 3 (read grep's status, then filter). `packs.rs` asserts the gate is in the
lua task, and a both-tools install test checks the rendered OpenCode instructions against the same
names. Depends on plans 02 and 03 (the text the gate passes) and plan 01 (`init_both`).

**Tech Stack:** cargo-make, POSIX `sh` and `grep -E` (BSD and GNU), Rust integration tests.

**Content authority:** spec §7, §10 (rows gate 4, gates 1-3, `the_lua_gate_tests_the_packs_and_is_part_of_dod`,
`opencode_instructions_name_no_claude_tools`), F7, §6.

---

## Files that change

```
Makefile.toml                           — modify: gate 4 block after gate 3; line 201 count
crates/agentstacks-cli/tests/packs.rs   — modify: loop 1..=4; opencode_instructions_name_no_claude_tools
```

## Order of work

### Task 1 — Add pack text gate 4 to the lua task

**Files:**
- Modify `Makefile.toml` — `[tasks.lua]`: insert the gate 4 block between the end of gate 3
  (line 200, its closing `fi`) and line 201; line 201 becomes `echo "pack text gates: 4 passed"`.
- Test `crates/agentstacks-cli/tests/packs.rs` — `the_lua_gate_tests_the_packs_and_is_part_of_dod`
  (line 733), its loop at line 751.

**Change:** The inserted block is byte for byte
`.claudestacks/sdlc/2026-11-24-packs-tool-neutral/probes/final/gate4.sh` (copied, not retyped; it
holds no `'''`, so it fits the TOML literal string). The test's loop becomes `for gate in 1..=4`.

**Proof:** written first, seen failing, then passing —
`the_lua_gate_tests_the_packs_and_is_part_of_dod` — with the loop at `1..=4` it fails on
"[tasks.lua] has no `pack text gate 4`" until the block is in; then it passes. Also, after the
block is in, `cargo make lua` ends with `pack text gates: 4 passed`; on a scratch copy of the tree
with one neutral line put back (e.g. `` `Agent` `` in a core agent), `cargo make lua` prints that
line and `pack text gate 4: tool-specific names in neutral pack text` and fails.

**Done when:** `cargo make lua` prints `pack text gates: 4 passed`,
`cargo test -p agentstacks-cli --test packs` passes and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod`
is green. Commit `build(repo): add pack text gate 4 for tool-neutral text`.

### Task 2 — Check the rendered OpenCode instructions

**Files:** Modify/Test `crates/agentstacks-cli/tests/packs.rs` — one new test and, if needed, one
helper listing the gate 4 names.

**Change:** A new test `opencode_instructions_name_no_claude_tools` (skip without airsl first) runs
plan 01's `init_both`, walks every file under `.opencode/skills/`, `.opencode/agents/` and
`.opencode/commands/` (the existing `files_under` helper), and fails naming each file and line that
contains one of gate 4's names. Every alternative of gate 4's pattern is a literal string once the
regex escape is dropped, so the test holds them as a list of plain strings and uses
`str::contains` (the crate has no regex dependency: `crates/agentstacks-cli/Cargo.toml:22-31`
lists clap, similar and agentstacks, and the dev-dependencies agentstacks, serde_json and
tempfile): `subagent_type`, `TodoWrite`, `Task / Agent`,
`` `Agent` ``, `` `Task` ``, `` `Bash` ``, `` `Read` ``, `` `Write` ``, `` `Edit` ``,
`` `MultiEdit` ``, `` `Grep` ``, `` `Glob` ``, `` `Skill` ``, `` `WebFetch` ``, `` `WebSearch` ``,
`` `AskUserQuestion` ``, `` `NotebookEdit` ``, `CLAUDE_CONFIG_DIR`, `CLAUDE_SESSION_ID`, `.claude/`,
`transcript JSONL`, `SessionStart`, `PreToolUse`, `PostToolUse`, `SubagentStop`,
`UserPromptSubmit`. Files named `doc-comment-discipline.md` or `docstring-discipline.md` are
skipped, as gate 4 skips them. The test's doc comment names the behaviour only.

**Proof:** written first, seen failing, then passing — `opencode_instructions_name_no_claude_tools`
— no rendered OpenCode skill, agent or command file holds a gate 4 name. It fails if run on a tree
before plans 02 and 03 (for example on `.opencode/agents/core-reviewer.md`); with them in, it
passes. Seen failing: run it once with one name put back in a scratch copy, or before plans 02-03
land, and record the output.

**Done when:** `cargo test -p agentstacks-cli --test packs` passes and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`test(agentstacks-cli): rendered opencode instructions name no claude tools`.

## Risks

- GNU grep on CI may treat the pattern differently from BSD grep on macOS; CI runs gate 4 on every
  push (spec K4).
- Text spec §6 keeps (statusline scripts, `transcript-path.lua`, the two discipline references,
  tests, READMEs, the changelog) is excluded by name; a new file of that kind elsewhere would need
  its own exclusion, which this plan does not add.
- Riskiest step: Task 1's block in the TOML literal string; a stray `'''` would end the string. The
  block holds none.

## Proof

- `cargo make lua` ends with `pack text gates: 4 passed`.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green.

## Review findings

One reviewer pass over the diff of plans 04-06. The reviewer re-ran the DoD: exit 0, `pack text gates: 4 passed`. It named nothing as blocking. All four findings were fixed on the author's word.

After the fixes:

- `cargo make lua` → `pack text gates: 4 passed`
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` → exit 0, `packs` tests 19 passed

- 🔵 fixed — The gate 4 comment did not describe every excluded file. It now names each kind (READMEs and the changelog, Lua tests, the statusline and transcript-path scripts, `doc-comment-discipline.md`), in both `Makefile.toml` and `probes/final/gate4.sh`. The two stay byte-identical (`cmp` equal).
- 🔵 fixed — Gate 4 excluded `docstring-discipline.md`, which holds no gate 4 name. The exclusion is removed from both files, and gate 4 still passes.
- 🔵 fixed — `CLAUDE_ONLY_NAMES` was a hand-kept copy of the gate's names. The test now reads them from gate 4's pattern in `Makefile.toml`:
  - `gate4_names` extracts the pattern, and `literal_alternatives` expands it. `expands_the_gate_pattern_into_literal_names` unit-tests the expansion.
  - The test asserts that `subagent_type`, `` `Agent` `` and `.claude/` are among the names read.
  - Seen following the gate: with `|with the skill tool` added to the pattern, the test failed on `.opencode/commands/core-discuss.md:4: with the skill tool`. `Makefile.toml` was then restored (`cmp` equal).
- 🔵 fixed — The skip for `docstring-discipline.md` did nothing. It is removed, and the doc names only `doc-comment-discipline.md`.

## Deviations

- 2026-11-25 — No task-briefer ledger or briefs were used. The plan is small (two tasks), so the orchestrator read it and built both tasks inline instead of through coders.
- 2026-11-25 — No context-handoff session was started. Each of the run's two subagents (plan 06's coder, then one reviewer) wrote to a literal temp path.
- 2026-11-25 — Plans 04, 05 and 06 ran as one batch with one review over their joint diff. Per-task commits were not made.
- 2026-11-25 — The plan's "seen failing" runs on a scratch copy of the tree were done in place instead. `` Spawn helpers with the `Agent` tool. `` was appended to `packs/core/agents/reviewer.md`, and the file was restored from a backup afterwards (`cmp` equal). Outputs:
  - `` [tasks.lua] has no `pack text gate 4` ``
  - `` .opencode/agents/core-reviewer.md:106: `Agent` ``
  - `` packs/core/agents/reviewer.md:114: Spawn helpers with the `Agent` tool. `` then `pack text gate 4: tool-specific names in neutral pack text` (exit 1)
- 2026-11-25 — The new test's doc first said "rendered for OpenCode". Clippy's `doc_markdown` lint refused `OpenCode` under `-D warnings`, so the doc now says "rendered under `.opencode/`".
- 2026-11-25 — The review fixes changed `probes/final/gate4.sh`, the file the plan copies the block from: a new comment, and `--exclude=docstring-discipline.md` dropped. The spec was then amended to match (spec §13, A2; proof `probes/raw/a11-gate4-amended.txt`).
