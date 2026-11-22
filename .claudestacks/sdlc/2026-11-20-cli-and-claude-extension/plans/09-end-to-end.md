---
status: approved
created: 2026-11-22
depends-on: [03, 06, 08]
built-on: cefc42fbbc4c
---

# End to end Implementation Plan

**Goal:** Prove the `agentstacks` binary end to end on a temp project with the real claude extension.

**Architecture:**
- `crates/agentstacks-cli/tests/commands.rs` runs the built binary
  (`env!("CARGO_BIN_EXE_agentstacks")`) on a tempdir project that holds a path-source pack and a
  copy of `extensions/claude`.
- No logic is tested only here (spec §12). This plan proves that the wiring holds.
- The `CLAUDE.md` status line closes the chain.

**Tech Stack:** Rust integration tests, tempfile, the airsl binary.

**Content authority:** spec §12 (CLI end-to-end row), §13 (docs row: `CLAUDE.md`), D15 (labels seen
through the binary).

---

## Files that change

```
crates/agentstacks-cli/tests/commands.rs                — create: end-to-end runs of the binary
crates/agentstacks-cli/tests/fixtures/project/          — create: a path-source pack with a skill, an agent and a lua hook
CLAUDE.md                                               — modify: Status section
```

## Order of work

### Task 1 — End-to-end command runs

**Files:**
- Create `crates/agentstacks-cli/tests/fixtures/project/packs/demo/`:
  - a `pack.toml`;
  - one skill;
  - one agent;
  - one `tool.before` lua hook;
  - one message hook on `session.start`.
- Create `crates/agentstacks-cli/tests/commands.rs`:
  - a helper copies the fixture into a tempdir, copies `<repo>/extensions/claude` to `ext/claude`
    inside it, and runs the binary with `-C <tempdir>`, stdin null;
  - tests skip with a printed reason when the airsl binary is absent, unless
    `AGENTSTACKS_REQUIRE_AIRSL` is set.

**Change:** None in `src`. The tests prove the command sequence.

**Proof:**
- `init_with_yes_writes_toml_and_applies`:
  - `init --tool ./ext/claude --pack ./packs/demo --yes` exits 0;
  - `agentstacks.toml` holds `path = "./ext/claude"` and `path = "./packs/demo"`;
  - `.claude/skills/<s>/SKILL.md`, `.claude/settings.json` and `agentstacks.lock` exist;
  - the last stdout line starts with `applied: `.
- `init_without_terminal_keeps_the_toml_and_is_refused`: plain `init` in a fresh tempdir with stdin null exits 1. Its stderr is `stdin is not a terminal; pass --yes to apply` then `agentstacks.toml written; nothing applied (run apply)`, and `agentstacks.toml` exists.
- `init_refuses_an_existing_toml`: a second `init` exits 1, with stderr
  `agentstacks.toml: already exists (edit it and run apply)`.
- `plan_after_apply_reports_nothing_to_write`: `plan` exits 0, and its summary starts `0 to write`.
- `apply_without_terminal_or_yes_refuses`:
  - after the written skill file is deleted, which makes a `Restore` step (`src/types/plan/action.rs:29-30`), `apply` with stdin null exits 1;
  - stderr is `stdin is not a terminal; pass --yes to apply`;
  - nothing is written.
- `apply_yes_with_nothing_pending_prints_nothing_to_change`: exits 0.
- `doctor_is_healthy_then_reports_a_change`:
  - `doctor` exits 0 and prints `healthy`;
  - after editing the written skill, it exits 1 and prints `changed: .claude/skills/<s>/SKILL.md`.
- `init_with_unknown_name_is_refused`: `--pack nope` exits 1, with the exact unknown-name message,
  and no toml is written.
- `usage_error_exits_two`: `init --tool claude` exits 2.
- `path_pack_refusal_names_the_path_as_written`:
  - a broken `packs/bad/pack.toml` is added to the toml by hand;
  - `plan` exits 1, and stderr starts with `packs/bad/pack.toml:`, with no tempdir path.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test -p agentstacks-cli --test commands` passes and
`cargo make dod` is green. Commit `test(agentstacks-cli): end-to-end command runs on the claude extension`.

### Task 2 — Status line

**Files:**
- Modify `CLAUDE.md`:
  - the Status heading and its first paragraph say the CLI commands and the claude extension are
    built;
  - the layout block's `agentstacks-cli` line lists the five commands;
  - the paragraph naming what has not landed drops `extensions/` and keeps `packs/`, `book/` and the
    claudevs move.

**Change:** The status text matches the built state. The org-level status (spec §14) belongs to
`release-and-migration`.

**Proof:** None; this is a docs-only task.

**Done when:** `cargo make dod` is green and `cargo deny check` passes. Commit
`docs(repo): CLI commands and claude extension built`.

---

## Risks

- **No git on the test path.** End-to-end runs use path sources only, because the official
  repository is private and has no `packs-v*` tag (spec §4.3, core §17). The official-name path is
  proven through fakes in plan 02.
- **Interactive `y` is not run here.** The binary is not driven through a real terminal. The prompt
  is proven by unit tests (plan 05 Task 4, plan 06 Task 2).

## Proof

- `cargo make dod` green.
- `cargo deny check` passes.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo test --workspace` green.
