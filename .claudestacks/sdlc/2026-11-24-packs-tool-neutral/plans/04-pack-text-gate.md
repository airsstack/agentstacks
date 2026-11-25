---
status: approved
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
