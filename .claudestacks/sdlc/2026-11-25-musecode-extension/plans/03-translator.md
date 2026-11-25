---
status: done
created: 2026-11-25
built-on: c1dbedb148db
---

# Muse Code Translator Implementation Plan

**Goal:** The musecode extension provides the spec's `translate.lua` text.

**Architecture:** One pure module, `extensions/musecode/lib/translate.lua`, whose `M.text()` returns
the spec §4.7 text verbatim, with a colocated `translate_test.lua` (spec §2). The text itself runs
under the core runtime in hook processes; it is written to `<root>/translate.lua` by the adapter
(plan 04).

**Tech Stack:** Lua under airsl 0.1.5 (`airsl test extensions`, part of `cargo make lua` in `dod`).

**Content authority:** spec §4.7, §6 (rows owned by `translate.lua`), §8.1 (`translate_test`).

---

## Files that change

```
extensions/musecode/lib/translate.lua        — create: the translator source text
extensions/musecode/lib/translate_test.lua   — create: tests for the text
```

## Order of work

### Task 1 — `translate`

**Files:**
- Create `extensions/musecode/lib/translate.lua`, `extensions/musecode/lib/translate_test.lua`.

**Change:** `M.text()` returns the `lua` block of spec §4.7 byte for byte (the header comment
`-- agentstacks translator for musecode, api 0.`, `EVENTS`, `TOOLS` with `read_file`/`edit_file`/
`write_file`, `SOURCES`, `str`, `absolute` with its empty-path and empty-`cwd` guards, `M.event`,
`encode`, `M.result`, `return M`). The module's own header comment says what the text is for, as
`extensions/claude/lib/translate.lua` does.

**Proof:** written first, seen failing, then passing —
- `text_is_the_spec_translator` — `M.text()` equals the spec §4.7 block, held in the test as the
  expected string.
- `text_defines_event_and_result` — the text contains `function M.event(p)` and
  `function M.result(r, p)` and ends with `return M\n`.

**Done when:** `airsl test extensions` reports `0 failed`, `airsl check extensions` compiles every
file, and the Definition of Done is green. Commit `feat(repo): musecode extension translator`.

## Risks

- The text is proven in `probes/translate-results.md` (`T`); a retyped character breaks that proof.
  The coder copies the spec block, never retypes it.
- Its behaviour on real payloads is tested by plan 04 (`tests/musecode.rs`), not here.

## Proof

- `airsl test extensions` → `0 failed`; `cargo make dod` green.

## Review findings

One reviewer pass over the batch of plans 01-03 (verdict: compliant, `blocking: none`, 0🔴); no
finding on `translate.lua` or `translate_test.lua`.

- Verified: `airsl test extensions` → `197 passed, 0 failed (15 files)` (reviewer re-run);
  `cargo make dod` → Lua `651 passed, 0 failed (25 files)`, `Build Done in 90.06 seconds`.
  `text_is_the_spec_translator` and `text_defines_event_and_result` were first seen failing
  (`module translate not found`). The text was copied from spec §4.7 by line range, not retyped.

## Deviations

- 2026-11-25 — No commit made: the execute instruction did not ask for one.
