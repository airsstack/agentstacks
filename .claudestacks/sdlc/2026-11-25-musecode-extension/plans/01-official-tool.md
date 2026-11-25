---
status: done
created: 2026-11-25
built-on: c1dbedb148db
---

# Official musecode Tool Implementation Plan

**Goal:** `init --tool musecode` resolves to the official repository's `extensions/musecode`.

**Architecture:** One constant in `crates/agentstacks/src/types/shorthand.rs`, the one place the
official source is named (spec §2, §3.1). No new type, module or dispatch; the core stays
tool-agnostic.

**Tech Stack:** Rust (workspace crate `agentstacks`), `cargo make dod`.

**Content authority:** spec §3.1, §8.2 (last bullet).

---

## Files that change

```
crates/agentstacks/src/types/shorthand.rs   — modify: OFFICIAL_TOOLS gains "musecode"; colocated test
```

## Order of work

### Task 1 — `musecode` joins the official tools

**Files:**
- Modify `crates/agentstacks/src/types/shorthand.rs` — the `OFFICIAL_TOOLS` constant and its
  `#[cfg(test)] mod tests`.

**Change:** `OFFICIAL_TOOLS` becomes a three-element array `["claude", "musecode", "opencode"]`
(name order, spec §3.1). Its doc comment stays "The tool names the official repository holds under
`extensions/`." with no count (spec §2, `doc-comment-discipline.md`). Nothing else in the file
changes.

**Proof:** written first, seen failing, then passing —
- `official_tool_musecode_expands_to_extensions_subdir` — `accepted(Flag::Tool, "musecode")` is
  `Argument::Official` with `url` equal to `OFFICIAL_REPOSITORY` and `subdir` `extensions/musecode`,
  the same shape as `official_tool_opencode_expands_to_extensions_subdir`. Before the change it fails
  with the refusal `--tool "musecode": not an official tool; pass a path, or add a git source to
  agentstacks.toml` (spec §3.1, `probes/agentstacks-results.md` A1).

**Done when:** `cargo test -p agentstacks official_tool_musecode_expands_to_extensions_subdir`
passes and the Definition of Done is green (`cargo make dod`: fmt-check, clippy `-D warnings`,
rustdoc `-D warnings`, tests, doctests, lua). Commit `feat(agentstacks): musecode is an official tool`.

## Risks

- Any other test that asserts the array length or its exact contents fails; the coder updates it in
  the same task (a grep for `OFFICIAL_TOOLS` finds the users).
- The CLI help or docs that list official tools are plan 06's.

## Proof

- `cargo make dod` green.

## Review findings

One reviewer pass over the batch of plans 01-03 (verdict: compliant, `blocking: none`, 0🔴).

- 🔵 open — `extensions/musecode/` has no `extension.toml`/`adapter.lua` yet, so `--tool musecode`
  resolves to a directory that cannot load until plan 04 lands; do not merge this plan alone.
- Verified: `cargo test -p agentstacks official_tool` → `3 passed` (incl.
  `official_tool_musecode_expands_to_extensions_subdir`, first seen failing with
  `--tool "musecode": not an official tool; pass a path, or add a git source to agentstacks.toml`);
  `cargo make dod` → Lua `651 passed, 0 failed (25 files)`, fingerprint and text gates passed,
  `Build Done in 90.06 seconds`.

## Deviations

- 2026-11-25 — No commit made: the execute instruction did not ask for one (the plan's
  `feat(agentstacks): musecode is an official tool` is left to the author).
- 2026-11-25 — `grep -rn OFFICIAL_TOOLS crates` found no other test asserting the array; nothing else changed.
