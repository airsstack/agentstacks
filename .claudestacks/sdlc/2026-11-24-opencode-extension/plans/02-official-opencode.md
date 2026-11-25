---
status: done
created: 2026-11-25
depends-on: [01]
built-on: d54f8422f1ff
---

# Official OpenCode Tool Implementation Plan

**Goal:** `agentstacks init --tool opencode` resolves to the official repository's `extensions/opencode`.

**Architecture:** One constant in `types/shorthand.rs`, the one place the official source is named;
the CLI already expands official names generically. Depends on plan 01 only because both edit
`crates/agentstacks/CHANGELOG.md`.

**Tech Stack:** Rust (`agentstacks`), mdBook.

**Content authority:** spec §3.1, §8.2 (shorthand test), §8.5 (CLI docs, changelogs).

---

## Files that change

```
crates/agentstacks/src/types/shorthand.rs     — modify: OFFICIAL_TOOLS = ["claude", "opencode"], new test
crates/agentstacks/CHANGELOG.md               — modify: Unreleased Added line
crates/agentstacks-cli/CHANGELOG.md           — modify: Unreleased Added line
book/src/ch04-00-cli.md                       — modify: official tools list
book/src/ch01-02-first-project.md             — modify: mention --tool opencode beside --tool claude
```

## Order of work

### Task 1 — Accept `opencode` as an official tool

**Files:**
- Modify `crates/agentstacks/src/types/shorthand.rs` — `OFFICIAL_TOOLS: [&str; 2] = ["claude", "opencode"]`; the doc comment keeps "The tool names the official repository holds under `extensions/`" with no count; a colocated test.

**Change:** `--tool opencode` expands to the official repository with subdir `extensions/opencode`,
as `claude` does. Unknown tool names are still refused with the existing wording.

**Proof:**
- `official_tool_opencode_expands_to_extensions_subdir` — `accepted(Flag::Tool, "opencode")` gives `Argument::Official` with subdir `extensions/opencode`.
- `unknown_tool_is_refused_with_the_tool_wording` (existing) still passes.

**Done when:** `cargo test -p agentstacks shorthand` passes and `cargo make dod` is green. Commit `feat(agentstacks): make opencode an official tool`.

### Task 2 — Document `--tool opencode`

**Files:**
- Modify `book/src/ch04-00-cli.md` — the official tools line reads `claude`, `opencode`.
- Modify `book/src/ch01-02-first-project.md` — where `--tool claude` is shown, one sentence that `--tool opencode` sets up OpenCode the same way, and both flags may be given.
- Modify `crates/agentstacks/CHANGELOG.md` — `## [Unreleased]` `### Added`: `opencode` is an official tool name.
- Modify `crates/agentstacks-cli/CHANGELOG.md` — `## [Unreleased]` `### Added`: `init --tool opencode`.

**Change:** The CLI chapters and changelogs name the new official tool.

**Proof:**
- `cargo make book-build` builds with no `[ERROR]` line.

**Done when:** `cargo make book-build` passes and `cargo make dod` is green. Commit `docs(agentstacks-cli): document --tool opencode`.

---

## Risks

- `init --tool opencode` against the official repository reaches users only with the next
  `packs-v*` tag, which holds `extensions/` and `packs/` together (spec §3.1,
  `src/types/shorthand.rs:35`). The release itself is outside this chain (`RELEASING.md`).

## Proof

- `cargo make dod` green.

## Review findings

Batch review, 2026-11-25 (code and docs reviewers, 0🔴 for this plan). `cargo test -p agentstacks shorthand` → 13 passed; workspace `cargo make dod` → exit 0.

- fixed 🟡 `book/src/ch01-02-first-project.md:15`, `book/src/ch04-00-cli.md:59` — the book names `--tool opencode` before a `packs-v*` tag holds `extensions/opencode` (already this plan's Risk; the release is outside the chain).

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

- 2026-11-25 — Task 1 ran in parallel with plan 01 Task 1 (disjoint files); Task 2 ran with plan 01 Task 2 in one coder (shared `crates/agentstacks/CHANGELOG.md`).
- 2026-11-25 — Not committed: the execute instruction did not ask for commits.
