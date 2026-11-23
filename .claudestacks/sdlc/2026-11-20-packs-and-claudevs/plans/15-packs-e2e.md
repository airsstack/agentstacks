---
status: approved
created: 2026-11-23
depends-on: [06, 07, 08, 12, 13, 14]
built-on: 0f89fdb3ebb3
---

# Packs end-to-end Implementation Plan

**Goal:** Prove end to end, through the CLI, that the five official packs work together.

**Architecture:**
- A new integration test target, `crates/agentstacks-cli/tests/packs.rs`, built like
  `commands.rs` (spec §1: it copies a fixture and `extensions/claude` into a temp dir, runs the real
  binary, and skips without airsl unless `AGENTSTACKS_REQUIRE_AIRSL` is set;
  `commands.rs:3-6,17-44,68-73`).
- It copies the repo's `packs/` and `extensions/claude` into a temp git project, runs `init` for
  all five packs, and checks the result and one rendered hook run. It holds no rule of its own; the
  rules are proven in the library and the Lua tests.
- The airsl check differs from `commands.rs` in one way: the packs need airsl ≥0.1.5, so the test
  skips (or, with `AGENTSTACKS_REQUIRE_AIRSL`, fails) when `airsl --version` reports an older
  version (spec §11).

**Tech Stack:** Rust integration test (`tempfile`, `std::process`), the `agentstacks` and `airsl`
binaries, `serde_json` for reading `settings.json`.

**Content authority:** spec §1 (skip count, matcher rows, sentinel life), §11 (end-to-end row and
the `packs.rs` assertions).

---

## Files that change

```
crates/agentstacks-cli/tests/packs.rs   — create: the five-pack install and the Read-hook run
```

## Order of work

### Task 1 — Five-pack install test

**Files:**
- Create `crates/agentstacks-cli/tests/packs.rs`.

**Change:**
- Helpers, each private to the file and named for its job:
  - `airsl_at_least_0_1_5()` reads `airsl --version` and compares the version;
  - `skip_without_airsl()` uses it, with the `commands.rs` skip/fail rule;
  - `project()` makes a temp dir, runs `git init -q` in it, and copies
    `../../packs` → `packs/` and `../../extensions/claude` → `ext/claude`;
  - `init_all()` runs `agentstacks -C <dir> init --tool ./ext/claude --pack ./packs/core
    --pack ./packs/sdlc --pack ./packs/journal --pack ./packs/guideline-rust
    --pack ./packs/guideline-python --yes` with stdin null.
- `commands.rs` stays as it is; the two files share no module. The helpers are small, and a shared
  test module would add a `mod` file to both targets for four functions.

**Proof:** (each a `#[test]`, each starting with `skip_without_airsl()`)
- `five_packs_install`:
  - `init_all()` exits 0;
  - `agentstacks plan` lists exactly one skip,
    `core/hook/3: claude shows no hook output to the model at session end`;
  - `.claude/skills/` holds exactly 28 directories: `core-*` ×7, `core-statusline`, `sdlc-*` ×11,
    `journal-*` ×7, `guideline-rust-guidelines` and `guideline-python-guidelines`;
  - `.claude/agents/` holds exactly 10 files.
- `agent_stop_matchers_are_pack_prefixed`: `.claude/settings.json` `hooks.SubagentStop` holds groups
  with the matchers `core-coder|core-explorer|core-reviewer`,
  `sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer` and `journal-curator`.
- `read_hook_points_at_the_rust_guideline`:
  - the project gets a `Cargo.toml`;
  - the test takes the rendered `PreToolUse` command whose matcher is `Read|Edit|Write`, and runs it
    with `sh -c` with:
    - `CLAUDE_PROJECT_DIR` set to the project;
    - `TMPDIR` set to a fresh directory inside the test's temp dir, so no sentinel from an earlier
      run is seen;
    - on stdin,
      `{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"<project>/src/main.rs"},"session_id":"s1","cwd":"<project>"}`;
  - stdout parses as JSON, and its `hookSpecificOutput.additionalContext` contains
    `/guideline-rust-guidelines`.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks-cli --test packs` passes and
`cargo make dod` is green. Commit `test(agentstacks-cli): five official packs install end to end`.

---

## Risks

- The test reads the repo's live `packs/`. A later pack edit that breaks install fails here first,
  which is the point. The counts (28, 10) change only with a spec change.
- The Read-hook run depends on `sh` and the airsl binary on `PATH`. CI has both (plan 09), and the
  skip rule covers a developer machine without airsl 0.1.5.
- A `TMPDIR` shared with an earlier run would hold a sentinel, and the hook would print nothing.
  The per-run directory prevents it (spec §1: a sentinel lives 24 h).

## Proof

- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green locally; CI `dod` green (it sets the variable).
